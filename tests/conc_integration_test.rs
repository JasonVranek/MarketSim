// extern crate <name_of_my_crate_to_test>
extern crate more_asserts;
use flow_rs::blockchain::mempool_processor::MemPoolProcessor;
use flow_rs::blockchain::order_processor::*;
use flow_rs::order::order::*;
use flow_rs::exchange::exchange_logic::Auction;
use std::sync::Arc;
use rand::{Rng, thread_rng};
use more_asserts::{assert_le};

// Include the common module for setting up state for tests
mod common;

const EPSILON: f64 =  0.000_000_001;	


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


#[test]
fn test_ask_transaction() {
	// Setup queue and order books
	let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut rng = thread_rng();
	let num_bids: usize = rng.gen_range(0, 1000) as usize;
	let (bids, asks) = common::setup_ask_cross_orders(num_bids);
	let mut handles = Vec::new();

	// Send all the bid orders in parallel 
	for bid in bids {
		handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the bid orders in the queue
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}						

	// There should be num_bids bids in the book, with max price num_bids and quantity 5.0
	assert_eq!(bids_book.len(), num_bids);
	let mut b_max_price = bids_book.get_max_price();
	assert_eq!(b_max_price, num_bids as f64);

	let mut handles = Vec::new();
	// Send two asks orders
	for ask in asks {
		handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&queue)));
	}

	for h in handles {
		h.join().unwrap();
	}

	// Process the new ask orders
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	
	for h in handles {
		h.join().unwrap();
	}

	// Only one ask should cross and fill, other will remain
	assert_eq!(asks_book.len(), 1);

	// The filled ask had 10x quantity as the bids so should have filled 10 bids
	assert_eq!(bids_book.len(), num_bids - 10);
	b_max_price = bids_book.get_max_price();

	let a_min_price = asks_book.get_min_price();
	assert_eq!(b_max_price, num_bids as f64 - 10.0);

	// Min price set by remaining ask
	assert_eq!(a_min_price, num_bids as f64 * 1000.0)
}


#[test]
fn test_bid_transaction() {
	// Setup queue and order books
	let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut rng = thread_rng();
	let num_asks = rng.gen_range(0, 1000) as usize;
	let (bids, asks) = common::setup_bid_cross_orders(num_asks);
	let mut handles = Vec::new();

	// Send all the ask orders in parallel 
	for ask in asks {
		handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the bid orders in the queue
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}						

	// There should be num_asks asks in the book, with min price 50 -> 50 + num_asks and quantity 5.0
	assert_eq!(asks_book.len(), num_asks);
	let mut a_min_price = asks_book.get_min_price();
	assert_eq!(a_min_price, 51.0);

	let mut handles = Vec::new();
	// Send two bid orders
	for bid in bids {
		handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&queue)));
	}

	for h in handles {
		h.join().unwrap();
	}

	// Process the new ask orders
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	
	for h in handles {
		h.join().unwrap();
	}

	// Only one bid should cross and fill, other will remain
	assert_eq!(bids_book.len(), 1);

	// The filled bid had 10x quantity as the asks so should have filled 10 asks
	assert_eq!(asks_book.len(), num_asks - 10);
	a_min_price = asks_book.get_min_price();
	assert_eq!(a_min_price, 61.0);

	// Max price set by remaining bid
	let b_max_price = bids_book.get_max_price();
	assert_eq!(b_max_price, 0.0)
}


#[test]
pub fn test_update_bid() {
    let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let (mut bids, asks) = common::setup_orders();
	bids[0].trader_id = format!("jason");
	let mut handles = Vec::new();

	// Send all the orders in parallel 
	for bid in bids {
		handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the orders in the queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}

	assert_eq!(bids_book.len(), 100);

	// Create a new order to update book 
	let mut update_order = common::setup_bid_limit_order();
	update_order.trader_id = format!("jason");
	update_order.order_type = OrderType::Update;
	update_order.price = 99.9;
	update_order.quantity = 555.5;

	// Send new order to queue
	OrderProcessor::conc_recv_order(update_order, Arc::clone(&queue)).join().unwrap();

	// Process queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	for h in handles {
		h.join().unwrap();
	}

	// Books should be same length
	assert_eq!(bids_book.len(), 100);

	// Find the order with id "jason"
	let index = bids_book.peek_id_pos(format!("jason"));

	// Unwrap the index and check order has been updating
	if let Some(i) = index {
		let order = &bids_book.orders.lock().unwrap()[i];
		assert_eq!(order.trader_id, format!("jason"));
		assert_eq!(order.price, 99.9);
		assert_eq!(order.quantity, 555.5);
		assert_eq!(order.order_type, OrderType::Update);
	} else {
		panic!("Update Order should exist");
	}

}

