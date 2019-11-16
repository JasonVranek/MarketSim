use crate::order::order::{Order};
use crate::players::{Player};
use crate::players::investor::Investor;
use crate::players::maker::Maker;
use crate::players::miner::Miner;

use std::collections::HashMap;
use std::sync::Mutex;


/// The struct for keeping track of active players and their balances and inventories
/// ClearingHouse is a HashMap indexed by each player's trader_id
pub struct ClearingHouse {
	pub players: Mutex<HashMap<String, Box<dyn Player>>>,
}



impl ClearingHouse {
	/// Create a new ClearingHouse to store player data
	pub fn new() -> Self {
		ClearingHouse {
			players: Mutex::new(HashMap::new()),	
		}
	}


	/// Register an investor to the ClearingHouse Hashmap
	pub fn reg_investor(&mut self, inv: Investor) {
		let mut players = self.players.lock().unwrap();
		players.entry(inv.trader_id.clone()).or_insert(Box::new(inv));
	}


	/// Register a miner to the ClearingHouse Hashmap
	pub fn reg_maker(&mut self, maker: Maker) {
		let mut players = self.players.lock().unwrap();
		players.entry(maker.trader_id.clone()).or_insert(Box::new(maker));
	}


	/// Register a miner to the ClearingHouse Hashmap
	pub fn reg_miner(&mut self, miner: Miner) {
		let mut players = self.players.lock().unwrap();
		players.entry(miner.trader_id.clone()).or_insert(Box::new(miner));
	}


	// Gets a reference to the player by popping it from the hashmap
	pub fn get_player(&mut self, id: String) -> Option<Box<dyn Player>> {
		let mut players = self.players.lock().unwrap();
		match players.remove(&id) {
			Some(player) => Some(player),
			None => None,
		}
	}


	/// Adds to the player's balance and returns their updated balance
	pub fn update_player_bal(&mut self, id: String, bal_to_add: f64) -> Option<f64> {
		let mut players = self.players.lock().unwrap();
		match players.get_mut(&id) {
			Some(player) => { 
				player.update_bal(bal_to_add);
				Some(player.get_bal())
			}
			None => None,
		}
	}


	/// Adds to the player's inventory and returns their updated inventory
	pub fn update_player_inv(&mut self, id: String, inv_to_add: f64) -> Option<f64> {
		let mut players = self.players.lock().unwrap();
		match players.get_mut(&id) {
			Some(player) => { 
				player.update_inv(inv_to_add);
				Some(player.get_inv())
			}
			None => None,
		}
	}


	/// Updates both a single player's balance and inventory
	/// Returns tuple Option<(updated_bal: f64, updated_inv: f64)>
	pub fn update_player(&mut self, id: String, bal_to_add: f64, inv_to_add: f64) -> Option<(f64, f64)>{
		let mut players = self.players.lock().unwrap();
		match players.get_mut(&id) {
			Some(player) => { 
				player.update_inv(inv_to_add);
				player.update_bal(bal_to_add);
				Some((player.get_bal(), player.get_inv()))
			}
			None => None,
		}
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

	
	/// Add a new order to the HashMap indexed by te player's id
	pub fn new_order(&mut self, order: Order) -> Result<(), &str> {
		let mut players = self.players.lock().unwrap();
		// Find the player by trader id and add their order
		match players.get_mut(&order.trader_id) {
			Some(player) => { 
				player.add_order(order);
				Ok(())
			}
			None => Err("Couldn't find trader to add order")
		}
	}

	/// Add a vector of new orders to the HashMap. This is preferable to new_order
	/// as the mutex lock only has to be acquired once.
	pub fn new_orders(&self, orders: Vec<Order>) -> Result<(), &str> {
		let mut players = self.players.lock().unwrap();
		for order in orders {
			match players.get_mut(&order.trader_id) {
				Some(player) => { 
					player.add_order(order);
				}
				None => return Err("Couldn't find trader to add order"),
			}
		}
		Ok(())
	}

	/// Updates a trader's order in the HashMap with the supplied 'order'
	pub fn update_player_order(&mut self, order: Order) {
		// self.players.lock().unwrap().insert(order.trader_id.clone(), order);
	}


	/// Cancel's a trader's order in the HashMap with the supplied 'order'
	pub fn cancel_player_order(&mut self, trader_id: String, order_id: u64) -> Result<(), &str> {
		if let Some(player) = self.get_player(trader_id) {
			// Get the lock on the player's orders
			let mut orders = player.orders.lock().expect("couldn't acquire lock cancelling order");
			// find the index of the existing order using the order_id
			let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &order_id);
			
			if let Some(i) = order_index {
	        	orders.remove(i);
	        } else {
	        	println!("ERROR: order not found to cancel: {:?}", order_id);
	        	return Err("ERROR: order not found to cancel");
	        }
		} else {
			Err("Couldn't find player to cancel order");
		}
	}

	/// Removes the player from the ClearingHouse HashMap
	pub fn del_player(&mut self, trader_id: String) -> Option<()>{
		match self.players.lock().unwrap().remove(&trader_id) {
			Some(_p) => Some(()),
			None => None
		}
	}

	/// Utility function for seeing how many Trader's are currently active
	pub fn num_players(&self) -> usize {
		self.players.lock().unwrap().len()
	}

	/// Utility function for seeing how many orders are currently active (not nec in order book)
	pub fn orders_in_house(&self) -> usize {
		let players = self.players.lock().unwrap();
		let mut sum = 0;
		for (_id, p) in players.iter() {
			sum += p.num_orders();
		}
		sum
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ch() {
		let mut i = Investor::new(format!("{:?}", "BillyBob"));
		i.update_bal(55.0);
		i.update_inv(100.0);

		let mut mkr = Maker::new(format!("{:?}", "NillyNob"));
		mkr.update_bal(55.0);
		mkr.update_inv(100.0);

		let min = Miner::new(format!("{:?}", "SquillyFob"));

		let mut ch = ClearingHouse::new();

		// Test adding new players
		ch.reg_investor(i);
		ch.reg_maker(mkr);
		ch.reg_miner(min);
		assert_eq!(ch.num_players(), 3);

		// Test updating a player's balance
		if let Some(bal) = ch.update_player_bal(format!("{:?}", "BillyBob"), 40.0) {
			assert_eq!(bal, 95.0);
		} else {
			panic!("AHHH failed to update player balance");
		}

		// Test updating a player's balance
		if let Some(inv) = ch.update_player_inv(format!("{:?}", "NillyNob"), -40.0) {
			assert_eq!(inv, 60.0);
		} else {
			panic!("AHHH failed to update player inventory");
		}

		// Test updating both
		if let Some((bal, inv)) = ch.update_player(format!("{:?}", "SquillyFob"), -40.0, 20.0) {
			assert_eq!(inv, 20.0);
			assert_eq!(bal, -40.0);
		} else {
			panic!("AHHH failed to update player");
		}

		if let Some(_) = ch.del_player(format!("{:?}", "SquillyFob")) {
			assert_eq!(ch.num_players(), 2);
		} else {
			panic!("AHHH failed to delete player");
		}

	}
}







