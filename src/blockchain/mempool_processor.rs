use crate::order::order::{Order, OrderType, TradeType};
use crate::blockchain::mem_pool::MemPool;
use crate::order::order_book::Book;
use crate::controller::{Task, State};
use crate::exchange::exchange_logic::{Auction, TradeResults};
use crate::exchange::MarketType;	

use std::thread;
use std::thread::JoinHandle;
use std::sync::{Mutex, Arc};

pub struct MemPoolProcessor {}

impl MemPoolProcessor {
	// Concurrently process orders in the pool. Each order is
	// either of OrderType::{Enter, Update, Cancel}. Each order will
	// modify the state of either the Bids or Asks Book, but must
	// first acquire a lock on the respective book. 
	pub fn conc_process_mem_pool(pool: Arc<MemPool>, 
									bids: Arc<Book>, 
									asks: Arc<Book>) 
									-> Vec<JoinHandle<()>>{
		// Acquire lock of MemPool
		// Pop off contents of MemPool
		// match over the OrderType
		// process each order based on OrderType
		
		let mut handles = Vec::<JoinHandle<()>>::new();
		for order in pool.pop_all() {
			let m_t = MarketType::CDA;		// CHANGE LATERRRRRRRRRRR
			let handle = match order.order_type {
				OrderType::Enter => MemPoolProcessor::conc_process_enter(Arc::clone(&bids), Arc::clone(&asks), order, m_t),
				OrderType::Update => MemPoolProcessor::conc_process_update(Arc::clone(&bids), Arc::clone(&asks), order, m_t),
				OrderType::Cancel => MemPoolProcessor::conc_process_cancel(Arc::clone(&bids), Arc::clone(&asks), order, m_t),
			};
			handles.push(handle);
		}
		handles
	}

	// Sequentially process orders in the frame. Each order is
	// either of OrderType::{Enter, Update, Cancel}. Each order will
	// modify the state of either the Bids or Asks Book, but must
	// first acquire a lock on the respective book. 
	pub fn seq_process_orders(frame: &mut Vec<Order>, bids: Arc<Book>, asks: Arc<Book>, m_t: MarketType) -> Option<Vec<TradeResults>> {
		// Create vec to return results of all the crossings
		let mut results: Vec<TradeResults> = Vec::new();
		for order in frame.drain(..) {
			println!("Processing order:{:?}", order);
			match order.order_type {
				OrderType::Enter => {
					if let Some(result) = MemPoolProcessor::seq_process_enter(Arc::clone(&bids), Arc::clone(&asks), order, m_t.clone()) {
						results.push(result);
					}
				}
				OrderType::Update => MemPoolProcessor::seq_process_update(Arc::clone(&bids), Arc::clone(&asks), order, m_t.clone()),
				OrderType::Cancel => MemPoolProcessor::seq_process_cancel(Arc::clone(&bids), Arc::clone(&asks), order, m_t.clone()),
			};
		}
		if results.len() == 0 {
			return None;
		}
		Some(results)
	}


