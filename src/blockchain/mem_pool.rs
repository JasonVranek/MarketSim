use crate::order::order::Order;
use std::sync::Mutex;


/// A threadsafe FIFO queue to store unprocessed messages arriving from players.
pub struct MemPool {
    pub items: Mutex<Vec<Order>>,
}

impl MemPool {
	pub fn new() -> MemPool {
		MemPool {
			items: Mutex::new(Vec::<Order>::new()),
		}
	}

	// New orders are pushed to the end of the MemPool
	pub fn add(&self, order: Order) {
        let mut items = self.items.lock().expect("Error locking Mempool");
        items.push(order);
	}

	pub fn pop(&self) -> Option<Order> {
		let mut items = self.items.lock().expect("Error locking Mempool");
		items.pop()
	}

	pub fn sort_by_gas(&self) {
		let mut items = self.items.lock().expect("Error locking Mempool");
		// Sort in descending gas order
		items.sort_by(|a, b| a.gas.partial_cmp(&b.gas).unwrap().reverse());
	}

	// Empties the MemPool into a vector of Orders. Drain() pops the items
	// out in the order of arrival, so once iterated upon, orders will be 
	// processed first -> last.
	pub fn pop_all(&self) -> Vec<Order> {
		// Acquire the lock
		let mut items = self.items.lock().expect("Error locking Mempool");
		// Pop all items out of the queue and return the contents as a vec
		items.drain(..).collect()
	}

	pub fn pop_n(&self, n: usize) -> Vec<Order> {
		// Acquire the lock
		let mut items = self.items.lock().expect("Error locking Mempool");
		// Pop all items out of the queue and return the contents as a vec
		items.drain(0..n).collect()
	}

	pub fn length(&self) -> usize {
		let items = self.items.lock().expect("Error locking Mempool");
		items.len()
	}
}