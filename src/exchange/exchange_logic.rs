use crate::controller::{Task, State};
use crate::order::order_book::Book;
use crate::order::order::Order;

use std::sync::{Mutex, Arc};
use std::cmp::Ordering;

use rayon::prelude::*;
use crate::utility::get_time;


const EPSILON: f64 =  0.000_000_001;
const MAX_PRICE: f64 = 999_999_999.0;
const MIN_PRICE: f64 = 0.0;


pub struct Auction {}

// TODO replace prints with way to log tx's

impl Auction {
	// Checks whether the new bid crosses the best ask. 
	// A new bid will cross at best ask.price iff best ask.price ≤ new bid.price
	// If the new order's quantity is not satisfied, the next best ask is checked.
	pub fn calc_bid_crossing(bids: Arc<Book>, asks:Arc<Book>, mut new_bid: Order) {
		if new_bid.price >= asks.get_min_price() {
			// buying for more than best ask is asking for -> tx @ ask price
			// Get the best ask from book, if there is one, else nothing to cross so add bid to book
			let mut best_ask = match asks.pop_from_end() {
				Some(order) => order,
				None => {
					bids.add_order(new_bid).expect("Failed to add bid to book...");
					return
				}
			};
			// Modify quantities of best ask and new bid
			match new_bid.quantity.partial_cmp(&best_ask.quantity).expect("bad cmp") {
				Ordering::Less => {
					// This new bid will be satisfied and not be added to the book
					best_ask.quantity -= new_bid.quantity;
					trace!("New bid:{} transacted {} shares with best ask:{} @{}", 
							new_bid.trader_id, new_bid.quantity, best_ask.trader_id, best_ask.price);
					// Return the best ask to the book
					asks.push_to_end(best_ask).expect("couldn't push");
				},
				Ordering::Greater => {
					// This new bid potentially will cross with multiple asks
					new_bid.quantity -= best_ask.quantity;
					info!("New bid:{} transacted {} shares with best ask:{} @{}, clearing best ask from book", 
							new_bid.trader_id, best_ask.quantity, best_ask.trader_id, best_ask.price);
					
					// Update the best ask price 
					match asks.peek_best_price() {
						Some(price) => {
							// There are more asks in the book
							asks.update_best_price(price);
						},
						None => {
							// No more asks in the book, need to add this bid to book, set default best ask price
							asks.update_best_price(MAX_PRICE);
							bids.add_order(new_bid).expect("Failed to add bid to book...");
							return
						}
					}
					// Don't return the bid to the book
					
					// Recursively check if new bid will fill more orders:
					Auction::calc_bid_crossing(bids, asks, new_bid);
				},
				Ordering::Equal => {
					// new bid clears the best ask removing it from book
					info!("New bid:{} transacted {} shares with best ask:{} @{}, clearing best ask from book", 
							new_bid.trader_id, new_bid.quantity, best_ask.trader_id, best_ask.price);

					// Update the best ask price 
					match asks.peek_best_price() {
						Some(price) => {
							// There are more asks in the book
							asks.update_best_price(price);
						},
						None => {
							// No more asks in the book, set default best ask price
							asks.update_best_price(MAX_PRICE);
						}
					}
					// Don't return the bid to the book
				}
			}  
		} else {
			// New bid didn't cross, needs to be added to the book
			bids.add_order(new_bid).expect("Failed to add bid to book...");
		}
	}

	// Checks whether the new ask crosses the best bid. 
	// A new ask will cross at best bid.price iff best bid.price ≥ new ask.price
	// If the new order's quantity is not satisfied, the next best bid is checked.
	pub fn calc_ask_crossing(bids: Arc<Book>, asks:Arc<Book>, mut new_ask: Order) {
		if new_ask.price <= bids.get_max_price() {
			// asking for less than best bid willing to pay -> tx @ bid price
			// Modify quantities of best bid and this new ask
			let mut best_bid = match bids.pop_from_end() {
				Some(order) => order,
				None => {
					asks.add_order(new_ask).expect("Failed to add ask to book...");
					return
				}
			};
			match new_ask.quantity.partial_cmp(&best_bid.quantity).expect("bad cmp") {
				Ordering::Less => {
					// This new ask will be satisfied and not be added to the book
					best_bid.quantity -= new_ask.quantity;
					println!("New ask:{} transacted {} shares with best bid:{} @{}", 
							new_ask.trader_id, new_ask.quantity, best_bid.trader_id, best_bid.price);
					// Return the best bid to the book
					bids.push_to_end(best_bid).expect("bad push");
				},
				Ordering::Greater => {
					// This new ask potentially will cross with multiple bids
					new_ask.quantity -= best_bid.quantity;
					println!("New ask:{} transacted {} shares with best bid:{} @{}, clearing best bid from book", 
							new_ask.trader_id, best_bid.quantity, best_bid.trader_id, best_bid.price);
					
					// Update the best bid price 
					match bids.peek_best_price() {
						Some(price) => {
							// There are more asks in the book
							bids.update_best_price(price);
						},
						None => {
							// No more bids in the book, need to add this ask to book, set default best bid price
							bids.update_best_price(MIN_PRICE);
							asks.add_order(new_ask).expect("Failed to add ask to book...");
							return
						}
					}
					// Recursively check if new ask will fill more orders:
					Auction::calc_ask_crossing(bids, asks, new_ask);
				},
				Ordering::Equal => {
					// new ask clears the best bid removing it from book
					println!("New ask:{} transacted {} shares with best bid:{} @{}, clearing best bid from book", 
							new_ask.trader_id, new_ask.quantity, best_bid.trader_id, best_bid.price);
					
					// Update the best bid price 
					match bids.peek_best_price() {
						Some(price) => {
							// There are more asks in the book
							bids.update_best_price(price);
						},
						None => {
							// No more bids in the book, need to add this ask to book, set default best bid price
							bids.update_best_price(MIN_PRICE);
						}
					}
					// Don't return the bid ot the book
				}
			}  
		} else {
			// New ask didn't cross, needs to be added to the book
			asks.add_order(new_ask).expect("Failed to add ask to book...");
		}
	}

	

