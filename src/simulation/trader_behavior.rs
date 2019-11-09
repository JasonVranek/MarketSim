use crate::utility::gen_rand_trader_id;
use crate::simulation::trader::Traders;
use crate::order::order::{Order, OrderType, TradeType, ExchangeType};

use std::sync::Arc;
use rand::{Rng, thread_rng};


/// Function for parsing an order into it's Json components. 
pub fn params_for_json(order: &Order) -> (String, OrderType, TradeType, f64, f64) {
    return (order.trader_id.clone(),
        order.order_type.clone(),
        order.trade_type.clone(),
        order.price.clone(),
        order.quantity.clone());
}

/// A function to randomly generate update orders for existing traders within 
/// the Trader HashMap. The output is a vector of tuples where each tuple contains
/// the required parameters to generate a JSON formatted order. The supplied u32
/// 'upper' is to change the probability with which an update will occur for a 
/// given trader. Probability of update = (1 / upper), where upper > 0
pub fn gen_rand_updates(t_struct: Arc<Traders>, upper: u32) 
-> Vec<(String, OrderType, TradeType, f64, f64)> 
{
		let mut rng = thread_rng();
		// Get a lock on the HashMap 
		let mut orders = t_struct.traders.lock().unwrap();

		// Vector of tuples to construct JSON messages
		let mut to_send: Vec<_> = Vec::new();

		// Iterate through hashmap and update based on rng
		for order in orders.values_mut() {
			// (1 / upper) chance of updating the given order
			if rng.gen_range(0, upper) == 1 {
				// generate a new order with same trader_id and trader_type
				let new_order = rand_update_order(order);
				// parse and save the new order for params to make JSON
				to_send.push(params_for_json(order));
				// save the new order in the hashmap
				*order = new_order;
			}
		}
		to_send
	}

/// A function to randomly generate cancel orders for existing traders within 
/// the Trader HashMap. The output is a vector of tuples where each tuple contains
/// the required parameters to generate a JSON formatted order. The supplied u32
/// 'upper' is to change the probability with which an update will occur for a 
/// given trader. Probability of update = (1 / upper), where upper > 0
pub fn gen_rand_cancels(t_struct: Arc<Traders>, upper: u32) 
-> Vec<(String, OrderType, TradeType, f64, f64)> 
{
		let mut rng = thread_rng();
		// Get a lock on the HashMap 
		let mut orders = t_struct.traders.lock().unwrap();

		// Vector of tuples to construct JSON messages
		let mut to_send: Vec<_> = Vec::new();

		let length_before = orders.len();

		// Iterate through hashmap and filter out orders based on rng
		orders.retain(|_, order| {
			let rand = rng.gen_range(0, upper);
			// order was randomly selected to be cancelled
			if rand == 1 {
				// copy order's params for cancel json
				let mut p = params_for_json(order);
				// update OrderType to be a cancel order
				p.1 = OrderType::Cancel;
				to_send.push(p)
			}

			// (1 / upper) chance of cancelling the given order
			!(rand == 1)
		});

		assert_eq!(length_before, orders.len() + to_send.len());
		to_send
	}

/// Generates a random number of Bid and Ask orders all of OrderType::Enter
/// and returns them in a vector.
pub fn rand_enters(upper: u64) -> Vec<Order> {
	let mut rng = thread_rng();
	let mut orders = Vec::<Order>::new();

	for _ in 0..rng.gen_range(0, upper) {
		orders.push(rand_bid_limit_enter());
	}

	for _ in 0..rng.gen_range(0, upper) {
		orders.push(rand_ask_limit_enter());
	}
	orders
}

/// Generates a random Ask order of OrderType::Enter
pub fn rand_ask_limit_enter() -> Order {
	let (price, quantity) = gen_limit_order();			//TODOOO LOOK AT THIS AGAIN
	Order::new(
		gen_rand_trader_id(),
		OrderType::Enter,
		TradeType::Ask,
		ExchangeType::LimitOrder,
		0.0,
		0.0,	
		price,
		quantity,
		0.5,
	)
}

/// Generates a random Bid order of OrderType::Enter
pub fn rand_bid_limit_enter() -> Order {
	let (price, quantity) = gen_limit_order();				//TODOOO LOOK AT THIS AGAIN
	Order::new(
		gen_rand_trader_id(),
		OrderType::Enter,
		TradeType::Bid,
		ExchangeType::LimitOrder,
		0.0,
		0.0,
		price,
		quantity,
		0.5,
	)
}

/// Randomizes the fields of an order but retains trade_id and trade_type
pub fn rand_update_order(old: &Order) -> Order {
	
    let mut new = match old.trade_type {
    	TradeType::Bid => rand_bid_limit_enter(),
    	TradeType::Ask => rand_ask_limit_enter(),
    };
    new.order_type = OrderType::Update;
    new.trader_id = old.trader_id.clone();
    new
}

/// Create a random price and quantity
pub fn gen_limit_order() -> (f64, f64) {
	let mut rng = thread_rng();
	let p: f64 = rng.gen_range(90.0, 110.0);
	let q: f64 = rng.gen_range(0.0, 10.0);
	(p, q)
}


#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_new_traders() {
		let t_struct = Traders::new();
		let map = t_struct.traders.lock().unwrap();
		assert_eq!(map.len(), 0);
	}

	#[test]
	fn test_insert_traders() {
		let mut t_struct = Traders::new();
		t_struct.new_trader(rand_bid_limit_enter());
		t_struct.new_trader(rand_ask_limit_enter());

		assert_eq!(t_struct.traders.lock().unwrap().len(), 2);
	}
}