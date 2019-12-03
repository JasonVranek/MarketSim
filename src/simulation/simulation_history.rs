use crate::exchange::exchange_logic::TradeResults;
use crate::exchange::MarketType;
use crate::order::order::{Order, TradeType};
use crate::utility::get_time;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;


// Tracks the essential information from an order in the order book
#[derive(Clone)]
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

// Shallow copy of an order book
pub struct ShallowBook { 
	pub orders: Vec<Entry>,
	pub block_num: u64,
	pub avg_bids_price: Option<f64>,
	pub avg_asks_price: Option<f64>,
	pub current_wtd_price: Option<f64>,
	pub num_bids: usize,
	pub num_asks: usize,
	pub best_order: Option<Order>,
	pub book_type: TradeType,
}

impl ShallowBook {
	pub fn new(bid_or_ask: TradeType, num: u64, abp: Option<f64>, 
		aap: Option<f64>, cwp: Option<f64>, order: Option<Order>, nb: usize, na: usize) -> Self {
		ShallowBook {
			orders: Vec::new(),
			block_num: num,
			avg_bids_price: abp,
			avg_asks_price: aap,
			current_wtd_price: cwp,
			num_bids: nb,
			num_asks: na,
			best_order: order,
			book_type: bid_or_ask,
		}
	}

	pub fn new_entry(&mut self, e: Entry) {
		self.orders.push(e);
	}
}

// Likelihood
// A struct to hold statistical data from the history. Used to infer a true value for a price
#[derive(Debug)]
pub struct LikelihoodStats {
	// pub med_pool: Option<f64>,		// Median price of all bids+asks to mempool
	// pub wtd_pool: Option<f64>, 		// Mean price of all bids+asks to mempool, weighted by number of orders (bids vs asks)
	// pub wtd_bids_pool: Option<f64>, // Mean price of all bids to mempool, weighted by recency
	// pub wtd_asks_pool: Option<f64>, // Mean price of all asks to mempool, weighted by recency
	// pub wtd_cp: Option<f64>,		// Mean price of all published clearing prices, weighted by recency

	// pub med_book: Option<f64>,		// Median price of all bids+asks to make it to order book
	// pub wtd_book: Option<f64>, 		// Mean price of all bids+asks to order book, weighted by number of orders
	// pub wtd_bids_book: Option<f64>, // Mean price of all bids to order book, weighted by recency
	// pub wtd_asks_book: Option<f64>, // Mean price of all asks to order book, weighted by recency
	pub mean_bids: Option<f64>,
	pub mean_asks: Option<f64>,
	pub num_bids: u64,
	pub num_asks: u64,
	pub weighted_price: Option<f64>,
}

// Prior
// A struct to hold the current data. 
// Used to measure how close the current price is from the inferred true value.
#[derive(Debug)]
pub struct PriorData {
	pub clearing_price: Option<f64>,
	pub best_bid: Option<Order>,
	pub best_ask: Option<Order>,
	pub current_bids: Vec<Order>,
	pub current_asks: Vec<Order>,
	pub current_wtd_price : Option<f64>,
	pub mean_pool_gas: f64,
	pub asks_volume: f64,
	pub bids_volume: f64,
	pub current_pool: Vec<Order>,
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
		let (avg_bids, avg_asks, num_bids, num_asks, wtd_avg_price) = History::average_order_prices(&new_book, self.market_type);

		let best_order = match new_book.last() {
			Some(order) => Some(order.clone()),
			None => None,
		};	

		// Parse the orders into a ShallowBook 
		let mut new_book_state = ShallowBook::new(book_type, block_num, avg_bids, avg_asks, wtd_avg_price, best_order, num_bids, num_asks);
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

	pub fn average_order_prices(orders: &Vec<Order>, market_type: MarketType) -> (Option<f64>, Option<f64>, usize, usize, Option<f64>) {
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
		} else {
			// No asks in book

		}
		if num_bids > 0.0 {
			bids_avg = Some(bids_sum / num_bids);
		} else {
			// No bids in book
		}

		let wtd_avg = History::calc_cur_wtd_price(bids_avg, asks_avg, num_bids as usize, num_asks as usize);


