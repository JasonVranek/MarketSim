// extern crate <name_of_my_crate_to_test>
extern crate more_asserts;
use flow_rs::blockchain::mempool_processor::MemPoolProcessor;
use flow_rs::blockchain::order_processor::*;
use flow_rs::order::order::*;
use flow_rs::exchange::exchange_logic::Auction;
use flow_rs::players::miner::Miner;
use flow_rs::exchange::MarketType;
use std::sync::Arc;
use rand::{Rng, thread_rng};
use more_asserts::{assert_le};

// Include the common module for setting up state for tests
mod common;

const EPSILON: f64 =  0.000_000_001;	
const BLOCK_SIZE: usize = 99999;


#[test]
fn default_test() {
	common::setup();
	assert_eq!(1, 1);
}

#[test]
fn test_add_order_to_book() {
	let bid = common::setup_bid_limit_order();

	let book = common::setup_bids_book();

	book.add_order(bid);

	assert_eq!(book.len(), 1);

	let order = book.orders.lock().unwrap().pop().unwrap();

}


#[test]
fn test_conc_queue_recv_order() {
	// Setup a queue
	let queue = Arc::new(common::setup_mem_pool());

	let mut order = common::setup_bid_limit_order();

	// Mutate order
	order.price = 199.0;

	// Accept order in a new thread
	let handle = OrderProcessor::conc_recv_order(order, Arc::clone(&queue));

	// Wait for thread to finish
	handle.join().unwrap();

	// Confirm the queue's order is correct
	let order = queue.pop().unwrap();

	assert_eq!(order.price, 199.0);
}

#[test]
fn test_mem_pool_pop_all() {
	let pool = common::setup_full_mem_pool();
	let popped_off = pool.pop_all();
	assert_eq!(popped_off.len(), 3);
}

#[test]
fn test_mem_pool_pop_n() {
	let n = 100;
	let pool = common::setup_n_full_mem_pool(n);
	assert_eq!(pool.length(), n);
	let popped_off = pool.pop_n(n/2);
	assert_eq!(popped_off.len(), n/2);
}

#[test]
fn test_mem_pool_sort_gas() {
	let n = 100;
	let pool = common::setup_n_full_mem_pool(n);
	pool.sort_by_gas();
	assert_eq!(pool.length(), n);
	while pool.length() >= 1 {
		// Pop from end of queue
		let item1 = pool.pop().unwrap();	//last in the queue
		let item2 = pool.pop().unwrap(); 	//2nd to last in the queue
		let diff = item2.gas - item1.gas;
		println!("item1:{}, item2:{}, item2-item1={}", item1.gas, item2.gas, diff);
		assert_le!(EPSILON, diff);
	}
}

// Tests that gas priority is correct and correct ask crosses with best bid
#[test]
fn test_cda_ask_transaction() {
	// Setup pool and order books
	let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());

	let mut miner = common::setup_miner();
	let market_type = MarketType::CDA;
	
	// Setup 1 bid and 2 asks
	let mut bid = common::setup_bid_limit_order();
	bid.gas = 99999.9;				// Make sure there is an order in book
	bid.price = 100.0;
	
	let mut better_price_ask = common::setup_ask_limit_order();
	better_price_ask.gas = 10.0;	// worse gas
	better_price_ask.price = 0.0;	// market order

	let mut better_gas_ask = common::setup_ask_limit_order();
	better_gas_ask.gas = 99.0;		// better gas
	better_gas_ask.price = 99.0;	// worse price


	let mut handles = Vec::new();
	// Send all the orders in parallel to mempool
	handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(better_price_ask, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(better_gas_ask, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles.drain(..) {
		h.join().unwrap();
	}

	// Create frame from the orders in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Assert that orders in frame are sorted in decreasing order by gas
	let mut last_gas = 999999999.0;
	for order in &miner.frame {
		println!("price: {}, gas: {}, type: {:?}", order.price, order.gas, order.trade_type);
		assert_le!(order.gas, last_gas);
		last_gas = order.gas;
	}

	miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type);

	// Only one ask should cross and fill, other will remain
	assert_eq!(asks_book.len(), 1);
	assert_eq!(bids_book.len(), 0);

	let ask = asks_book.pop_from_end().unwrap();
	assert_eq!(ask.gas, 10.0);
	assert_eq!(ask.price, 0.0);
}


