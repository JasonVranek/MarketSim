use crate::players::Player;
use std::sync::Mutex;
use crate::order::order::{Order, TradeType};

/// A struct for the Maker player. 
pub struct Maker {
	pub trader_id: String,
	pub orders: Mutex<Vec<Order>>,
	pub balance: f64,
	pub inventory: f64,
}

/// Logic for Maker trading strategy
impl Maker {
	pub fn new(trader_id: String) -> Maker {
		Maker {
			trader_id: trader_id,
			orders: Mutex::new(Vec::<Order>::new()),
			balance: 0.0,
			inventory: 0.0,
		}
	}
}



impl Player for Maker {
	fn get_id(&self) -> String {
		self.trader_id.clone()
	}
	
	fn get_bal(&self) -> f64 {
		self.balance
	}

	fn get_inv(&self) -> f64 {
		self.inventory
	}

	fn update_bal(&mut self, to_add: f64) {
		self.balance += to_add;
	}

	fn update_inv(&mut self, to_add: f64) {
		self.inventory += to_add;
	}

	fn add_order(&mut self,	 order: Order) {
		let mut orders = self.orders.lock().expect("Couldn't lock orders");
		orders.push(order);
	} 

	fn num_orders(&self) -> usize {
		self.orders.lock().unwrap().len()
	}

	fn cancel_order(&mut self, o_id: u64) -> Result<(), &'static str> {
		// Get the lock on the player's orders
		let mut orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
        	orders.remove(i);
        	return Ok(());
        } else {
        	return Err("ERROR: order not found to cancel");
        }
	}


	fn update_order_vol(&mut self, o_id: u64, vol_to_add: f64) -> Result<(), &'static str> {
		// Get the lock on the player's orders
		let mut orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
        	orders[i].quantity += vol_to_add;
        	return Ok(());
        } else {
        	return Err("ERROR: order not found to cancel");
        }
	}

	fn copy_orders(&self) -> Vec<Order> {
		let orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		let mut copied = Vec::<Order>::new();
		for o in orders.iter() {
			copied.push(o.clone());
		}
		copied
	}

}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new_maker() {
		let mut m = Maker::new(format!("{:?}", "BillyBob"));
		m.update_bal(55.0);
		m.update_inv(100.0);

		assert_eq!(m.get_bal(), 55.0);
		assert_eq!(m.get_inv(), 100.0);

	}


}