	// Checks if the new order crosses. Modifies orders in book then calculates new max price
	fn seq_process_enter(bids: Arc<Book>, asks: Arc<Book>, order: Order, m_t: MarketType) -> Option<TradeResults> {
		// Spawn a new thread to process the order
    	match m_t {
    		MarketType::FBA|MarketType::KLF => {
				// KLF and FBA are processed the same way by the order book
				match order.trade_type {
					TradeType::Ask => {
						asks.add_order(order).expect("Failed to add order");
						return None;
					},
					TradeType::Bid => {
						bids.add_order(order).expect("Failed to add order...");
						return None
					}
				}
			},
			MarketType::CDA => {
				// Since CDA we will check if the order transacts here:
				match order.trade_type {
					TradeType::Ask => {
						// Only check for cross if this ask price is lower than best ask
						if order.price < asks.get_min_price() {
							// This will add the new ask to the book if it doesn't fully transact
							if let Some(results) = Auction::calc_ask_crossing(bids, asks, order) {
								// We have some trade results return them to apply updates to the clearing house
								return Some(results);
							} 
						} else {
							// We need to add the ask to the book, best price will be updated in add_order
							asks.add_order(order).expect("Failed to add order");
							return None
						}
					},
					TradeType::Bid => {
						// Only check for cross if this bid price is higher than best bid
						if order.price > bids.get_max_price() {
							// This will add the new bid to the book if it doesn't fully transact
							if let Some(results) = Auction::calc_bid_crossing(bids, asks, order) {
								// We have some trade results return them to apply updates to the clearing house
								return Some(results);
							}
						} else {
							// We need to add the ask to the book, best price will be updated in add_order
							bids.add_order(order).expect("Failed to add order...");
							return None;
						}
					}
				}
			}
    	}
    	None
		
	}

	// Cancels the previous order and then enters this as a new one
	// Updates an order in the Bids or Asks Book in it's own thread
	fn seq_process_update(bids: Arc<Book>, asks: Arc<Book>, order: Order, m_t: MarketType) {
		// update books min/max price if this overwrites current min/max OR this order contains new min/max
		match order.trade_type {
			TradeType::Ask => {
				// Cancel the orginal order:
				println!("Cancelling!");
				match asks.cancel_order_by_id(order.order_id) {
					Ok(()) => {},
					Err(e) => println!("{:?}", e),
				}
				// Only check for cross if this ask price is lower than best ask
				if order.price < asks.get_min_price() {
					println!("Gonna auction!");
					// This will add the new ask to the book if it doesn't fully transact
					Auction::calc_ask_crossing(bids, asks, order);
				} else {
					println!("Adding to ask book");
					// We need to add the ask to the book, best price will be updated in add_order
					asks.add_order(order).expect("Failed to add order");
				}
			},
			TradeType::Bid => {
				// Cancel the orginal order:
				println!("Cancelling!");
				match bids.cancel_order_by_id(order.order_id) {
					Ok(()) => {},
					Err(e) => println!("{:?}", e),
				}
				// Only check for cross if this bid price is higher than best bid
				if order.price > bids.get_max_price() {
					println!("Gonna auction!");
					// This will add the new bid to the book if it doesn't fully transact
					Auction::calc_bid_crossing(bids, asks, order);
				} else {
					println!("Adding to ask book");
					// We need to add the ask to the book, best price will be updated in add_order
					bids.add_order(order).expect("Failed to add order...");
				}
			}
		}
	}

	// Cancels the order living in the Bids or Asks Book
	fn seq_process_cancel(bids: Arc<Book>, asks: Arc<Book>, order: Order, m_t: MarketType) {
		let book = match order.trade_type {
			TradeType::Ask => asks,
			TradeType::Bid => bids,
		};

		// If the cancel fails bubble error up.
		match book.cancel_order(order) {
    		Ok(()) => {},
    		Err(e) => {
    			println!("ERROR: {}", e);
    			// TODO send an error response over TCP
    		}
    	}
	}

