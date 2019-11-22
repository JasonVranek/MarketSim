use crate::exchange::exchange_logic::TradeResults;
use crate::exchange::MarketType;
use crate::order::order::{Order, TradeType};
use crate::utility::get_time;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;


// Tracks the essential information from an order in the order book
pub struct Entry {
	pub order_id: u64,
	pub quantity: f64,	// Only thing that changes with order
	pub timestamp: Duration,
}

impl Entry {
	pub fn new(order_id: u64, quantity: f64) -> Self {
		Entry {
			order_id: order_id,
			quantity: quantity,
			timestamp: get_time(),
		}
	}
}

// Shallow copy of each order
pub struct ShallowBook { 
	pub orders: Vec<Entry>,
	pub block_num: u64,
	pub avg_bids_price: Option<f64>,
	pub avg_asks_price: Option<f64>,
	pub book_type: TradeType,
}

impl ShallowBook {
	pub fn new(bid_or_ask: TradeType, num: u64, abp: Option<f64>, aap: Option<f64>) -> Self {
		ShallowBook {
			orders: Vec::new(),
			block_num: num,
			avg_bids_price: abp,
			avg_asks_price: aap,
			book_type: bid_or_ask,
		}
	}

	pub fn new_entry(&mut self, e: Entry) {
		self.orders.push(e);
	}
}

/// A struct to track the state of the simulation for logging and player strategies. 
/// mempool_data: a hashmap containing every order sent to the mempool, indexed by order id
/// order_books: a vector of shallowbooks which contain the minimum information to recreate state.
/// 			 Each index in the vector will correspond to mutation of state
/// clearings: A vector of TradeResults 
pub struct History {
	pub mempool_data: Mutex<HashMap<u64, (Order, Duration)>>,
	pub order_books: Mutex<Vec<ShallowBook>>,
	pub clearings: Mutex<Vec<(TradeResults, Duration)>>,
	pub market_type: MarketType,
}


impl History {
	pub fn new(m: MarketType) -> History {
		History {
			mempool_data: Mutex::new(HashMap::new()),
			order_books: Mutex::new(Vec::new()),
			clearings: Mutex::new(Vec::new()),
			market_type: m,
		}
	}

	// Adds an order indexed by its order id to a history of all orders to mempool 
	pub fn mempool_order(&self, order: Order) {
		let mut pool = self.mempool_data.lock().expect("History mempool lock");
		pool.insert(order.order_id, (order, get_time()));
	}

	// Parses through the orders and creates a shallow clone of the book
	pub fn clone_book_state(&self, new_book: Vec<Order>, book_type: TradeType, block_num: u64) {
		// Calculate average bid/ask prices from this book
		let (avg_bids, avg_asks) = History::average_order_prices(&new_book, self.market_type);

		// Parse the orders into a ShallowBook 
		let mut new_book_state = ShallowBook::new(book_type, block_num, avg_bids, avg_asks);
		for order in new_book.iter() {
			new_book_state.new_entry(Entry::new(order.order_id, order.quantity));
		}

		let mut prev_histories = self.order_books.lock().expect("History mempool lock");
		prev_histories.push(new_book_state);
	}

	pub fn save_results(&self, results: TradeResults) {
		let mut clearings = self.clearings.lock().expect("save_results");
		clearings.push((results, get_time()));
	}

	// Searches the hashmap of mempool orders
	// Returns a copy of the order and the timestamp it was sent
	pub fn find_orig_order(&self, order_id: u64) -> Option<(Order, Duration)> {
		let mempool_data = self.mempool_data.lock().expect("find_orig_order");
		match mempool_data.get(&order_id) {
			Some((order, time)) => {
				Some((order.clone(), time.clone()))
			}
			None => None,
		}
	}

	pub fn average_order_prices(orders: &Vec<Order>, market_type: MarketType) -> (Option<f64>, Option<f64>) {
		let (mut asks_sum, mut bids_sum) = (0.0, 0.0);
		let (mut num_asks, mut num_bids) = (0.0, 0.0);
		match market_type {
			MarketType::CDA|MarketType::FBA => {
				// For each order in the mempool sum 
				for order in orders {
					match order.trade_type {
						TradeType::Bid => {
							num_bids += 1.0;
							bids_sum += order.price;
						},
						TradeType::Ask => {
							num_asks += 1.0;
							asks_sum += order.price;
						}
					}
				}
			},
			MarketType::KLF => {
				for order in orders {
					match order.trade_type {
						TradeType::Bid => {
							num_bids += 1.0;
							bids_sum += order.p_high;
						},
						TradeType::Ask => {
							num_asks += 1.0;
							asks_sum += order.p_low;
						}
					}
				}
				
			}
		}

		let (mut bids_avg, mut asks_avg) = (None, None); 
		if num_asks > 0.0 {
			asks_avg = Some(asks_sum / num_asks);
		} 
		if num_bids > 0.0 {
			bids_avg = Some(bids_sum / num_bids);
		} 

		(bids_avg, asks_avg)
	}

	// Iterates over all submitted orders to average the bid and ask price.
	// Returns tuple (avg_bids_price, avg_asks_price)
	pub fn average_seen_prices(&self, _weight: f64) -> (Option<f64>, Option<f64>) {
		let (mut asks_sum, mut bids_sum) = (0.0, 0.0);
		let (mut num_asks, mut num_bids) = (0.0, 0.0);
		let all_orders = self.mempool_data.lock().expect("average_prices");
		match self.market_type {
			MarketType::CDA|MarketType::FBA => {
				// For each order in the mempool sum 
				for (_key, (order, _timestamp)) in all_orders.iter() {
					match order.trade_type {
						TradeType::Bid => {
							num_bids += 1.0;
							bids_sum += order.price;
						},
						TradeType::Ask => {
							num_asks += 1.0;
							asks_sum += order.price;
						}
					}
				}
			},
			MarketType::KLF => {
				for (_key, (order, _timestamp))in all_orders.iter() {
					match order.trade_type {
						TradeType::Bid => {
							num_bids += 1.0;
							bids_sum += order.p_high;
						},
						TradeType::Ask => {
							num_asks += 1.0;
							asks_sum += order.p_low;
						}
					}
				}
				
			}
		}

		let (mut bids_avg, mut asks_avg) = (None, None); 
		if num_asks > 0.0 {
			asks_avg = Some(asks_sum / num_asks);
		} 
		if num_bids > 0.0 {
			bids_avg = Some(bids_sum / num_bids);
		} 

		(bids_avg, asks_avg)
	}

	// Looks at current MemPool orders and current orders in OrderBook
	pub fn average_current_prices(&self, pool: Vec<Order>) {
		// Get average from current mempool
		let (avg_bids, avg_asks) = History::average_order_prices(&pool, self.market_type);
		let mut book_bprice: Option<f64> = None;
		let mut book_aprice: Option<f64> = None;
		
		// Calculate the average prices from the orders in order book
		if let Some(last_seen_book) = self.order_books.lock().expect("average_current_prices").last() {
			book_bprice = last_seen_book.avg_bids_price;
			book_bprice = last_seen_book.avg_asks_price;
		}

		// Do something with these prices...
	}


}