// Tests that gas priority is correct and correct bid crosses with best ask
#[test]
fn test_cda_bid_transaction() {
	// Setup pool and order books
	let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());

	let mut miner = common::setup_miner();
	let market_type = MarketType::CDA;
	
	// Setup 1 ask and 2 bids
	let mut ask = common::setup_ask_limit_order();
	ask.gas = 99999.9;				// Make sure ask enters book first
	ask.price = 100.0;
	
	let mut better_price_bid = common::setup_bid_limit_order();
	better_price_bid.gas = 10.0;	// worse gas
	better_price_bid.price = 99999.0;	//market order

	let mut better_gas_bid = common::setup_bid_limit_order();
	better_gas_bid.gas = 99.0;	// better gas
	better_gas_bid.price = 101.0;	// worse price


	let mut handles = Vec::new();
	// Send all the bid orders in parallel to mempool
	handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(better_price_bid, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(better_gas_bid, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles.drain(..) {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Assert that orders in frame are sorted in decreasing order by gas
	let mut last_gas = 999999999.0;
	for order in &miner.frame {
		println!("price: {}, gas: {}, type: {:?}", order.price, order.gas, order.trade_type);
		assert_le!(order.gas, last_gas);
		last_gas = order.gas;
	}

	// Process the bid order
	miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type);

	// Only one bid should cross and fill, other will remain
	assert_eq!(asks_book.len(), 0);
	assert_eq!(bids_book.len(), 1);

	let bid = bids_book.pop_from_end().unwrap();
	assert_eq!(bid.price, 99999.0);
	assert_eq!(bid.gas, 10.0);
}


#[test]
pub fn test_klf_crossing_price() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let (bids, asks) = common::setup_flow_orders();
	let mut handles = Vec::new();

	let mut miner = common::setup_miner();
	let market_type = MarketType::KLF;

	// Send all the orders in parallel 
	for bid in bids {
		handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&pool)));
	}
	for ask in asks {
		handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&pool)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the bid order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).unwrap();

	assert_eq!(bids_book.len(), 100);
	assert_eq!(asks_book.len(), 100);

	assert!(Auction::equal_e(&results.uniform_price.unwrap(), &81.09048166081236));
}


#[test]
pub fn test_fba_uniform_price1() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 50.0;
	ask1.price = 11.30;
	let ask1_id = ask1.order_id;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 12.50;

	let mut bid1 = common::setup_bid_limit_order();
	bid1.quantity = 44.0;
	bid1.price = 12.0;
	let bid1_id = bid1.order_id;

	let mut bid2 = common::setup_bid_limit_order();
	bid2.quantity = 23.0;
	bid2.price = 11.20;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(bid1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(bid2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).unwrap();

	// The bid1's volume was filled so it should have been removed from the book
	assert_eq!(bids_book.len(), 1);

	// Ask1 should have 50-44 = 6 remaining quanitity in order
	assert_eq!(asks_book.len(), 2);

	assert!(Auction::equal_e(&results.uniform_price.unwrap(), &11.30));

	println!("{:?}", results);

	if let Some(player_updates) = results.cross_results {
		// Should have received updates
		assert_ne!(0, player_updates.len());
		for pu in player_updates {
			assert_eq!(pu.payer_order_id, bid1_id);
			assert_eq!(pu.vol_filler_order_id, ask1_id);
			assert_eq!(pu.volume, 44.0);
			assert_eq!(pu.price, 11.30);
		}
	}
	
	println!("bids: {:?}, asks: {:?}", bids_book, asks_book);

	

}


#[test]
pub fn test_fba_uniform_price2() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 6.0;
	ask1.price = 11.30;
	let ask1_id = ask1.order_id;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 12.50;
	let ask2_id = ask2.order_id;

	let mut bid1 = common::setup_bid_limit_order();
	bid1.quantity = 10.0;
	bid1.price = 15.0;
	let bid1_id = bid1.order_id;

	let mut bid2 = common::setup_bid_limit_order();
	bid2.quantity = 23.0;
	bid2.price = 11.20;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(bid1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(bid2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).unwrap();

	// The bid that was filled is removed
	assert_eq!(bids_book.len(), 1);
	// The ask that was completely filled will be removed
	assert_eq!(asks_book.len(), 1);

	assert!(Auction::equal_e(&results.uniform_price.unwrap(), &12.50));

	if let Some(player_updates) = results.cross_results {
		// player_updates[0] -> bid1 + ask1
		assert_eq!(player_updates[0].payer_order_id, bid1_id);
		assert_eq!(player_updates[0].vol_filler_order_id, ask1_id);
		assert_eq!(player_updates[0].volume, 6.0);
		assert_eq!(player_updates[0].price, 12.50);

		// player_updates[1] -> bid1 + ask2
		assert_eq!(player_updates[1].payer_order_id, bid1_id);
		assert_eq!(player_updates[1].vol_filler_order_id, ask2_id);
		assert_eq!(player_updates[1].volume, 4.0);
		assert_eq!(player_updates[1].price, 12.50);

	}
}


