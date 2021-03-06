use crate::controller::{Task, State};
use crate::order::order_book::Book;
use crate::order::order::{Order};
use crate::exchange::MarketType;
use crate::utility::get_time;
use crate::log_order_book;

use std::sync::{Mutex, Arc};
use std::cmp::Ordering;

use rayon::prelude::*;
use log::{log, Level};



const EPSILON: f64 =  0.000_001;
const MAX_PRICE: f64 = 999_999_999.0;
const MIN_PRICE: f64 = 0.0;
const MAX_ITERS: usize = 1000;
// const PRECISION: i8 = 4;

#[derive(Debug, Clone)]
pub struct PlayerUpdate {
	pub payer_id: String,
	pub vol_filler_id: String,
	pub payer_order_id: u64,
	pub vol_filler_order_id: u64,
	pub price: f64,
	pub volume: f64,
	pub cancel: bool,
}

impl PlayerUpdate {
	pub fn new(payer_id: String, vol_filler_id: String, payer_order_id: u64, 
		vol_filler_order_id: u64, price: f64, volume: f64, cancel: bool) -> PlayerUpdate {
		PlayerUpdate {
			payer_id,
			vol_filler_id,
			payer_order_id,
			vol_filler_order_id,
			price,
			volume,
			cancel,
		}
	}
}

