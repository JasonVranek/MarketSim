use std::sync::Arc;
use core::f64::MAX;
use crate::order::order::{Order, TradeType};

use std::sync::Mutex;
use std::io;

pub fn test_order_book_mod() {
	println!("Hello, order_book!");
}

/// The struct for the order books in the exchange. The purpose
/// is to keep track of bids and asks for calculating order crossings.
/// book_type: TradeType{Bid, Ask} -> To differentiate the two order books
/// orders: Mutex<Vec<Order>> -> Threadsafe vector to keep track of orders
/// min_price: Mutex<f64> -> Threadsafe minimum market price for computing clearing price
/// max_price: Mutex<f64> -> Threadsafe maximum market price for computing clearing price
#[derive(Debug)]
pub struct Book {
	pub book_type: TradeType,
	pub orders: Mutex<Vec<Order>>,
	pub min_price: Mutex<f64>,
	pub max_price: Mutex<f64>,
}

impl Book {
    pub fn new(book_type: TradeType) -> Book {
    	Book {
    		book_type,
    		orders: Mutex::new(Vec::<Order>::new()),
    		min_price: Mutex::new(MAX),
    		max_price: Mutex::new(0.0),
    	}
    }

    /// Adds a new order to the Book after acquiring a lock, then sorts by price
    pub fn add_order(&self, order: Order) -> io::Result<()> {
    	let mut orders = self.orders.lock().expect("ERROR: Couldn't lock book to update order");
    	match order.trade_type {
			// Sort bids in descending order -> best bid (highest price) at end
			TradeType::Bid => {
				orders.push(order);
				orders.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
				// Update best price once book is sorted
				let best_price = orders.last().unwrap().price;
				self.update_best_price(best_price);
			},
			// Sort asks in ascending order -> best ask (lowest price) at end
			TradeType::Ask => {
				orders.push(order);
				// Reverse a and b to get in ascending order
    			orders.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap().reverse());
				// Update best price once book is sorted
				let best_price = orders.last().unwrap().price;
				self.update_best_price(best_price);
			}
		}
		
