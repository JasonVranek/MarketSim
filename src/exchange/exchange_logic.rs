use crate::controller::{Task, State};
use crate::order::order_book::Book;
use crate::order::order::{Order, TradeType};
use crate::exchange::MarketType;
use crate::utility::get_time;

use std::sync::{Mutex, Arc};
use std::cmp::Ordering;

use rayon::prelude::*;
use math::round;


const EPSILON: f64 =  0.000_001;
const MAX_PRICE: f64 = 999_999_999.0;
const MIN_PRICE: f64 = 0.0;
const MAX_ITERS: usize = 1000;
const PRECISION: i8 = 4;

#[derive(Debug)]
pub struct PlayerUpdate {
	pub payer_id: String,
	pub vol_filler_id: String,
	pub payer_order_id: u64,
	pub vol_filler_order_id: u64,
	pub price: f64,
	pub volume: f64,
}

impl PlayerUpdate {
	pub fn new(payer_id: String, vol_filler_id: String, payer_order_id: u64, 
		vol_filler_order_id: u64, price: f64, volume: f64) -> PlayerUpdate {
		PlayerUpdate {
			payer_id,
			vol_filler_id,
			payer_order_id,
			vol_filler_order_id,
			price,
			volume,
		}
	}
}

#[derive(Debug)]
pub struct TradeResults {
	pub auction_type: MarketType,
	pub uniform_price: Option<f64>,
	pub agg_demand: f64,
	pub agg_supply: f64,
	pub cross_results: Option<Vec<PlayerUpdate>>,
}

impl TradeResults {
	pub fn new(a_t: MarketType, p: Option<f64>, agg_d: f64, agg_s: f64, player_updates: Option<Vec<PlayerUpdate>>) -> TradeResults {
		TradeResults {
			auction_type: a_t,
			uniform_price: p,
			agg_demand: agg_d,
			agg_supply: agg_s,
			cross_results: player_updates

		}
	}
}

pub struct Auction {}

// TODO replace prints with way to log tx's

impl Auction {

	pub fn run_auction(bids: Arc<Book>, asks:Arc<Book>, m_t: MarketType) -> Option<TradeResults>{
		match m_t {
			MarketType::CDA => None,
			MarketType::FBA => {
				Auction::frequent_batch_auction(bids, asks)
			},
			MarketType::KLF => {
				Auction::bs_cross(bids, asks)
			},
		}
	}

