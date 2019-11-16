extern crate flow_rs;
extern crate more_asserts;
use flow_rs::blockchain::order_processor::OrderProcessor;
use flow_rs::simulation::trader_behavior::*;
use flow_rs::blockchain::mem_pool::*;
use flow_rs::order::order::*;
use flow_rs::order::order_book::*;
use flow_rs::utility::{gen_rand_f64, gen_rand_trader_id};
use flow_rs::players::miner::Miner;
use flow_rs::players::investor::Investor;
use flow_rs::players::maker::Maker;
use flow_rs::players::Player;
use std::sync::Arc;
use rand::Rng;

pub fn setup() {
	// setup code specific to lib's tests go here
	// this code can then be accessed from other tests via
	// common::setup()
}

pub fn setup_miner() -> Miner {
	Miner::new(format!("{:?}", "asfasdf"))
}

pub fn setup_bid_limit_order() -> Order {
	Order::new(
		String::from("bid_id"),
		OrderType::Enter,
		TradeType::Bid,
		ExchangeType::LimitOrder,
		0.0,	// p_low
		0.0,	// p_high
		100.0,	// price
		5.0,	// quantity
		0.1,	// gas
	)
}

pub fn setup_ask_limit_order() -> Order {
	Order::new(
		String::from("ask_id"),
		OrderType::Enter,
		TradeType::Ask,
		ExchangeType::LimitOrder,
		0.0,	// p_low
		0.0,	// p_high
		100.0,	// price
		5.0,	// quantity
		0.1,	// gas
	)
}

pub fn setup_rand_bid_limit_order() -> Order {
	Order::new(
		gen_rand_trader_id(),
		OrderType::Enter,
		TradeType::Bid,
		ExchangeType::LimitOrder,
		gen_rand_f64(),	// p_low
		gen_rand_f64(),	// p_high
		gen_rand_f64(),	// price
		gen_rand_f64(),	// quantity
		gen_rand_f64(),	// gas
	)	
}


pub fn setup_rand_ask_limit_order() -> Order {
	Order::new(
		gen_rand_trader_id(),
		OrderType::Enter,
		TradeType::Ask,
		ExchangeType::LimitOrder,
		gen_rand_f64(),	// p_low
		gen_rand_f64(),	// p_high
		gen_rand_f64(),	// price
		gen_rand_f64(),	// quantity
		gen_rand_f64(),	// gas
	)
}

pub fn setup_bids_book() -> Book {
	Book::new(TradeType::Bid)
}

pub fn setup_asks_book() -> Book {
	Book::new(TradeType::Ask)
}

pub fn setup_mem_pool() -> MemPool {
	MemPool::new()
}

pub fn each_order_type() -> Vec<Order> {
	let mut orders = Vec::<Order>::new();

	let b1 = setup_bid_limit_order();
	orders.push(b1);
	let mut b2 = setup_bid_limit_order();
	b2.order_type = OrderType::Update;
	orders.push(b2);
	let mut b3 = setup_bid_limit_order();
	b3.order_type = OrderType::Cancel;
	orders.push(b3);
	orders
}

pub fn setup_full_mem_pool() -> Arc<MemPool> {
	let mem_pool = Arc::new(setup_mem_pool());
	let mut handles: Vec<_> = Vec::new();

	for order in each_order_type() {
		handles.push(OrderProcessor::conc_recv_order(order, Arc::clone(&mem_pool)));
	}

	for h in handles {
		h.join().unwrap();
	}

	mem_pool
}

pub fn setup_n_full_mem_pool(n: usize) -> Arc<MemPool> {
	let mem_pool = Arc::new(setup_mem_pool());
	let mut handles: Vec<_> = Vec::new();

	for i in 0..n {
		if i % 2 == 0 {
			handles.push(OrderProcessor::conc_recv_order(setup_rand_bid_limit_order(), Arc::clone(&mem_pool)));
		} else {
			handles.push(OrderProcessor::conc_recv_order(setup_rand_ask_limit_order(), Arc::clone(&mem_pool)));
		}
	}

	for h in handles {
		h.join().unwrap();
	}

	mem_pool
}

pub fn rand_coef_vector() -> Vec<f64> {
	// Create a variable length vector filled with random f64's
	let mut rng = rand::thread_rng();
	let coefs: Vec<f64> = (0..rng.gen_range(0, 6)).map(|_| {
		let coef: f64 = rng.gen();
		coef * 10.0
	}).collect();
	coefs
}

