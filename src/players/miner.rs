use crate::simulation::simulation_history::UpdateReason;
use crate::players::{Player,TraderT};
use crate::order::order::{Order, TradeType, OrderType};
use crate::blockchain::mem_pool::MemPool;
use crate::blockchain::mempool_processor::MemPoolProcessor;
use crate::order::order_book::Book;
use crate::exchange::MarketType;
use crate::exchange::exchange_logic::{Auction, TradeResults};
use crate::utility::{gen_order_id,get_time};

use std::any::Any;
use std::sync::{Mutex, Arc};
use rand::{thread_rng};
use rand::seq::SliceRandom;

/// A struct for the Miner player. 
pub struct Miner {
	pub trader_id: String,
	pub orders: Mutex<Vec<Order>>,
	pub frame: Vec<Order>,
	pub balance: f64,
	pub inventory: f64,
	pub player_type: TraderT,
	pub sent_orders: Mutex<Vec<(u64, OrderType)>>,
}

impl Miner {
	pub fn new(trader_id: String) -> Miner {
		Miner {
			// trader_id: gen_trader_id(TraderT::Miner),
			trader_id: trader_id,
			orders: Mutex::new(Vec::<Order>::new()),
			frame: Vec::<Order>::new(),
			balance: 0.0,
			inventory: 0.0,
			player_type: TraderT::Miner,
			sent_orders: Mutex::new(Vec::<(u64, OrderType)>::new()),
		}
	}

	/// Miner grabs ≤ block_size orders from the MemPool to construct frame for next block
	/// sorted by gas price
	pub fn make_frame(&mut self, pool: Arc<MemPool>, block_size: usize) {
		let size = pool.length();
		if size == 0 {
			println!("No orders to grab from MemPool!");
			return
		}
		// Sort orders in the MemPool in decreasing order by gas price
		pool.sort_by_gas();

		if size <= block_size {
			self.frame = pool.pop_all();
		} 
		else {
			self.frame = pool.pop_n(block_size);
		}
	}

	pub fn publish_frame(&mut self, bids: Arc<Book>, asks: Arc<Book>, m_t: MarketType) -> Option<Vec<TradeResults>> {
		println!("Publishing Frame: {:?}", self.frame);
		// The results from processing the orders in sequential order
		// For CDA: Cancels, Transactions
		// For FBA & KLF: Cancels,
		let process_results: Option<Vec<TradeResults>> = MemPoolProcessor::seq_process_orders(&mut self.frame, 
											Arc::clone(&bids), 
											Arc::clone(&asks), 
											m_t.clone());

		// Don't run end-of-batch auction

		if m_t == MarketType::CDA {
			return process_results;
		}
		if let Some(auction_result) = Auction::run_auction(bids, asks, m_t) {
			// Received some results from FBA or KLF auction, merge with the process_results
			// Option<TradeResults>
			if let Some(mut unwrapped_process_results) = process_results {
				unwrapped_process_results.push(auction_result);
				Some(unwrapped_process_results)
			} else {
				// There were no process results so convert to proper output
				let mut v = Vec::<TradeResults>::new();
				v.push(auction_result);
				return Some(v);
			}
			
		} else {
			return process_results;
		}
	}

	// Selects a random order from the frame and appends an identical order with higher block priority
	pub fn random_front_run(&mut self) -> Result<Order, &'static str> {
		let mut rng = thread_rng();
		if let Some(rand_order) = self.frame.choose(&mut rng) {
			// Copy and update order 
			let mut copied = rand_order.clone();
			copied.trader_id = self.trader_id.clone();
			copied.gas = 0.0;	// No gas needed since this is miner
			copied.order_id = gen_order_id();

			// Add order to highest priority spot in frame
			self.frame.insert(0, copied.clone());
			Ok(copied)
		} else {
			Err("No orders in the frame to front-run")
		}

	}

	// Selects the best priced bid or ask in the book and checks against best bid or ask in order book
	pub fn strategic_front_run(&mut self, best_bid_price: f64, best_ask_price: f64) -> Result<Order, &'static str> {
		if self.frame.len() == 0 {
			return Err("No orders in the frame to front-run");
		}

		let mut orders = self.frame.clone();
		// Sort frame in descending order by price
		orders.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
		// look for highest priced bid and lowest priced ask
		let mut best_bid: Option<Order> = None;
		let mut best_ask: Option<Order> = None;

		for o in orders.iter() {
			match o.trade_type {
				TradeType::Bid => {
					// The best bid will be the first bid order in descending price order
					if best_bid.is_none() {
						best_bid = Some(o.clone());
					}
				},
				TradeType::Ask => {
					// The best ask in frame will be the last ask order in descending price order
					best_ask = Some(o.clone());
				},
			}  
		}
		// println!("\norders in frame: {:?} \n selecting {:?}, {:?}", orders, best_bid, best_ask);


		let mut front_run_order;
		if best_bid.is_none() && best_ask.is_none() {
			return Err("No orders in the frame to front-run");
		} 
		else if best_bid.is_some() && best_ask.is_none() {
			front_run_order = best_bid.expect("frontrun");
		} 
		else if best_bid.is_none() && best_ask.is_some() {
			front_run_order = best_ask.expect("frontrun");
		} 
		else {
			// found both a best bid and best ask, pick the better one relative to current best book prices
			let best_bid = best_bid.expect("frontrun");
			let best_ask = best_ask.expect("frontrun");
			let bid_profit = best_ask_price - best_bid.price;
			let ask_profit = best_ask.price - best_bid_price;
			// println!("\nbid_profit: {}, ask prof: {}\n", bid_profit, ask_profit, );
			if bid_profit < 0.0 && ask_profit < 0.0 {
				// Both orders are worse than best prices in order book, don't front-run
				return Err("No orders in the frame good enough to front-run");
			}
			else if bid_profit >= 0.0 && ask_profit < 0.0 {
				front_run_order = best_bid;
			} 
			else if bid_profit < 0.0 && ask_profit >= 0.0 {
				front_run_order = best_ask;
			} 
			else {
				// Both bid and ask orders are better than best prices in order book, pick order with smallest delta
				if bid_profit >= ask_profit {
					front_run_order = best_ask;
				} else {
					front_run_order = best_bid;
				}
			}
		}

		// println!("\nbest bid: {}, best ask: {}, Chose frontrun order: {:?}\n", best_bid_price, best_ask_price, front_run_order);

		// Copy and update order 
		front_run_order.trader_id = self.trader_id.clone();
		front_run_order.gas = 0.0;	// No gas needed since this is miner
		front_run_order.order_id = gen_order_id();

		// Add order to highest priority spot in frame
		self.frame.insert(0, front_run_order.clone());
		return Ok(front_run_order);
	}

	// Iterate through each order in frame and make a vec to update the
	// players balances in the clearing house. Each update is in the form
	// (trader_id, gas_update_amount)
	// total_gas is the amount to update the miner with
	pub fn collect_gas(&mut self) -> (Vec<(String, f64)>, f64) {
		let mut to_update = Vec::<(String, f64)>::new();
		let mut total_gas = 0.0;
		for order in self.frame.iter() {
			let gas = order.gas;
			total_gas += gas;
			to_update.push((order.trader_id.clone(), gas));
		}
		// Add the miners gas update amount
		to_update.push((self.trader_id.clone(), -total_gas));

		(to_update, total_gas)
	}
}



