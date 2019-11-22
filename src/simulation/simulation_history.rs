use flowrs::order::Order;
use flowrs::order::order_book::Book;
use flowrs::utlity::get_time_now;

pub struct Entry {
	pub order_id: u64,
	pub quantity: f64,	// Only thing that changes with order
	pub timestamp: Duration,
}

// Shallow copy of each order
pub struct ShallowBook { // this should be a vec 
	pub books: Mutex<HashMap<Vec<Entry>>>>,
	pub block_num: u32,
}

impl ShallowBook {
	pub fn new(order_id, quantity) -> Self {
		ShallowBook {
			order_id: order_id,
			quantity: quantity,
			timestamp: get_time_now(),
		}
	}
}

// A struct to track the state of the simulation for logging
// and player strategies
pub struct History {
	pub mempool_data: Mutex<Hashmap<u64, Order>>,
	pub order_books: Mutex<Vec<ShallowBook>>,
}


impl History {
	pub fn new() -> History {
		History {
			mempool_data: Mutex::new(HashMap::new()),
			order_books: Mutex::new(Vec::new()),
		}
	}

	pub fn mempool_order(&self, order: Order) {
		let mut pool = self.mempool_data.lock().expect("History mempool lock");
		pool.push(order);
	}

	pub fn orderbook_change(&self, new_book: Book) {
		let mut pool = self.order_books.lock().expect("History mempool lock");
		pool.push(order);
	}
}