		(bids_avg, asks_avg, num_bids as usize, num_asks as usize, wtd_avg)
	}

	// Iterates over all submitted orders to average the bid and ask price.
	// Returns tuple (avg_bids_price, avg_asks_price, num_bids, num_asks)
	pub fn average_seen_prices(&self) -> (Option<f64>, Option<f64>, u64, u64) {
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

		(bids_avg, asks_avg, num_bids as u64, num_asks as u64)
	}


	pub fn get_last_clearing_price(&self) -> Option<f64> {
		let clearings = self.clearings.lock().unwrap();
		let most_recent = clearings.last();
		match most_recent {
			Some((result, _time)) => result.uniform_price.clone(),
			None => None,
		}
		
	}

	// Returns (best_bid, best_ask) from the most recent order book
	pub fn get_best_orders(&self) -> (Option<Order>, Option<Order>) {
		let books = self.order_books.lock().unwrap();
		let last_index: i64 = books.len() as i64 - 1;
		if last_index == 0 {
			// only have one book to look at
			let shallow_book = books.last().expect("get_best_orders");
			match shallow_book.book_type {
				TradeType::Bid => (shallow_book.best_order.clone(), None),
				TradeType::Ask => (None, shallow_book.best_order.clone()),
			}
		} else if last_index > 0 {
			// More than one book, return two best orders
			let mut best_bid = None;
			let mut best_ask = None;
			// Look at the last book in the history and get best bid or best ask from it
			let last_book = books.last().expect("get_best_orders");
			match last_book.book_type {
				TradeType::Bid => best_bid = last_book.best_order.clone(),
				TradeType::Ask => best_ask = last_book.best_order.clone(),
			}

			// Look at second to last book and get best bid or best ask
			let second_last: usize = (last_index - 1) as usize;
			let next_book = books.get(second_last).expect("get_best_orders");
			match next_book.book_type {
				TradeType::Bid => {
					best_bid = next_book.best_order.clone();
				}
				TradeType::Ask => {
					best_ask = next_book.best_order.clone();
				}
			}
			return (best_bid, best_ask);
		} else {
			return (None, None);
		}
		
	}

	// Returns the most recent list of bids and asks and their volumes: 
	// -> (Vec<bids>, Vec<asks>, bids_volume, asks_volume)
	pub fn get_current_orders(&self) -> (Vec<Order>, Vec<Order>, f64, f64) {
		let mut bids_out = Vec::<Order>::new();
		let mut asks_out = Vec::<Order>::new();
		let mut bids_entries = Vec::<Entry>::new();
		let mut asks_entries = Vec::<Entry>::new();
		{
			let books = self.order_books.lock().unwrap();
			let last_index: i64 = books.len() as i64 - 1;
			if last_index == 0 {
				// only have one book to look at
				let shallow_book = books.last().expect("get_current_orders");
				match shallow_book.book_type {
					TradeType::Bid => {
						bids_entries = shallow_book.orders.clone();
					}
					TradeType::Ask => {
						asks_entries = shallow_book.orders.clone();
					}
				}
			} else if last_index > 0 {
				// More than one book, return two most recent list of entires
				// Look at the last book in the history and get best bid or best ask from it
				let last_book = books.last().expect("get_current_orders");
				match last_book.book_type {
					TradeType::Bid => {
						bids_entries = last_book.orders.clone();
					}
					TradeType::Ask => {
						asks_entries = last_book.orders.clone();
					}
				}

				// Look at second to last book and get best bid or best ask
				let second_last: usize = (last_index - 1) as usize;
				let next_book = books.get(second_last).expect("get_current_orders");
				match next_book.book_type {
					TradeType::Bid => {
						bids_entries = next_book.orders.clone();
					}
					TradeType::Ask => {
						asks_entries = next_book.orders.clone();
					}
				}
			} else {
				// No order books, return empty vecs
				return (bids_out, asks_out, 0.0, 0.0);
			}
		}
		let (mut bids_vol, mut asks_vol) = (0.0, 0.0);
		// Drop lock on the order_books, get the original orders from the entries
		for entry in bids_entries {
			bids_vol += entry.quantity;
			if let Some((order, _time)) = self.find_orig_order(entry.order_id) {
				bids_out.push(order);
			}
		}

		for entry in asks_entries {
			asks_vol += entry.quantity;
			if let Some((order, _time)) = self.find_orig_order(entry.order_id) {
				asks_out.push(order);
			}
		}
		return (bids_out, asks_out, bids_vol, asks_vol);
	}

	pub fn produce_data(&self, mempool: Vec<Order>) -> (PriorData, LikelihoodStats) {
		(self.decision_data(mempool), self.inference_data())
	}


	// Returns the weighted averages of bids and asks seen in the mempool
	pub fn inference_data(&self) -> LikelihoodStats {
		let (mean_bids, mean_asks, num_bids, num_asks) = self.average_seen_prices();
		
		// Avoid divide by zero	
		if num_bids == 0 && num_asks == 0 {
			return LikelihoodStats {
				mean_bids: None,
				mean_asks: None,
				num_bids: num_bids,
				num_asks: num_asks,
				weighted_price: None,
			};
		}
		let raw_bids = match mean_bids {
			Some(price) => Some(price * num_bids as f64),
			None => None,
		};

		let raw_asks = match mean_asks {
			Some(price) => Some(price * num_asks as f64),
			None => None,
		};

		if raw_bids.is_none() && raw_asks.is_none() {
			return LikelihoodStats {
				mean_bids: None,
				mean_asks: None,
				num_bids: num_bids,
				num_asks: num_asks,
				weighted_price: None,
			};
		} else if raw_bids.is_none() && raw_asks.is_some() {
			let weighted_price = Some(raw_asks.unwrap() / num_asks as f64);
			LikelihoodStats {
				mean_bids,
				mean_asks,
				num_bids,
				num_asks,
				weighted_price,
			}
		} else if raw_bids.is_some() && raw_asks.is_none() {
			let weighted_price = Some(raw_bids.unwrap() / num_bids as f64);
			LikelihoodStats {
				mean_bids,
				mean_asks,
				num_bids,
				num_asks,
				weighted_price,
			}
		} else {
			let weighted_price = Some((raw_bids.unwrap() + raw_asks.unwrap()) / (num_asks as f64 + num_bids as f64));
			LikelihoodStats {
				mean_bids,
				mean_asks,
				num_bids,
				num_asks,
				weighted_price,
			}
		}
	}
 
	// calculate the current weighted average price given the mean of bids, asks, and their respective quantities
	pub fn calc_cur_wtd_price(mean_bids: Option<f64>, mean_asks: Option<f64>, num_bids: usize, num_asks: usize) -> Option<f64> {
		// Avoid divide by zero	
		if num_bids == 0 && num_asks == 0 {
			return None;
		}
		let raw_bids = match mean_bids {
			Some(price) => Some(price * num_bids as f64),
			None => None,
		};

		let raw_asks = match mean_asks {
			Some(price) => Some(price * num_asks as f64),
			None => None,
		};

		if raw_bids.is_none() && raw_asks.is_none() {
			return None;
		} else if raw_bids.is_none() && raw_asks.is_some() {
			return Some(raw_asks.unwrap() / num_asks as f64);
		} else if raw_bids.is_some() && raw_asks.is_none() {
			return Some(raw_bids.unwrap() / num_bids as f64);
		} else {
			return Some((raw_bids.unwrap() + raw_asks.unwrap()) / (num_asks as f64 + num_bids as f64));
		}
	}

	pub fn get_weighted_price(&self) -> Option<f64> {
		let books = self.order_books.lock().expect("get_weighted_price");
		let last_book = books.last();
		match last_book {
			Some(book) => book.current_wtd_price,
			None => None,
		}
	}

	pub fn get_mean_gas(pool: &Vec<Order>) -> f64 {
		let num = pool.len();
		if num <= 0 {
			return 0.0;
		}

		let mut sum = 0.0;

		for o in pool {
			sum += o.gas;
		}

		sum / num as f64
	}


	pub fn decision_data(&self, current_pool: Vec<Order>) -> PriorData {
		let clearing_price = self.get_last_clearing_price();
		let (best_bid, best_ask) = self.get_best_orders();
		let (current_bids, current_asks, bids_volume, asks_volume) = self.get_current_orders();
		
		// Get the weighted average price from the last public order book
		let current_wtd_price = self.get_weighted_price();

		// Get the current average gas price in the mmepool 
		let mean_pool_gas = History::get_mean_gas(&current_pool);

		PriorData {
			clearing_price, 
			best_bid,
			best_ask,
			current_bids,
			current_asks,
			current_wtd_price,
			mean_pool_gas, 
			asks_volume,
			bids_volume,
			current_pool,
		}
	}
}