	/// ***CDA function***
	/// Checks whether the new bid crosses the best ask. 
	/// A new bid will cross at best ask.price iff best ask.price ≤ new bid.price
	/// If the new order's quantity is not satisfied, the next best ask is checked.
	pub fn calc_bid_crossing(bids: Arc<Book>, asks:Arc<Book>, mut new_bid: Order) {
		if new_bid.price >= asks.get_min_price() {
			// buying for more than best ask is asking for -> tx @ ask price
			// Get the best ask from book if there is one, else nothing to cross so add bid to book
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

	/// ***CDA function***
	/// Checks whether the new ask crosses the best bid. 
	/// A new ask will cross at best bid.price iff best bid.price ≥ new ask.price
	/// If the new order's quantity is not satisfied, the next best bid is checked.
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

	

	/// **FBA function**
	/// Calculates the uniform clearing price for the orders in the bids and asks books.
	/// Orders are sorted by price (descending for bids, ascending for asks).
	/// Outputs the uniform clearing price if it exists and the total trade volume
	pub fn frequent_batch_auction(bids: Arc<Book>, asks: Arc<Book>) -> Option<TradeResults> {
		// Check if auction necessary
		if bids.len() == 0 || asks.len() == 0 {
			let result = TradeResults::new(MarketType::FBA, None, 0.0, 0.0, None);
			return Some(result);
		}

		// There will be no crossings if best bid < best ask
		if bids.get_max_price() < asks.get_min_price() {
			let result = TradeResults::new(MarketType::FBA, None, 0.0, 0.0, None);
			return Some(result);
		}

		// Calc total ask volume 
		let ask_book_vol = asks.get_book_volume();
		// Merge both books and sort in decreasing price order 
		let merged_book = Book::merge_sort_books(Arc::clone(&bids), Arc::clone(&asks));

		// Initialize the min and max prices seen while traversing the merged book
		let mut max_seen_price = MIN_PRICE;
		let mut min_seen_price = MAX_PRICE;
		let mut clearing_price: Option<f64> = None;

		// Initialize vars to track volume seen while traversing the merged book
		let mut seen_vol = 0.0;
		let mut prev_seen_vol = 0.0;
		let mut prev_order_price = 0.0;	// is 0.0 acceptable?
		let mut cur_order_price = 0.0;

		// Iterate through descending orders. Sum volume of each order and track the min and max seen prices
		let orders = merged_book.orders.lock().expect("ERROR: Couldn't lock book to sort");
		println!("Calculating clearing price...");
		for order in orders.iter() {
			cur_order_price = order.price;
			// Process best prices
			if cur_order_price > max_seen_price {
				max_seen_price = cur_order_price;
			}
			if cur_order_price < min_seen_price {
				min_seen_price = cur_order_price;
			}

			// Process seen volumes
			prev_seen_vol = seen_vol;
			seen_vol += order.quantity;
			println!("Checking price:{}, seen_vol:{} / ask_vol:{}", cur_order_price, seen_vol, ask_book_vol);
			if seen_vol >= ask_book_vol {
				// NOTE: darrell's implementation didn't include <=, just <, but this fixed horizontal cross edge case
				break;
			}
			// Track the price of the last traversed order
			prev_order_price = cur_order_price;
		}	

		// If we have still not found a max or min seen price, loop until we do:
		if max_seen_price == MIN_PRICE || min_seen_price == MAX_PRICE {
			for order in orders.iter() {
				cur_order_price = order.price;
				// Process best prices
				if cur_order_price > max_seen_price {
					max_seen_price = cur_order_price;
				}
				if cur_order_price < min_seen_price {
					min_seen_price = cur_order_price;
				}
				println!("Looping until price < {}, cur_price={}", MAX_PRICE, cur_order_price);
				if cur_order_price < MAX_PRICE {
					break;
				}
			}
		}

		// Find the clearing price
		if max_seen_price == MIN_PRICE && min_seen_price == MAX_PRICE {
			// We weren't able to find a clearing price
			clearing_price = None;
		} 
		// We perfectly matched volume
		else if seen_vol == ask_book_vol {	
			if prev_order_price == MAX_PRICE && MIN_PRICE < cur_order_price && cur_order_price < MAX_PRICE {
				// The current order crossed, so use this price
				clearing_price = Some(cur_order_price);
			} 
			
			else if prev_order_price < MAX_PRICE && MIN_PRICE < cur_order_price {
				// let p = round::ceil((prev_order_price + cur_order_price) / 2.0, PRECISION);
				let p = (prev_order_price + cur_order_price) / 2.0;		// NOTE changed this from darrell's...confirm with dan
				clearing_price = Some(p);
			}

			else if MIN_PRICE < prev_order_price && prev_order_price < MAX_PRICE && cur_order_price == MIN_PRICE {
				clearing_price = Some(prev_order_price);
			}

			else if prev_order_price == MIN_PRICE {
				clearing_price = Some(min_seen_price);
			}
		}
		// The last order's volume caused us to cross
		else if seen_vol > ask_book_vol {
			clearing_price = Some(Auction::max_float(&cur_order_price, &min_seen_price));
		}

		println!("Clearing price: {:?}", clearing_price);

		

		// Initialize updates to send to ClearingHouse
		let mut updates = Vec::<PlayerUpdate>::new();

		let mut result = TradeResults::new(MarketType::FBA, clearing_price, 0.0, 0.0, None);

		let mut cancel_bids = Vec::<u64>::new();
		let mut _vol_filled = 0.0;

		// If we have a clearing price, calculate which orders transact and at what volume, otherwise exit returning results
		match clearing_price {
			None => return Some(result),
			Some(cp) => {
				// Lock bids book 
				// let mut bids_descending = bids.orders.lock().expect("ERROR: Couldn't lock book");
				
				// Iterate over bids in (last item in bids (first to pop) is best bid, so must iterate in reverse order)
				// for cur_bid in bids_descending.iter_mut().rev() {
				loop {
					// Pop the best bid from the bids book if it exists
					let mut cur_bid = match bids.pop_from_end() {
						Some(bid) => bid,
						None => break,
					};
					let bid_price = cur_bid.price;

					// Pop the best ask from the asks book if it exists
					let mut cur_ask = match asks.pop_from_end() {
						Some(ask) => ask,
						None => {
							bids.push_to_end(cur_bid).expect("Couldn't push order");
							break;
						},
					};
					let ask_price = cur_ask.price;

					// Check whether we will cross at all
					if bid_price < cp || ask_price > cp {
						println!("breaking out of loop...cp={}, bp={}, ap={}", cp, bid_price, ask_price);
						// A bid with price < cp will not tx, same with ask with price > cp
						// Return the popped ask to the book before exiting
						bids.push_to_end(cur_bid).expect("Couldn't push order");
						asks.push_to_end(cur_ask).expect("Couldn't push order");
						break;
					}
					// The current bid will exchange at clearing price with current ask
					match cur_bid.quantity.partial_cmp(&cur_ask.quantity).expect("bad cmp") {
						Ordering::Less => {
							println!("cur bid: {} volume < cur ask volume {}", cur_bid.order_id, cur_ask.order_id);
							// cur_bid's interest is less than the cur_ask's volume
							let trade_amount = cur_bid.quantity;
							cur_ask.quantity -= trade_amount;
							cur_bid.quantity = 0.0;
							_vol_filled += trade_amount;
							// Information to be sent to clearing house
							updates.push(PlayerUpdate::new(cur_bid.trader_id.clone(), 
											  cur_ask.trader_id.clone(), 
											  cur_bid.order_id, 
											  cur_ask.order_id.clone(), 
											  cp, trade_amount));
							// Cancel the bid from the book
							cancel_bids.push(cur_bid.order_id);
							// Return the ask for next loop iteration
							asks.push_to_end(cur_ask).expect("Couldn't push order");
						},
						Ordering::Greater => {
							println!("cur bid: {} volume > cur ask volume {}", cur_bid.order_id, cur_ask.order_id);
							// cur_bid's interest is more than the cur_ask's volume
							let trade_amount = cur_ask.quantity;
							cur_ask.quantity = 0.0;
							cur_bid.quantity -= trade_amount;
							_vol_filled += trade_amount;
							// Information to be sent to clearing house
							updates.push(PlayerUpdate::new(cur_bid.trader_id.clone(), 
											  cur_ask.trader_id.clone(), 
											  cur_bid.order_id, 
											  cur_ask.order_id, 
											  cp, trade_amount));
							// Cancel ask order since was filled (Simply don't add it back to the book...)
							// This bid's interest is not fully filled so return it to be used again:
							bids.push_to_end(cur_bid).expect("Couldn't push order");
						},
						Ordering::Equal => {
							println!("cur bid: {} volume = cur ask volume {}", cur_bid.order_id, cur_ask.order_id);
							// cur_bid's interest is equal to the cur_ask's volume
							let trade_amount = cur_bid.quantity;
							cur_ask.quantity = 0.0;
							cur_bid.quantity = 0.0;
							_vol_filled += trade_amount;
							// Information to be sent to clearing house
							updates.push(PlayerUpdate::new(cur_bid.trader_id.clone(), 
											  cur_ask.trader_id.clone(), 
											  cur_bid.order_id, 
											  cur_ask.order_id, 
											  cp, trade_amount));

							// Cancel bid order from bids books
							cancel_bids.push(cur_bid.order_id);

							// Cancel ask order since was filled (Simply don't add it back to the book...)
						}
					}
				}
			}
		}
		// Execute bid cleaning outside of scope where bids were borrwed so no deadlock.
		// Clean the books by removing all orders with quanitity = 0
		// for o_id in cancel_bids {
		// 	println!("Cancelling order with oid: {}", o_id);
		// 	bids.cancel_order_by_id(o_id).expect("Couldn't cancel");
		// }

		result.agg_demand = _vol_filled;
		result.agg_supply = _vol_filled;
		// Add all of the PlayerUpdates to our TradeResults
		result.cross_results = Some(updates);
		return Some(result)
	}

	/// FBA clearing price using binary search...
	pub fn frequent_batch_auction2(bids: Arc<Book>, asks: Arc<Book>) -> Option<TradeResults> {
		unimplemented!();

		// Check if auction necessary
		if bids.len() == 0 || asks.len() == 0 {
			let result = TradeResults::new(MarketType::FBA, None, 0.0, 0.0, None);
			return Some(result);
		}

		// There will be no crossings if best bid < best ask
		if bids.get_max_price() < asks.get_min_price() {
			let result = TradeResults::new(MarketType::FBA, None, 0.0, 0.0, None);
			return Some(result);
		}

		// Start from the bids in descending order and calculate total seen volume as you move left

		// Start from the asks in ascending order and calculate total seen volume as you move right

		// If bid price >= ask price, we have crossed. uniform price = 

		// Volume to trade is the min(seen_bid_vol, seen_ask_vol)


		return None;
	}


	/// Helper function for Flow Order clearing price calculation: bs_cross
	/// Iterate over each order in parallel and compute the aggregate supply and
	/// demand using the order's p_low, p_high, and quantity (u_max).
	pub fn calc_aggs(p: f64, bids: Arc<Book>, asks: Arc<Book>) -> (f64, f64) {
		let bids = bids.orders.lock().expect("ERROR: No bids book");
		let asks = asks.orders.lock().expect("ERROR: No asks book");

		let agg_demand: f64 = bids.par_iter()
		    .map(|order| {
		    	if p <= order.p_low {
		    		order.quantity
		    	} else if p > order.p_high {
		    		0.0
		    	} else {
		    		order.calc_flow_demand(p)
		    	}
		    }).sum();

		let agg_supply: f64 = asks.par_iter()
		    .map(|order| {
		    	if p < order.p_low {
		    		0.0
		    	} else if p >= order.p_high {
		    		order.quantity
		    	} else {
		    		order.calc_flow_supply(p)
		    	}
		    }).sum();

		(agg_demand, agg_supply)
	}

	/// **KLF function**
	/// Calculates the market clearing price from the bids and asks books. Uses a 
	/// binary search to find the intersection point between the aggregates supply and 
	/// demand curves. 
	pub fn bs_cross(bids: Arc<Book>, asks: Arc<Book>) -> Option<TradeResults> {
		// get_price_bounds obtains locks on the book's prices
	    let (mut left, mut right) = Auction::get_price_bounds(Arc::clone(&bids), Arc::clone(&asks));
	    let mut curr_iter = 0;
	    println!("Min Book price: {}, Max Book price: {}", left, right);
	    while left < right {
	    	curr_iter += 1;
	    	// Find a midpoint with the correct price tick precision
	    	let index: f64 = (left + right) / 2.0;
	    	// Calculate the aggregate supply and demand at this price
	    	let (dem, sup) = Auction::calc_aggs(index, Arc::clone(&bids), Arc::clone(&asks));
	    	// println!("price_index: {}, dem: {}, sup: {}", index, dem, sup);

	    	if Auction::greater_than_e(&dem, &sup) {  		// dev > sup
	    		// We are left of the crossing point
	    		left = index;
	    	} else if Auction::less_than_e(&dem, &sup) {	// sup > dem
	    		// We are right of the crossing point
	    		right = index;
	    	} else {
	    		println!("Found cross at: {}", index);
	    		let mut result = TradeResults::new(MarketType::KLF, Some(index), dem, sup, None);
	    		// Push the player updates for updating the player's state in ClearingHouse
	    		let player_updates = Auction::flow_player_updates(index, Arc::clone(&bids), Arc::clone(&asks));
	    		result.cross_results = Some(player_updates);
	    		return Some(result);
	    	}

	    	if curr_iter == MAX_ITERS {
	    		println!("Trouble finding cross in max iterations, got: {}", index);
	    		let mut result = TradeResults::new(MarketType::KLF, Some(index), dem, sup, None);
	    		// Push the player updates for updating the player's state in ClearingHouse
	    		let player_updates = Auction::flow_player_updates(index, Arc::clone(&bids), Arc::clone(&asks));
	    		result.cross_results = Some(player_updates);
	    		return Some(result);
	    	}
	    }
	    None
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
	    	if let Some(result) = Auction::frequent_batch_auction(Arc::clone(&bids), Arc::clone(&asks)) {
	    		println!("Found Cross at @{:?} \nP = {}\n", get_time(), result.uniform_price.unwrap());
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

	// helper function to calculate the changes to each player following the flow auction
	pub fn flow_player_updates(clearing_price: f64, bids: Arc<Book>, asks: Arc<Book>) -> Vec<PlayerUpdate> {
		let mut updates = Vec::<PlayerUpdate>::new();
		let bid_orders = bids.orders.lock().expect("couldn't lock");
		for bid in bid_orders.iter() {
			let v = bid.calc_flow_demand(clearing_price);
			updates.push(PlayerUpdate::new(
					bid.trader_id.clone(),
					format!("N/A"), // No filler id -> assuming trade with ex (update later)
					bid.order_id,
					0,				// No filler order -> assuming trade with ex (update later)
					clearing_price,
					v
				));
		}

		let ask_orders = asks.orders.lock().expect("couldn't lock");
		for ask in ask_orders.iter() {
			let v = ask.calc_flow_supply(clearing_price);
			updates.push(PlayerUpdate::new(
					format!("N/A"), // No filler id -> assuming trade with ex (update later)
					ask.trader_id.clone(),
					0,				// No filler order -> assuming trade with ex (update later)
					ask.order_id,
					clearing_price,
					v
				));
		}
		updates
	}

	pub fn get_price_bounds(bids: Arc<Book>, asks: Arc<Book>) -> (f64, f64) {		
		let bids_min: f64 = bids.get_min_plow();
		let bids_max: f64 = bids.get_max_phigh();
		let asks_min: f64 = asks.get_min_plow();
		let asks_max: f64 = asks.get_max_phigh();

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