    	Ok(())
    }

    /// Replaces the order in the order book with the supplied 'order' of the same trader_id
    pub fn update_order(&self, order: Order) -> Result<(), &'static str> {
    	// Acquire the lock
        let mut orders = self.orders.lock().expect("ERROR: Couldn't lock book to update order");
        // Search for existing order's index
        let order_index = orders.iter().position(|o| o.order_id == order.order_id);

        if let Some(i) = order_index {
        	// Add new order to end of the vector
        	orders.push(order);
    		// Swap orders then pop off the old order that is now at the end of vector
        	let last = orders.len() - 1;
        	orders.swap(i, last);
        	orders.pop();
        } else {
        	println!("ERROR: order not found to update: {:?}", &order.order_id);
        	return Err("ERROR: order not found to update");
        }

        Ok(())
    }

    /// Cancels the existing order in the order book if it exists
    pub fn cancel_order(&self, order: Order) -> Result<(), &'static str> {
    	// Acquire the lock
        let mut orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
        // Search for existing order's index
        let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &order.order_id);

        if let Some(i) = order_index {
        	orders.remove(i);
        } else {
        	println!("ERROR: order not found to cancel: {:?}", &order.order_id);
        	return Err("ERROR: order not found to cancel");
        }

		// Update the best price 
        if let Some(last_order) = orders.last(){ 
            let best_price = last_order.price;
            self.update_best_price(best_price);
        } else {
            self.reset_best_price();
        }


        Ok(())
    }

	pub fn cancel_order_by_id(&self, id: u64) -> Result<(), &'static str> {
		// Acquire the lock
        let mut orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
        // Search for existing order's index
        let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &id);

		if let Some(i) = order_index {
        	orders.remove(i);
        } else {
        	println!("ERROR: order not found to cancel: {:?}", id);
        	return Err("ERROR: order not found to cancel");
        }
		// Update the best price 
		if let Some(last_order) = orders.last(){ 
            let best_price = last_order.price;
            self.update_best_price(best_price);
        } else {
            self.reset_best_price();
        }
		

        Ok(())
	}

	// Pushes best bid/ask to end of sorted book
	pub fn push_to_end(&self, order: Order) -> io::Result<()> {
		let mut orders = self.orders.lock().expect("ERROR: Couldn't lock book to update order");
    	orders.push(order);
		Ok(())
	}

	// Pops best bid/ask from end of sorted book
	pub fn pop_from_end(&self) -> Option<Order> {
		let mut orders = self.orders.lock().expect("ERROR: Couldn't lock book to update order");
    	if orders.len() > 0 {
			let order = orders.pop();
			return order;
		} 
		return None
	}

	pub fn merge_sort_books(book1: Arc<Book>, book2: Arc<Book>) -> Book {
		let merged = Book::new(TradeType::Bid);
		{
			let mut m_orders = merged.orders.lock().expect("Error...");
			let b1_orders = book1.orders.lock().expect("ERROR: Couldn't lock book to update order");
			for o in b1_orders.iter() {
				m_orders.push(o.clone());
			}

			let b2_orders = book2.orders.lock().expect("ERROR: Couldn't lock book to update order");
			for o in b2_orders.iter() {
				m_orders.push(o.clone());
			}
		}

		merged.sort_desc_price();
		return merged;
	}

    // Puts orders with lower prices at the end of array, so iterating is descending, popping is ascending.
	pub fn sort_desc_price(&self) {
    	// Acquire the lock
        let mut orders = self.orders.lock().expect("ERROR: Couldn't lock book to sort");
		// Sort orders in descending order
		orders.sort_by(|a, b| a.price.partial_cmp(&b.price).expect("Failed to sorted").reverse());
    }

    pub fn peek_id_pos(&self, trader_id: String) -> Option<usize> {
    	// Acquire the lock
        let orders = self.orders.lock().unwrap();
        // Search for existing order's index
        orders.iter().position(|o| o.trader_id == trader_id)
    }

    /// Utility to see depth of order book
    pub fn len(&self) -> usize {
    	let orders = self.orders.lock().unwrap();
    	orders.len()
    }

	/// Atomically updates Book's best bid/ask
	pub fn update_best_price(&self, price: f64) {
		match self.book_type {
			TradeType::Bid => {
				let mut max_p = self.max_price.lock().unwrap();
				*max_p = price;
			},
			TradeType::Ask => {
				let mut min_p = self.min_price.lock().unwrap();
				*min_p = price;
			}
		}
	}

	pub fn peek_best_price(&self) -> Option<f64> {
		let orders = self.orders.lock().unwrap();
		if orders.len() > 0 {
			return Some(orders.last().expect("Couldn't peek best price").price);
		}
		None
	}

    /// Atomically updates the Book's max price
    pub fn update_max_price(&self, p_high: &f64) {
		let mut max_price = self.max_price.lock().unwrap();
		if *p_high > *max_price {
			*max_price = *p_high;
		} 
    }

    /// Atomically updates the Book's min price
	pub fn update_min_price(&self, p_low: &f64) {
		let mut min_price = self.min_price.lock().unwrap();
		if *p_low < *min_price {
			*min_price = *p_low;
		} 
    }

    /// Returns the Book's min price
    pub fn get_min_price(&self) -> f64 {
    	let price = self.min_price.lock().expect("Error getting min price");
    	price.clone() as f64
    }

    /// Returns the Book's max price
    pub fn get_max_price(&self) -> f64 {
    	let price = self.max_price.lock().expect("Error getting max price");
    	price.clone() as f64
    }

    /// Returns sum of book's volume
    pub fn get_book_volume(&self) -> f64 {
    	let orders = self.orders.lock().expect("couldn't acquire lock");
    	orders.iter().map(|o| o.quantity).sum()
    }

    /// Returns lowest p_low for the book
    pub fn get_min_plow(&self) -> f64 {
    	let orders = self.orders.lock().expect("couldn't acquire lock");
    	let mut p_low = MAX;
    	for order in orders.iter() {
    		if order.p_low < p_low {
    			p_low = order.p_low;
    		}
    	}
    	p_low
    }

    /// Returns highest p_high for the book
    pub fn get_max_phigh(&self) -> f64 {
    	let orders = self.orders.lock().expect("couldn't acquire lock");
    	let mut p_high = 0.0;
    	for order in orders.iter() {
    		if order.p_high > p_high {
    			p_high = order.p_high;
    		}
    	}
    	p_high
    }

    /// Finds a new maximum Book price in the event that the previous was
    /// updated or cancelled and updates the Book. Utilizes Book being sorted by p_high
    pub fn find_new_max(&self) {
    	// find the order with the max price (from sorted list):
    	let orders = self.orders.lock().unwrap();

    	let new_max = orders.last().unwrap().price; //UNSAFE!

    	// Update the book with new max price
    	let mut max_price = self.max_price.lock().unwrap();
    	*max_price = new_max;
    }

    /// Finds a new minimum Book price in the event that the previous was
    /// updated or cancelled and updates the Book.
    pub fn find_new_min(&self) {
    	let orders = self.orders.lock().unwrap();

    	// Iterates over all orders until a minimum is found
    	let new_min = orders.iter().fold(MAX, |min, order| if order.price < min {order.price} else {min});

    	// Update the book with new min price
    	let mut min_price = self.min_price.lock().unwrap();
    	*min_price = new_min;
    }

    pub fn copy_orders(&self) -> Vec<Order> {
        let orders = self.orders.lock().unwrap();
        let mut v = Vec::new();
        for o in orders.iter() {
            v.push(o.clone());
        }
        v

    }

    pub fn reset_best_price(&self) {
        match self.book_type {
            TradeType::Bid => {
                {
                    let mut minp = self.min_price.lock().unwrap();
                    *minp = MAX;
                }
                {
                    let mut maxp = self.max_price.lock().unwrap();
                    *maxp = 0.0;
                }
            }
            TradeType::Ask => {
                {
                    let mut minp = self.min_price.lock().unwrap();
                    *minp = 0.0;
                }
                {
                    let mut maxp = self.max_price.lock().unwrap();
                   *maxp = MAX;
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
	use super::*;
    use crate::order::order::{TradeType};
    use std::sync::Arc;
    use std::thread;

	#[test]
	fn test_new_book() {
		let book = Book::new(TradeType::Bid);
		assert_eq!(book.book_type, TradeType::Bid);
		assert_eq!(*book.min_price.lock().unwrap(), MAX);
		assert_eq!(*book.max_price.lock().unwrap(), 0.0);
	}

	#[test]
	fn test_book_mutex() {
		// Make sure not to acquire another lock in the same scope or it will deadlock
		let book = Arc::new(Book::new(TradeType::Bid));
		let mut handles = Vec::new();
		{
			// spawn 10 threads to update the book
			for _ in 0..10 {
				// Create a threadsafe cloned reference to mutex
				let book = Arc::clone(&book);

				let handle = thread::spawn(move || {
					// Acquire lock and update book in separate thread
					let mut max_price = book.max_price.lock().unwrap();
					// dereference the mutex to modify
					*max_price += 5.0;
				});
				handles.push(handle);
			}
			
		}
		// Wait for all the threads to finish
		for handle in handles {
			handle.join().unwrap();
		}

		assert_eq!(*book.max_price.lock().unwrap(), 50.0);

	}
}