pub fn n_bid_enters(n: usize) -> Vec<Order> {
	let mut bids = Vec::<Order>::new();
	for _ in 0..n {
		bids.push(rand_bid_limit_enter());
	}
	bids
}

pub fn n_ask_enters(n: usize) -> Vec<Order> {
	let mut asks = Vec::<Order>::new();
	for _ in 0..n {
		asks.push(rand_ask_limit_enter());
	}
	asks
}

pub fn setup_orders() -> (Vec<Order>, Vec<Order>) {
	let mut bids = Vec::<Order>::new();
	let mut asks = Vec::<Order>::new();
	for i in 1..101 {
		bids.push(Order::new(
			format!("INV{}", i), 
    		OrderType::Enter, 
    		TradeType::Bid, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		i as f64, 
    		5.0, 
    		0.5,
		));
		asks.push(Order::new(
			format!("MKR{}", i), 
    		OrderType::Enter, 
    		TradeType::Ask, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		i as f64, 
    		50.0, 
    		0.5,
		));

	}

	(bids, asks)
}

pub fn setup_n_investors(n: usize) -> Vec<Investor>{
	let mut vec = Vec::<Investor>::new();
	for i in 0..n {
		vec.push(Investor::new(format!("INV{}", i)));
	}
	vec
} 

pub fn setup_n_makers(n: usize) -> Vec<Maker> {
	let mut vec = Vec::<Maker>::new();
	for i in 0..n {
		vec.push(Maker::new(format!("MKR{}", i)));
	}
	vec
}

pub fn setup_flow_orders() -> (Vec<Order>, Vec<Order>) {
	let mut bids = Vec::<Order>::new();
	let mut asks = Vec::<Order>::new();
	for i in 0..100 {
		bids.push(Order::new(
			format!("INV{}", i), 
    		OrderType::Enter, 
    		TradeType::Bid, 
    		ExchangeType::FlowOrder,
    		i as f64, 	// p_low
    		100.0, 		// p_high
    		0.0,		// price
    		500.0,		// quantity
    		0.1, 		// gas
		));
		asks.push(Order::new(
			format!("MKR{}", i), 
    		OrderType::Enter, 
    		TradeType::Ask, 
    		ExchangeType::FlowOrder,
    		i as f64, 	// p_low
    		100.0, 		// p_high
    		0.0,		// price
    		500.0,		// quantity
    		0.1, 		// gas
		));

	}

	(bids, asks)
}

// N Bids, 2 Asks
pub fn setup_ask_cross_orders(num_bids: usize) -> (Vec<Order>, Vec<Order>) {
	let mut bids = Vec::<Order>::new();
	let mut asks = Vec::<Order>::new();
	for i in 1..num_bids + 1 {
		bids.push(Order::new(
			gen_rand_trader_id(), 
    		OrderType::Enter, 
    		TradeType::Bid, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		i as f64, 
    		5.0, 
    		0.5,
		));
	}

	// Essentially a market order
	asks.push(Order::new(
			gen_rand_trader_id(), 
    		OrderType::Enter, 
    		TradeType::Ask, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		0.0, 
    		50.0, 
    		0.5,
		));

	// An order that won't transact
	asks.push(Order::new(
			gen_rand_trader_id(), 
    		OrderType::Enter, 
    		TradeType::Ask, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		num_bids as f64 * 1000.0, 
    		50.0,
    		0.5, 
		));	

	(bids, asks)
}

// 2 Bids, N Asks
pub fn setup_bid_cross_orders(num_asks: usize) -> (Vec<Order>, Vec<Order>) {
	let mut bids = Vec::<Order>::new();
	let mut asks = Vec::<Order>::new();
	for i in 1..num_asks + 1 {
		asks.push(Order::new(
			gen_rand_trader_id(), 
    		OrderType::Enter, 
    		TradeType::Ask, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		50.0 + i as f64, 
    		5.0, 
    		0.5,
		));
	}

	// Essentially a market order
	bids.push(Order::new(
			gen_rand_trader_id(), 
    		OrderType::Enter, 
    		TradeType::Bid, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		num_asks as f64 * 1000.0, 
    		50.0, 
    		0.5,
		));

	// An order that won't transact
	bids.push(Order::new(
			gen_rand_trader_id(), 
    		OrderType::Enter, 
    		TradeType::Bid, 
    		ExchangeType::LimitOrder,
    		0.0,
    		0.0,
    		0.0, 
    		50.0, 
    		0.5,
		));	

	(bids, asks)
}











