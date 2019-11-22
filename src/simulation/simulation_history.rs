use crate::order::order::{Order, TradeType};
use crate::order::order_book::Book;
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
	pub book_type: TradeType,
}

impl ShallowBook {
	pub fn new(bid_or_ask: TradeType, num: u64) -> Self {
		ShallowBook {
			orders: Vec::new(),
			block_num: num,
			book_type: bid_or_ask,
		}
	}

	pub fn new_entry(&mut self, e: Entry) {
		self.orders.push(e);
	}
}

// A struct to track the state of the simulation for logging
// and player strategies
pub struct History {
	pub mempool_data: Mutex<HashMap<u64, (Order, Duration)>>,
	pub order_books: Mutex<Vec<ShallowBook>>,
}


impl History {
	pub fn new() -> History {
		History {
			mempool_data: Mutex::new(HashMap::new()),
			order_books: Mutex::new(Vec::new()),
		}
	}

	// Adds an order indexed by its order id to a history of all orders to mempool 
	pub fn mempool_order(&self, order: Order) {
		let mut pool = self.mempool_data.lock().expect("History mempool lock");
		pool.insert(order.order_id, (order, get_time()));
	}

	// Parses through the orders and creates a shallow clone of the book
	pub fn clone_book_state(&self, new_book: Vec<Order>, book_type: TradeType, block_num: u64) {
		// Parse the orders into a ShallowBook 
		let mut new_book_state = ShallowBook::new(book_type, block_num);
		for order in new_book.iter() {
			new_book_state.new_entry(Entry::new(order.order_id, order.quantity));
		}

		let mut prev_histories = self.order_books.lock().expect("History mempool lock");
		prev_histories.push(new_book_state);
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
}