impl Player for Miner {
	fn as_any(&self) -> &dyn Any {
		self
	}

	fn get_id(&self) -> String {
		self.trader_id.clone()
	}

	fn get_bal(&self) -> f64 {
		self.balance
	}

	fn get_inv(&self) -> f64 {
		self.inventory
	}

	fn get_player_type(&self) -> TraderT {
		self.player_type
	}

	fn update_bal(&mut self, to_add: f64) {
		self.balance += to_add;
	}

	fn update_inv(&mut self, to_add: f64) {
		self.inventory += to_add;
	}

	fn add_order(&mut self,	 order: Order) {
		let mut orders = self.orders.lock().expect("Couldn't lock orders");
		// Add the order info to the sent_orders to track orders to mempool
		self.sent_orders.lock().expect("miner add_order").push((order.order_id, order.order_type.clone()));
		orders.push(order);
	} 

	// Checks if a cancel order has already been sent to the mempool
	fn check_double_cancel(&self, o_id: u64) -> bool {
		let sent = self.sent_orders.lock().unwrap();
		for order in sent.iter() {
			if order.0 == o_id && order.1 == OrderType::Cancel {
				return true;
			}
		}
		false
	}

	fn add_to_sent(&self, o_id: u64, order_type: OrderType) {
		let mut sent = self.sent_orders.lock().expect("add_to_sent");
		sent.push((o_id, order_type));
	}

	fn num_orders(&self) -> usize {
		self.orders.lock().unwrap().len()
	}

	fn get_enter_order_ids(&self) -> Vec<u64> {
		let orders = self.orders.lock().expect("get_enter_order_ids");
		let mut ids = Vec::new();
		for o in orders.iter() {
			if o.order_type == OrderType::Enter {
				ids.push(o.order_id);
			}
		}
		ids
	}

	// Creates a cancel order for the specified order id
	fn gen_cancel_order(&mut self, o_id: u64) -> Result<Order, &'static str> {
		// Get the lock on the player's orders
		let orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
			let order = orders.get(i).expect("investor cancel_order");
			let mut copied = order.clone();
			copied.order_type = OrderType::Cancel;
			return Ok(copied.clone());
        } else {
        	return Err("ERROR: order not found to cancel");
        }
	}


	// Removes the cancel order from the player's active orders
	fn cancel_order(&mut self, o_id: u64) -> Result<(), &'static str> {
		// Get the lock on the player's orders
		let mut orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
			orders.remove(i);
			return Ok(());
        } else {
        	return Err("ERROR: order not found to cancel");
        }
	}


	// Updates the order's volume and removes it if the vol <= 0
	fn update_order_vol(&mut self, o_id: u64, vol_to_add: f64) -> Result<(), &'static str> {
		// Get the lock on the player's orders
		let mut orders = self.orders.lock().expect("couldn't acquire lock on orders");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
        	orders[i].quantity += vol_to_add;
        	// println!("new quantity: {}", orders[i].quantity);
        	if orders[i].quantity <= 0.0 {
        		orders.remove(i);
        	}
        	return Ok(());
        } else {
        	return Err("ERROR: order not found to cancel");
        }
	}

	fn copy_orders(&self) -> Vec<Order> {
		let orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		let mut copied = Vec::<Order>::new();
		for o in orders.iter() {
			copied.push(o.clone());
		}
		copied
	}

	fn log_to_csv(&self, reason: UpdateReason) -> String {
		format!("{:?},{:?},{},{:?},{},{},", 
				get_time(), 
				reason,
				self.trader_id.clone(),
				self.player_type.clone(),
				self.balance,
				self.inventory)
	}
}