#[derive(Debug, Clone)]
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
	pub fn calc_bid_crossing(bids: Arc<Book>, asks:Arc<Book>, mut new_bid: Order) -> Option<TradeResults> {
		let mut results = TradeResults::new(MarketType::CDA, None, 0.0, 0.0, None);
		let mut updates = Vec::<PlayerUpdate>::new();
		loop {
			if new_bid.price >= asks.get_min_price() {
				// buying for more than best ask is asking for -> tx @ ask price
				// Get the best ask from book if there is one, else nothing to cross so add bid to book
				let mut best_ask = match asks.pop_from_end() {
					Some(order) => order,
					None => {
						bids.add_order(new_bid).expect("Failed to add bid to book...");
						bids.find_new_max();
						results.cross_results = Some(updates);
						return Some(results);
					}
				};
				// Modify quantities of best ask and new bid
				match new_bid.quantity.partial_cmp(&best_ask.quantity).expect("bad cmp") {
					Ordering::Less => {
						// This new bid will be satisfied and not be added to the book
						best_ask.quantity -= new_bid.quantity;
						trace!("New bid:{} transacted {} shares with best ask:{} @{}", 
								new_bid.trader_id, new_bid.quantity, best_ask.trader_id, best_ask.price);

						// Update player results to modify ExchangeHouse
						updates.push(PlayerUpdate::new(
							new_bid.trader_id.clone(),
							best_ask.trader_id.clone(),
							new_bid.order_id,
							best_ask.order_id,
							best_ask.price,
							new_bid.quantity,
							false
							));

						// Return the best ask to the book
						asks.push_to_end(best_ask).expect("couldn't push");

						// This bid is done crossing, exit loop
						break;
					},
					Ordering::Greater => {
						// This new bid potentially will cross with multiple asks
						new_bid.quantity -= best_ask.quantity;
						info!("New bid:{} transacted {} shares with best ask:{} @{}, clearing best ask from book", 
								new_bid.trader_id, best_ask.quantity, best_ask.trader_id, best_ask.price);

						// Update player results to modify ExchangeHouse
						updates.push(PlayerUpdate::new(
							new_bid.trader_id.clone(),
							best_ask.trader_id.clone(),
							new_bid.order_id,
							best_ask.order_id,
							best_ask.price,
							best_ask.quantity,
							false
							));
						
						// Update the best ask price 
						asks.find_new_min();
						// Don't return the bid to the book, instead restart loop to see if bid crosses anymore
						continue;
					},
					Ordering::Equal => {
						// new bid clears the best ask removing it from book
						info!("New bid:{} transacted {} shares with best ask:{} @{}, clearing best ask from book", 
								new_bid.trader_id, new_bid.quantity, best_ask.trader_id, best_ask.price);

						updates.push(PlayerUpdate::new(
							new_bid.trader_id.clone(),
							best_ask.trader_id.clone(),
							new_bid.order_id,
							best_ask.order_id,
							best_ask.price,
							new_bid.quantity,
							false
							));

						// Update the best ask price 
						asks.find_new_min();
						// Don't return the bid to the book
						break;
					}
				}  
			} else {
				// New bid didn't cross, needs to be added to the book then exit
				bids.add_order(new_bid.clone()).expect("Failed to add bid to book...");
				bids.find_new_max();
				// log_order_book!(format!("{},{:?},{:?},",Order::order_to_csv(&new_bid),bids.orders,asks.orders));
				results.cross_results = Some(updates);
				return Some(results);
			}
		}
		// Done with loop, return the results
		log_order_book!(format!("{},{:?},{:?},",Order::order_to_csv(&new_bid),bids.orders,asks.orders));
		results.cross_results = Some(updates);
		return Some(results);
	}


	/// ***CDA function***
	/// Checks whether the new ask crosses the best bid. 
	/// A new ask will cross at best bid.price iff best bid.price ≥ new ask.price
	/// If the new order's quantity is not satisfied, the next best bid is checked.
	pub fn calc_ask_crossing(bids: Arc<Book>, asks:Arc<Book>, mut new_ask: Order)  -> Option<TradeResults> {
		let mut results = TradeResults::new(MarketType::CDA, None, 0.0, 0.0, None);
		let mut updates = Vec::<PlayerUpdate>::new();
		loop {
			if new_ask.price <= bids.get_max_price() {
				// asking for less than best bid willing to pay -> tx @ bid price
				// Modify quantities of best bid and this new ask
				let mut best_bid = match bids.pop_from_end() {
					Some(order) => order,
					None => {
						// There were no bids in the book, simply add this order to asks book
						asks.add_order(new_ask).expect("Failed to add ask to book...");
						asks.find_new_min();
						results.cross_results = Some(updates);
						return Some(results);
					}
				};
				match new_ask.quantity.partial_cmp(&best_bid.quantity).expect("bad cmp") {
					Ordering::Less => {
						// This new ask will be satisfied and not be added to the book
						best_bid.quantity -= new_ask.quantity;
						println!("New ask:{} transacted {} shares with best bid:{} @{}", 
								new_ask.trader_id, new_ask.quantity, best_bid.trader_id, best_bid.price);

						// Update player results to modify ExchangeHouse
						updates.push(PlayerUpdate::new(
							best_bid.trader_id.clone(),
							new_ask.trader_id.clone(),
							best_bid.order_id,
							new_ask.order_id,
							best_bid.price,
							new_ask.quantity,
							false
							));

						// Return the best bid to the book
						bids.push_to_end(best_bid).expect("bad push");

						// This ask is done crossing, exit loop
						break;
					},
					Ordering::Greater => {
						// This new ask potentially will cross with multiple bids
						new_ask.quantity -= best_bid.quantity;
						println!("New ask:{} transacted {} shares with best bid:{} @{}, clearing best bid from book", 
								new_ask.trader_id, best_bid.quantity, best_bid.trader_id, best_bid.price);

						// Update player results to modify ExchangeHouse
						updates.push(PlayerUpdate::new(
							best_bid.trader_id.clone(),
							new_ask.trader_id.clone(),
							best_bid.order_id,
							new_ask.order_id,
							best_bid.price,
							best_bid.quantity,
							false
							));
						
						// Update the best bid price 
						bids.find_new_max();
						// Don't return the bid to the book, instead restart loop to see if ask crosses anymore
						continue;
					},
					Ordering::Equal => {
						// new ask clears the best bid removing it from book
						println!("New ask:{} transacted {} shares with best bid:{} @{}, clearing best bid from book", 
								new_ask.trader_id, new_ask.quantity, best_bid.trader_id, best_bid.price);

						updates.push(PlayerUpdate::new(
							best_bid.trader_id.clone(),
							new_ask.trader_id.clone(),
							best_bid.order_id,
							new_ask.order_id,
							best_bid.price,
							new_ask.quantity,
							false,
							));
						
						// Update the best bid price 
						bids.find_new_max();
						// Don't return the ask to the book
						break;
					}
				}  
			} else {
				// New ask didn't cross, needs to be added to the book
				asks.add_order(new_ask.clone()).expect("Failed to add ask to book...");
				asks.find_new_min();
				// log_order_book!(format!("{},{:?},{:?},", Order::order_to_csv(&new_ask),bids.orders,asks.orders));

				results.cross_results = Some(updates);
				return Some(results);
			}
		}
		// Done with loop, return the results
		log_order_book!(format!("{},{:?},{:?},", Order::order_to_csv(&new_ask),bids.orders,asks.orders));
		results.cross_results = Some(updates);
		return Some(results);
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
		// let mut prev_seen_vol = 0.0;
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
			// prev_seen_vol = seen_vol;
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
											  cp, trade_amount, false));
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
											  cp, trade_amount, false));
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
											  cp, trade_amount,false));

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


	/// Helper function for Flow Order clearing price calculation: bs_cross
	/// Iterate over each order in parallel and compute the aggregate supply and
	/// demand at a certain price.
	pub fn calc_aggs(p: f64, bids: Arc<Book>, asks: Arc<Book>) -> (f64, f64) {
		let bids = bids.orders.lock().expect("ERROR: No bids book");
		let asks = asks.orders.lock().expect("ERROR: No asks book");

		// Calculate cummulative demand schedule trade volume
		let agg_demand: f64 = bids.par_iter()
		    .map(|order| {
	    		order.calc_flow_demand(p)
		    }).sum();


		// Calculate cummulative supply schedule trade volume
		let agg_supply: f64 = asks.par_iter()
		    .map(|order| {
	    		order.calc_flow_supply(p)
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
	    		println!("Found cross at: {}\n", index);
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

	pub fn klf_clearing(bids: Arc<Book>, asks: Arc<Book>) -> Option<f64> {
		let best_bid_p = bids.get_max_phigh();
		let best_ask_p = asks.get_min_plow();

		// Demand and supply at the best bid price
		let (dem_bb, sup_bb) = Auction::calc_aggs(best_bid_p, Arc::clone(&bids), Arc::clone(&asks));

		// Demand and supply at the best ask price
		let (dem_ba, sup_ba) = Auction::calc_aggs(best_ask_p, Arc::clone(&bids), Arc::clone(&asks));

		let order_imbalance = (dem_bb - sup_bb) / (dem_bb - sup_bb + sup_ba - dem_ba);

		let clearing_price = best_bid_p + order_imbalance * (best_ask_p - best_bid_p);

		println!("Clearing Price new way: {:?}, w:{}, pb:{}, pa:{}", clearing_price, order_imbalance, best_bid_p, best_ask_p);

		Some(clearing_price)
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
		let mut cancel_bids = Vec::<u64>::new();
		let mut cancel_asks = Vec::<u64>::new();
		{
			let mut bid_orders = bids.orders.lock().expect("couldn't lock");
			for bid in bid_orders.iter_mut() {
				let v = bid.calc_flow_demand(clearing_price);
				// Generate the PlayerUpdate for the ClearingHouse to update the player if they transact at clearing price
				if v > 0.0 {
					updates.push(PlayerUpdate::new(
							bid.trader_id.clone(),
							format!("N/A"), // No filler id -> assuming trade with ex (update later)
							bid.order_id,
							0,				// No filler order -> assuming trade with ex (update later)
							clearing_price,
							v,
							false
						));
					// Modify the order in the order book
					bid.quantity -= v;
					// println!("bid:{}, p_l: {}, p_h:{}, trade_vol:{}, old_vol:{}, new_vol:{}", bid.order_id, bid.p_low, bid.p_high, v, bid.quantity + v, bid.quantity);
					if bid.quantity <= 0.0 {
						// println!("cancelling flow bid");
						cancel_bids.push(bid.order_id);
					}
				}
			}
		}
		{
			let mut ask_orders = asks.orders.lock().expect("couldn't lock");
			for ask in ask_orders.iter_mut() {
				let v = ask.calc_flow_supply(clearing_price);
				// Generate the PlayerUpdate for the ClearingHouse to update the player if they transact at clearing price
				if v > 0.0 {
					updates.push(PlayerUpdate::new(
							format!("N/A"), // No filler id -> assuming trade with ex (update later)
							ask.trader_id.clone(),
							0,				// No filler order -> assuming trade with ex (update later)
							ask.order_id,
							clearing_price,
							v,
							false
						));
					// Modify the order in the order book
					ask.quantity -= v;
					// println!("ask:{}, p_l: {}, p_h:{}, trade_vol:{}, old_vol:{}, new_vol:{}", ask.order_id, ask.p_low, ask.p_high, v, ask.quantity + v, ask.quantity);
					if ask.quantity <= 0.0 {
						// println!("cancelling flow ask");
						cancel_asks.push(ask.order_id);
					}
				}
			}
		}

		// println!("cancelling bids:{:?} and asks:{:?}", cancel_bids, cancel_asks);

		// Cancel all of the orders that have been fully filled
		for id in cancel_bids {
			bids.cancel_order_by_id(id).expect("Error cancelling filled flow order");
		}
		for id in cancel_asks {
			asks.cancel_order_by_id(id).expect("Error cancelling filled flow order");
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













