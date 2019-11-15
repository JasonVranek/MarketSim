use crate::order::order::{Order, TradeType};
use crate::players::{TraderT, Player};
use crate::players::investor::Investor;
use crate::players::maker::Maker;
use crate::players::miner::Miner;

use std::collections::HashMap;
use std::sync::Mutex;


/// The struct for keeping track of active players and their balances and inventories
/// ClearingHouse is a HashMap indexed by each player's trader_id
pub struct ClearingHouse {
	pub players: Mutex<HashMap<String, Box<Investor>>>,
}



impl ClearingHouse {
	pub fn new() -> Self {
		ClearingHouse {
			players: Mutex::new(HashMap::new()),	
		}
	}

	/// Register an investor to the ClearingHouse Hashmap
	pub fn reg_investor(&mut self, inv: Investor) {
		unimplemented!();
	}


	/// Register a miner to the ClearingHouse Hashmap
	pub fn reg_maker(&mut self, maker: Maker) {
		unimplemented!();
	}


	/// Register a miner to the ClearingHouse Hashmap
	pub fn reg_miner(&mut self, miner: Miner) {
		unimplemented!();
	}


	/// Updates a single player's balance and inventory 
	pub fn update_player(&mut self, bidder: String, asker: String, p: f64, q:f64) {
		unimplemented!();
	}	

	/// Atomically updates balance and inventory for two players
	/// Adds p to pay_to's balance and subtracts q from pay_to's inventory
	/// Adds q to inv_to's inventory and subtracts p from inv_to's balance
	pub fn atomic_swap(&mut self, pay_to: String, inv_to: String, p: f64, q: f64) {
		unimplemented!();
	}

	pub fn update_house() {
		unimplemented!();
	}

	/// Removes player from clearing house
	pub fn remove_player(&mut self, order: Order) {
		unimplemented!();
		// search based on order.trader_id
		// pop based on order_id
	}
	
	/// Add a new order to the HashMap indexed by te player's id
	pub fn new_order(&mut self, order: Order) {
		let mut players = self.players.lock().unwrap();
		// Find the player by trader id

		// Append the order to player's vec of orders
	}

	/// Add a vector of new orders to the HashMap. This is preferable to new_order
	/// as the mutex lock only has to be acquired once.
	pub fn new_orders(&self, orders: Vec<Order>) {
		let mut traders = self.players.lock().unwrap();
		for order in orders {
			// or_insert will not overwrite an existing entry, but will insert if the key doesn't exist
			// traders.entry(order.trader_id.clone()).or_insert(order);
		}
	}

	/// Updates a trader's order in the HashMap with the supplied 'order'
	// pub fn update_trader(&mut self, order: Order) {
	// 	self.traders.lock().unwrap().insert(order.trader_id.clone(), order);
	// }

	/// Removes the player from the HashMap
	pub fn del_player(&mut self, trader_id: String) {
		self.players.lock().unwrap().remove(&trader_id);
	}

	/// Utility function for seeing how many Trader's are currently active
	pub fn num_players(&self) -> usize {
		self.players.lock().unwrap().len()
	}
}

	// Utility function for seeing how many orders are currently active (not nec in order book)
	// pub fn num_orders(&self) -> usize {
	// 	let players = self.players.lock().unwrap();
	// 	let mut sum = 0;
	// 	for p in players {
			
	// 	}









