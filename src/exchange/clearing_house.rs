use crate::order::order::{Order, TradeType};
use crate::players::{TraderT, Player};
use crate::players::investor::Investor;
use crate::players::maker::Maker;
use crate::players::miner::Miner;

use std::collections::HashMap;
use std::sync::Mutex;


/// The struct for keeping track of active players and their balances and inventories
/// ClearingHouse is a HashMap indexed by the 
pub struct ClearingHouse {
	pub traders: Mutex<HashMap<String, Player>>,
}



impl ClearingHouse {
	pub fn new() -> Self {
		ClearingHouse {
			traders: Mutex::new(HashMap::new()),	
		}
	}

	/// Add an investor to the ClearingHouse Hashmap
	pub fn reg_investor(&mut self, inv: Investor) {
		unimplemented!();
	}


	/// Add a miner to the ClearingHouse Hashmap
	pub fn reg_maker(&mut self, maker: Maker) {
		unimplemented!();
	}


	/// Add a miner to the ClearingHouse Hashmap
	pub fn reg_miner(&mut self, miner: Miner) {
		unimplemented!();
	}


	/// Updates a single player's balance and inventory 
	pub fn update_trader(&mut self, bidder: String, asker: String, p: f64, q:f64) {
		unimplemented!();
	}	

	/// Atomically updates balance and inventory for two players
	/// Adds p to pay_to's balance and subtracts q from pay_to's inventory
	/// Adds q to inv_to's inventory and subtracts p from inv_to's balance
	pub fn atomic_swap(&mut self, pay_to: String, inv_to: String, p: f64, q: f64) {
		unimplemented!();
	}

	/// Removes order from Player list
	pub fn remove_order(&mut self, order: Order) {
		unimplemented!();
		// search based on order.trader_id
		// pop based on order_id
	}
	

	/// Add a new order to the Traders HashMap
	// pub fn new_trader(&mut self, order: Order) {
	// 	let mut traders = self.traders.lock().unwrap();
	// 	// or_insert will not overwrite an existing entry, but will insert if the key doesn't exist
	// 	traders.entry(order.trader_id.clone()).or_insert(order);
	// }

	/// Add a vector of new orders to the Traders HashMap. This is preferable to new_trader
	/// as the mutex lock only has to be acquired once.
	// pub fn new_traders(&self, orders: Vec<Order>) {
	// 	let mut traders = self.traders.lock().unwrap();
	// 	for order in orders {
	// 		traders.entry(order.trader_id.clone()).or_insert(order);
	// 	}
	// }

	/// Updates a trader's order in the HashMap with the supplied 'order'
	// pub fn update_trader(&mut self, order: Order) {
	// 	self.traders.lock().unwrap().insert(order.trader_id.clone(), order);
	// }

	/// Removes the trader and their order from the HashMap
	pub fn del_trader(&mut self, trader_id: String) {
		self.traders.lock().unwrap().remove(&trader_id);
	}

	/// Utility function for seeing how many Trader's are currently active
	pub fn num_traders(&self) -> usize {
		self.traders.lock().unwrap().len()
	}
}