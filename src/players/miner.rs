use crate::exchange::clearing_house::ClearingHouse;
use crate::players::{Player,TraderT};
use crate::order::order::Order;
use crate::blockchain::mem_pool::MemPool;
use crate::blockchain::mempool_processor::MemPoolProcessor;
use crate::order::order_book::Book;
use crate::exchange::MarketType;
use crate::exchange::exchange_logic::{Auction, TradeResults};
use crate::utility::{gen_order_id};


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

	/// 'Publishes' the Miner's frame by sequentially executing the orders in the frame
	pub fn publish_frame(&mut self, bids: Arc<Book>, asks: Arc<Book>, m_t: MarketType) -> Option<Vec<TradeResults>> {
		println!("Publishing Frame: {:?}", self.frame);
		if let Some(results) = MemPoolProcessor::seq_process_orders(&mut self.frame, 
											Arc::clone(&bids), 
											Arc::clone(&asks), 
											m_t.clone()) {
			// TradeResults were received from processing orders, implying results from CDA market
			return Some(results);
		}
		// Run auction after book has been updated (CDA is prcessed in seq_process_orders)
		if let Some(auction_result) = Auction::run_auction(bids, asks, m_t) {
			// Received some results from FBA or KLF auction, convert to same output format as CDA results
			let mut v = Vec::<TradeResults>::new();
			v.push(auction_result);
			return Some(v);
		} 
		None
	}

	// Selects a random order from the frame and appends an identical order with higher block priority
	pub fn front_run(&mut self) -> Result<Order, &'static str> {
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
}



impl Player for Miner {
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
		orders.push(order);
	} 

	fn num_orders(&self) -> usize {
		self.orders.lock().unwrap().len()
	}

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

	fn update_order_vol(&mut self, o_id: u64, vol_to_add: f64) -> Result<(), &'static str> {
		// Get the lock on the player's orders
		let mut orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
        	orders[i].quantity += vol_to_add;
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
}







