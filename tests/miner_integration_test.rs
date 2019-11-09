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
	better_price_ask.price = 0.0;	//market order

	let mut better_gas_ask = common::setup_ask_limit_order();
	better_gas_ask.gas = 99.0;	// better gas
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
	miner.publish_frame(Arc::clone(&bids_book), Arc::clone(&asks_book), market_type);

	// Only one ask should cross and fill, other will remain
	assert_eq!(asks_book.len(), 1);
	assert_eq!(bids_book.len(), 0);

	let ask = asks_book.pop_from_end().unwrap();
	assert_eq!(ask.price, 0.0);
	assert_eq!(ask.gas, 10.0);
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

	assert!(Auction::equal_e(&results.uniform_price, &81.09048166081236));
}