#[test]
pub fn test_fba_uniform_price3() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 10.0;
	ask1.price = 11.20;
	let ask1_id = ask1.order_id;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 11.60;
	let ask2_id = ask2.order_id;

	let mut ask3 = common::setup_ask_limit_order();
	ask3.quantity = 22.0;
	ask3.price = 12.30;

	let mut ask4 = common::setup_ask_limit_order();
	ask4.quantity = 30.0;
	ask4.price = 12.50;	

	let mut bid1 = common::setup_bid_limit_order();
	bid1.quantity = 80.0;
	bid1.price = 12.0;
	let bid1_id = bid1.order_id;

	let mut bid2 = common::setup_bid_limit_order();
	bid2.quantity = 40.0;
	bid2.price = 11.0;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(bid1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(bid2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask3, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask4, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).unwrap();

	assert_eq!(bids_book.len(), 2);
	// Both asks that were completely filled will be removed
	assert_eq!(asks_book.len(), 2);

	println!("{:?}", results);
	assert!(Auction::equal_e(&results.uniform_price.expect("no price!!"), &12.0));

	assert_eq!(results.agg_supply, 60.0);

	if let Some(player_updates) = results.cross_results {
		// player_updates[0] -> bid1 + ask1
		assert_eq!(player_updates[0].payer_order_id, bid1_id);
		assert_eq!(player_updates[0].vol_filler_order_id, ask1_id);
		assert_eq!(player_updates[0].volume, 10.0);
		assert_eq!(player_updates[0].price, 12.0);

		// player_updates[1] -> bid1 + ask2
		assert_eq!(player_updates[1].payer_order_id, bid1_id);
		assert_eq!(player_updates[1].vol_filler_order_id, ask2_id);
		assert_eq!(player_updates[1].volume, 50.0);
		assert_eq!(player_updates[1].price, 12.0);
	}
}


#[test]
pub fn test_fba_no_cross() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 6.0;
	ask1.price = 11.30;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 12.50;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).expect("errorrrr");

	assert_eq!(bids_book.len(), 0);
	assert_eq!(asks_book.len(), 2);

	println!("{:?}", results);
	assert!(&results.uniform_price.is_none());
}


#[test]
pub fn test_fba_vertical_cross() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 50.0;
	ask1.price = 11.30;
	let ask1_id = ask1.order_id;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 12.50;

	let mut bid1 = common::setup_bid_limit_order();
	bid1.quantity = 44.0;
	bid1.price = 11.30;
	let bid1_id = bid1.order_id;

	let mut bid2 = common::setup_bid_limit_order();
	bid2.quantity = 23.0;
	bid2.price = 11.20;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(bid1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(bid2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).expect("errorrrr");

	assert_eq!(bids_book.len(), 1);
	assert_eq!(asks_book.len(), 2);

	println!("{:?}", results);
	assert!(Auction::equal_e(&results.uniform_price.unwrap(), &11.30));

	if let Some(player_updates) = results.cross_results {
		for pu in player_updates {
			assert_eq!(pu.payer_order_id, bid1_id);
			assert_eq!(pu.vol_filler_order_id, ask1_id);
			assert_eq!(pu.volume, 44.0);
			assert_eq!(pu.price, 11.30);
		}
	}
}

#[test]
pub fn test_fba_vertical_cross2() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 10.0;
	ask1.price = 11.20;
	let ask1_id = ask1.order_id;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 11.60;
	let ask2_id = ask2.order_id;

	let mut ask3 = common::setup_ask_limit_order();
	ask3.quantity = 22.0;
	ask3.price = 12.30;
	let ask3_id = ask3.order_id;

	let mut ask4 = common::setup_ask_limit_order();
	ask4.quantity = 30.0;
	ask4.price = 12.50;	

	let mut bid1 = common::setup_bid_limit_order();
	bid1.quantity = 61.0;
	bid1.price = 12.3;
	let bid1_id = bid1.order_id;

	let mut bid2 = common::setup_bid_limit_order();
	bid2.quantity = 40.0;
	bid2.price = 11.0;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(bid1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(bid2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask3, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask4, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).unwrap();

	assert_eq!(bids_book.len(), 1);
	// Both asks that were completely filled will be removed
	assert_eq!(asks_book.len(), 2);

	println!("{:?}", results);
	assert!(Auction::equal_e(&results.uniform_price.expect("no price!!"), &12.3));

	assert_eq!(results.agg_supply, 61.0);

	if let Some(player_updates) = results.cross_results {
		// player_updates[0] -> bid1 + ask1
		assert_eq!(player_updates[0].payer_order_id, bid1_id);
		assert_eq!(player_updates[0].vol_filler_order_id, ask1_id);
		assert_eq!(player_updates[0].volume, 10.0);
		assert_eq!(player_updates[0].price, 12.3);

		assert_eq!(player_updates[1].payer_order_id, bid1_id);
		assert_eq!(player_updates[1].vol_filler_order_id, ask2_id);
		assert_eq!(player_updates[1].volume, 50.0);
		assert_eq!(player_updates[1].price, 12.3);

		assert_eq!(player_updates[2].payer_order_id, bid1_id);
		assert_eq!(player_updates[2].vol_filler_order_id, ask3_id);
		assert_eq!(player_updates[2].volume, 1.0);
		assert_eq!(player_updates[2].price, 12.3);

	}
}