	// Checks if the new order crosses. Modifies orders in book then calculates new max price
	fn conc_process_enter(bids: Arc<Book>, asks: Arc<Book>, order: Order, m_t: MarketType) -> JoinHandle<()> {
		// Spawn a new thread to process the order
	    thread::spawn(move || {
	    	match m_t {
	    		MarketType::FBA|MarketType::KLF => {
    				// KLF and FBA are processed the same way by the order book
					match order.trade_type {
						TradeType::Ask => {
							asks.add_order(order).expect("Failed to add order");
						},
						TradeType::Bid => {
							bids.add_order(order).expect("Failed to add order...");
						}
					}
    			},
    			MarketType::CDA => {
    				// Since CDA we will check if the order transacts here:
					match order.trade_type {
						TradeType::Ask => {
							// Only check for cross if this ask price is lower than best ask
							if order.price < asks.get_min_price() {
								// This will add the new ask to the book if it doesn't fully transact
								Auction::calc_ask_crossing(bids, asks, order);
							} else {
								// We need to add the ask to the book, best price will be updated in add_order
								asks.add_order(order).expect("Failed to add order");
							}
						},
						TradeType::Bid => {
							// Only check for cross if this bid price is higher than best bid
							if order.price > bids.get_max_price() {
								// This will add the new bid to the book if it doesn't fully transact
								Auction::calc_bid_crossing(bids, asks, order);
							} else {
								// We need to add the ask to the book, best price will be updated in add_order
								bids.add_order(order).expect("Failed to add order...");
							}
						}
					}
    			}
	    	}
			
	    })
	}

	// Cancels the previous order and then enters this as a new one
	// Updates an order in the Bids or Asks Book in it's own thread
	fn conc_process_update(bids: Arc<Book>, asks: Arc<Book>, order: Order, m_t: MarketType) -> JoinHandle<()> {
		// update books min/max price if this overwrites current min/max OR this order contains new min/max
	    thread::spawn(move || {
			match order.trade_type {
				TradeType::Ask => {
					// Cancel the orginal order:
					println!("Cancelling!");
					match asks.cancel_order_by_id(order.order_id) {
						Ok(()) => {},
						Err(e) => println!("{:?}", e),
					}
					// Only check for cross if this ask price is lower than best ask
					if order.price < asks.get_min_price() {
						println!("Gonna auction!");
						// This will add the new ask to the book if it doesn't fully transact
						Auction::calc_ask_crossing(bids, asks, order);
					} else {
						println!("Adding to ask book");
						// We need to add the ask to the book, best price will be updated in add_order
						asks.add_order(order).expect("Failed to add order");
					}
				},
				TradeType::Bid => {
					// Cancel the orginal order:
					println!("Cancelling!");
					match bids.cancel_order_by_id(order.order_id) {
						Ok(()) => {},
						Err(e) => println!("{:?}", e),
					}
					// Only check for cross if this bid price is higher than best bid
					if order.price > bids.get_max_price() {
						println!("Gonna auction!");
						// This will add the new bid to the book if it doesn't fully transact
						Auction::calc_bid_crossing(bids, asks, order);
					} else {
						println!("Adding to ask book");
						// We need to add the ask to the book, best price will be updated in add_order
						bids.add_order(order).expect("Failed to add order...");
					}
				}
			}
	    })
	}

	// Cancels the order living in the Bids or Asks Book
	fn conc_process_cancel(bids: Arc<Book>, asks: Arc<Book>, order: Order, m_t: MarketType) -> JoinHandle<()> {
	    thread::spawn(move || {
			let book = match order.trade_type {
				TradeType::Ask => asks,
				TradeType::Bid => bids,
			};

			// If the cancel fails bubble error up.
			match book.cancel_order(order) {
	    		Ok(()) => {},
	    		Err(e) => {
	    			println!("ERROR: {}", e);
	    			// TODO send an error response over TCP
	    		}
	    	}
	    })
	}

	pub fn async_queue_task(queue: Arc<MemPool>, 
							bids: Arc<Book>, 
							asks: Arc<Book>, 
							state: Arc<Mutex<State>>, 
							duration: u64) -> Task
	{
	    Task::rpt_task(move || {
	    	match *state.lock().expect("Couldn't lock state in queue task") {
				State::Process => {
					let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
								Arc::clone(&bids),
								Arc::clone(&asks));

					for h in handles {
						h.join().expect("Couldn't join queue tasks");
					}
					// println!("Processing order queue");
				},
				State::Auction => println!("Can't process order queue because auction!"),
				State::PreAuction => println!("Can't process order queue because pre-auction!"),
			}
	    }, duration)
	}
}