	// Calculates which orders in the order book will transact at auction time.
	pub fn frequent_batch_auction(bids: Arc<Book>, asks: Arc<Book>) -> Option<f64> {
		unimplemented!();
	}

	/// Schedules an auction to run on an interval determined by the duration parameter in milliseconds.
	/// Outputs a task that will be dispatched asynchronously via the controller module.
	pub fn async_auction_task(bids: Arc<Book>, asks: Arc<Book>, state: Arc<Mutex<State>>, duration: u64) -> Task {
		Task::rpt_task(move || {
			{
	    		// Obtain lock on the global state and switch to Auction mode, will stop
	    		// the queue from being processed.
	    		let mut state = state.lock().unwrap();
	    		*state = State::Auction;
	    	}
	    	println!("Starting Auction @{:?}", get_time());
	    	if let Some(cross_price) = Auction::frequent_batch_auction(Arc::clone(&bids), Arc::clone(&asks)) {
	    		println!("Found Cross at @{:?} \nP = {}\n", get_time(), cross_price);
	    	} else {
	    		println!("Error, Cross not found\n");
	    	}
	    	
	    	{
	    		// Change the state back to process to allow the books to be mutated again
	    		let mut state = state.lock().unwrap();
	    		*state = State::Process;
	    	}
		}, duration)
	}

	pub fn get_price_bounds(bids: Arc<Book>, asks: Arc<Book>) -> (f64, f64) {		
		let bids_min: f64 = bids.get_min_price();
		let bids_max: f64 = bids.get_max_price();
		let asks_min: f64 = asks.get_min_price();
		let asks_max: f64 = asks.get_max_price();

		(Auction::min_float(&bids_min, &asks_min), Auction::max_float(&bids_max, &asks_max))
	}

	fn max_float(a: &f64, b: &f64) -> f64 {
	    match a.partial_cmp(b).unwrap() {
			Ordering::Less => *b,
			Ordering::Greater => *a,
			Ordering::Equal => *a
		}
	}

	fn min_float(a: &f64, b: &f64) -> f64 {
	    match a.partial_cmp(b).unwrap() {
			Ordering::Less => *a,
			Ordering::Greater => *b,
			Ordering::Equal => *a
		}
	}

	// true if a > b
	pub fn greater_than_e(a: &f64, b: &f64) -> bool {
		let a = a.abs();
		let b = b.abs();
	    if (a - b).abs() > EPSILON && a - b > 0.0 {
	    	return true;
	    } else {
	    	return false;
	    }
	}

	// true if a < b
	pub fn less_than_e(a: &f64, b: &f64) -> bool {
		let a = a.abs();
		let b = b.abs();
	    if (a - b).abs() > EPSILON && a - b < 0.0 {
	    	return true;
	    } else {
	    	return false;
	    }
	}

	pub	fn equal_e(a: &f64, b: &f64) -> bool {
	    if (a - b).abs() < EPSILON {
	    	return true;
	    } else {
	    	return false;
	    }
	}
}



#[test]
fn test_par_iter() {
	let big_sum: u32 = (0..10).collect::<Vec<u32>>()
		.par_iter()
	    .map(|x| x * x)
	    .sum();

	assert_eq!(big_sum, 285);
}

#[test]
fn test_min_max_float() {
	let a = 2.0;
	let b = 10.0;
	assert_eq!(2.0, Auction::min_float(&a, &b));
	assert_eq!(10.0, Auction::max_float(&a, &b));
}

#[test]
fn test_float_helpers() {
	let a = 2.0;
	let b = 10.0;
	assert_eq!(2.0, Auction::min_float(&a, &b));
	assert_eq!(10.0, Auction::max_float(&a, &b));

	assert!(!Auction::greater_than_e(&a, &b));
	assert!(Auction::less_than_e(&a, &b));
	assert!(Auction::equal_e(&(1.1 + 0.4), &1.5));
}