// #[test]		// Need to confirm what the price is for this...
pub fn test_fba_horizontal_cross() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 50.0;
	ask1.price = 11.30;
	let ask1_id = ask1.order_id;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 12.50;

	let mut bid1 = common::setup_bid_limit_order();
	bid1.quantity = 50.0;
	bid1.price = 12.0;
	let bid1_id = bid1.order_id;

	let mut bid2 = common::setup_bid_limit_order();
	bid2.quantity = 23.0;
	bid2.price = 11.20;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(bid1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(bid2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).expect("errorrrr");

	assert_eq!(bids_book.len(), 1);
	assert_eq!(asks_book.len(), 1);

	println!("{:?}", results);
	assert!(Auction::equal_e(&results.uniform_price.unwrap(), &12.25));

	if let Some(player_updates) = results.cross_results {
		assert_eq!(player_updates[0].payer_order_id, bid1_id);
		assert_eq!(player_updates[0].vol_filler_order_id, ask1_id);
		assert_eq!(player_updates[0].volume, 50.0);
		assert!(Auction::equal_e(&player_updates[0].price, &12.25));
	}
}


#[test]
pub fn test_fba_horizontal_cross2() {
    let pool = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut ask1 = common::setup_ask_limit_order();
	ask1.quantity = 10.0;
	ask1.price = 11.20;
	let ask1_id = ask1.order_id;

	let mut ask2 = common::setup_ask_limit_order();
	ask2.quantity = 50.0;
	ask2.price = 11.60;
	let ask2_id = ask2.order_id;

	let mut ask3 = common::setup_ask_limit_order();
	ask3.quantity = 22.0;
	ask3.price = 12.30;

	let mut ask4 = common::setup_ask_limit_order();
	ask4.quantity = 30.0;
	ask4.price = 12.50;	

	let mut bid1 = common::setup_bid_limit_order();
	bid1.quantity = 60.0;
	bid1.price = 12.4;
	let bid1_id = bid1.order_id;

	let mut bid2 = common::setup_bid_limit_order();
	bid2.quantity = 40.0;
	bid2.price = 11.0;

	// Setup Miner
	let mut handles = Vec::new();
	let mut miner = common::setup_miner();
	let market_type = MarketType::FBA;

	// Send all the orders in parallel 
	handles.push(OrderProcessor::conc_recv_order(bid1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(bid2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask1, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask2, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask3, Arc::clone(&pool)));
	handles.push(OrderProcessor::conc_recv_order(ask4, Arc::clone(&pool)));

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Create frame from bid order in mempool
	miner.make_frame(Arc::clone(&pool), BLOCK_SIZE);

	// Process the orders order
	let results = miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type).unwrap();

	assert_eq!(bids_book.len(), 1);
	// Both asks that were completely filled will be removed
	assert_eq!(asks_book.len(), 2);

	println!("{:?}", results);
	assert!(Auction::equal_e(&results.uniform_price.expect("no price!!"), &12.35));

	assert_eq!(results.agg_supply, 60.0);

	if let Some(player_updates) = results.cross_results {
		// player_updates[0] -> bid1 + ask1
		assert_eq!(player_updates[0].payer_order_id, bid1_id);
		assert_eq!(player_updates[0].vol_filler_order_id, ask1_id);
		assert_eq!(player_updates[0].volume, 10.0);
		assert!(Auction::equal_e(&player_updates[0].price, &12.35));

		assert_eq!(player_updates[1].payer_order_id, bid1_id);
		assert_eq!(player_updates[1].vol_filler_order_id, ask2_id);
		assert_eq!(player_updates[1].volume, 50.0);
		assert!(Auction::equal_e(&player_updates[1].price, &12.35));

	}
}