#[test]
pub fn test_update_ask() {
    let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let (bids, mut asks) = common::setup_orders();
	asks[0].trader_id = format!("jason");
	let mut handles = Vec::new();

	// Send all the asks in parallel 
	
	for ask in asks {
		handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the orders in the queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}

	assert_eq!(asks_book.len(), 100);

	// Create a new order to update book 
	let mut update_order = common::setup_ask_limit_order();
	update_order.trader_id = format!("jason");
	update_order.order_type = OrderType::Update;
	update_order.price = 99.9;
	update_order.quantity = 555.5;

	// Send new order to queue
	OrderProcessor::conc_recv_order(update_order, Arc::clone(&queue)).join().unwrap();

	// Process queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	for h in handles {
		h.join().unwrap();
	}

	// Books should be same length
	assert_eq!(asks_book.len(), 100);

	// Find the order with id "jason"
	let index = asks_book.peek_id_pos(format!("jason"));

	// Unwrap the index and check order has been updating
	if let Some(i) = index {
		let order = &asks_book.orders.lock().unwrap()[i];
		assert_eq!(order.trader_id, format!("jason"));
		assert_eq!(order.price, 99.9);
		assert_eq!(order.quantity, 555.5);
		assert_eq!(order.order_type, OrderType::Update);
	} else {
		panic!("Update Order should exist");
	}
}

#[test]
pub fn test_cancel_bid() {
    let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let (mut bids, asks) = common::setup_orders();
	bids[0].trader_id = format!("jason");
	bids[0].price = 99999.9;
	bids[0].quantity = 1.0;
	let mut handles = Vec::new();

	// Send all the orders in parallel 
	for bid in bids {
		handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the orders in the queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}

	assert_eq!(bids_book.len(), 100);

	// New max price will be equal to mutated order 
	assert_eq!(bids_book.get_max_price(), 99999.9);

	// Create a new order to update book 
	let mut update_order = common::setup_bid_limit_order();
	update_order.trader_id = format!("jason");
	update_order.price = 999.9;
	update_order.order_type = OrderType::Cancel;
	update_order.quantity = -1.0; 

	// Send new order to queue
	OrderProcessor::conc_recv_order(update_order, Arc::clone(&queue)).join().unwrap();

	// Process queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	for h in handles {
		h.join().unwrap();
	}

	// Book should shorter by 1
	assert_eq!(bids_book.len(), 99);

	// Find the order with id "jason"
	let index = bids_book.peek_id_pos(format!("jason"));

	// Unwrap the index and check order has been updating
	if let Some(_) = index {
		panic!("Cancel Order should not exist anymore");
	} 

	// The new max price will be updated to something lower once order has been cancelled
	assert_ne!(bids_book.get_max_price(), 99999.9);
	assert_eq!(bids_book.get_max_price(), 100.0)

}

#[test]
pub fn test_cancel_ask() {
    let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let (bids, mut asks) = common::setup_orders();
	asks[0].trader_id = format!("jason");
	asks[0].price = 0.1;		// Set the best ask price
	asks[0].quantity = 10.0;
	let mut handles = Vec::new();

	// Send all the orders in parallel 
	for ask in asks {
		handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the orders in the queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}

	assert_eq!(asks_book.len(), 100);

	// New max price will be equal to mutated order 
	assert_eq!(asks_book.get_min_price(), 0.1);

	// Create a new order to update book 
	let mut update_order = common::setup_ask_limit_order();
	update_order.trader_id = format!("jason");
	update_order.price = 99999.9;
	update_order.order_type = OrderType::Cancel;
	update_order.quantity = 1.0; 

	// Send new order to queue
	OrderProcessor::conc_recv_order(update_order, Arc::clone(&queue)).join().unwrap();

	// Process queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	for h in handles {
		h.join().unwrap();
	}

	// Books should be same length
	assert_eq!(asks_book.len(), 99);

	// Find the order with id "jason"
	let index = asks_book.peek_id_pos(format!("jason"));

	// Unwrap the index and check order has been updating
	if let Some(_) = index {
		panic!("Cancel Order should not exist anymore");
	} 

	// The new max price will be updated to something lower once order has been cancelled
	assert_ne!(asks_book.get_min_price(), 0.1);
	assert_eq!(asks_book.get_min_price(), 2.0);
}


#[test]
pub fn test_update_ask_to_cross() {
    // Setup queue and order books
	let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut rng = thread_rng();
	let num_bids: usize = rng.gen_range(0, 1000) as usize;
	let (bids, mut asks) = common::setup_ask_cross_orders(num_bids);
	let mut handles = Vec::new();

	// Send all the bid orders in parallel 
	for bid in bids {
		handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the bid orders in the queue
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}						

	// There should be num_bids bids in the book, with max price num_bids and quantity 5.0
	assert_eq!(bids_book.len(), num_bids);
	let mut b_max_price = bids_book.get_max_price();
	assert_eq!(b_max_price, num_bids as f64);


	// Setup ask to be updated: 
	asks[0].trader_id = format!("jason");
	asks[0].price = 99999.0;		// Modify from 0.0 -> 99999.0 so won't cross
	asks[0].quantity = 50.0;

	let mut handles = Vec::new();
	// Send two asks orders
	for ask in asks {
		handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&queue)));
	}

	for h in handles {
		h.join().unwrap();
	}

	// Process the new ask orders
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	
	for h in handles {
		h.join().unwrap();
	}

	// No asks should have cross
	assert_eq!(asks_book.len(), 2);

	// Update the order:
	let mut update_order = common::setup_ask_limit_order();
	update_order.trader_id = format!("jason");
	update_order.price = 0.0;	// Will tx as market order
	update_order.order_type = OrderType::Update;
	update_order.quantity = 50.0;	// Should fill 10 bids

	// Send new order to queue
	OrderProcessor::conc_recv_order(update_order, Arc::clone(&queue)).join().unwrap();

	// Process queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	for h in handles {
		h.join().unwrap();
	}

	// The filled ask had 10x quantity as the bids so should have filled 10 bids
	assert_eq!(bids_book.len(), num_bids - 10);
	b_max_price = bids_book.get_max_price();

	let a_min_price = asks_book.get_min_price();
	assert_eq!(b_max_price, num_bids as f64 - 10.0);

	// Min price set by remaining ask
	assert_eq!(a_min_price, num_bids as f64 * 1000.0);
	assert_eq!(asks_book.len(), 1);
}



#[test]
pub fn test_update_bid_to_cross() {
    // Setup queue and order books
	let queue = Arc::new(common::setup_mem_pool());
	let bids_book = Arc::new(common::setup_bids_book());
	let asks_book = Arc::new(common::setup_asks_book());
	
	// Setup bids and asks
	let mut rng = thread_rng();
	let num_asks: usize = rng.gen_range(0, 1000) as usize;
	let (mut bids, asks) = common::setup_bid_cross_orders(num_asks);
	let mut handles = Vec::new();

	// Send all the ask orders in parallel 
	for ask in asks {
		handles.push(OrderProcessor::conc_recv_order(ask, Arc::clone(&queue)));
	}

	// Wait for the threads to finish
	for h in handles {
		h.join().unwrap();
	}

	// Process all of the ask orders in the queue
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));

	for h in handles {
		h.join().unwrap();
	}						

	// There should be num_asks bids in the book, with max price num_asks and quantity 5.0
	assert_eq!(asks_book.len(), num_asks);
	let mut a_min_price = asks_book.get_min_price();
	assert_eq!(a_min_price, 51.0);


	// Setup bid to be updated: 
	bids[0].trader_id = format!("jason");
	bids[0].price = 0.0;		// Modify from 99999.0 -> 0.0 so won't cross
	bids[0].quantity = 50.0;

	let mut handles = Vec::new();
	// Send two bid orders
	for bid in bids {
		handles.push(OrderProcessor::conc_recv_order(bid, Arc::clone(&queue)));
	}

	for h in handles {
		h.join().unwrap();
	}

	// Process the new ask orders
	let mut handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	
	for h in handles {
		h.join().unwrap();
	}

	// No bids should have cross
	assert_eq!(bids_book.len(), 2);

	// Update the order:
	let mut update_order = common::setup_bid_limit_order();
	update_order.trader_id = format!("jason");
	update_order.price = 999999.0;	// Will tx as market order
	update_order.order_type = OrderType::Update;
	update_order.quantity = 50.0;	// Should fill 10 asks

	// Send new order to queue
	OrderProcessor::conc_recv_order(update_order, Arc::clone(&queue)).join().unwrap();

	// Process queue
	let handles = MemPoolProcessor::conc_process_mem_pool(Arc::clone(&queue), 
							Arc::clone(&bids_book),
							Arc::clone(&asks_book));
	for h in handles {
		h.join().unwrap();
	}

	// The filled bid had 10x quantity as the asks so should have filled 10 asks
	assert_eq!(asks_book.len(), num_asks - 10);
	let b_max_price = bids_book.get_max_price();

	// Min price set by remaining bid
	assert_eq!(b_max_price, 0.0);
	assert_eq!(bids_book.len(), 1);
}
















