use crate::simulation::simulation_history::UpdateReason;
use crate::utility::get_time;
use crate::players::{Player,TraderT};
use std::sync::Mutex;
use crate::order::order::{Order, OrderType};

use std::any::Any;



/// A struct for the Investor player. 
pub struct Investor {
	pub trader_id: String,
	pub orders: Mutex<Vec<Order>>,
	pub balance: f64,
	pub inventory: f64,
	pub player_type: TraderT,
	pub sent_orders: Mutex<Vec<(u64, OrderType)>>,
}

/// The 
impl Investor {
	pub fn new(trader_id: String) -> Investor {
		Investor {
			trader_id: trader_id,
			orders: Mutex::new(Vec::<Order>::new()),
			balance: 0.0,
			inventory: 0.0,
			player_type: TraderT::Investor,
			sent_orders: Mutex::new(Vec::<(u64, OrderType)>::new()),
		}
	}

	pub fn new_limit_order() -> Order {
		unimplemented!();
	}
}

impl Player for Investor {
	fn as_any(&self) -> &dyn Any {
		self
	}

	fn get_id(&self) -> String {
		self.trader_id.clone()
	}
	

	fn get_bal(&self) -> f64 {
		self.balance
	}

	fn get_inv(&self) -> f64 {
		self.inventory
	}

	fn get_player_type(&self) -> TraderT {
		self.player_type
	}

	fn update_bal(&mut self, to_add: f64) {
		self.balance += to_add;
	}

	fn update_inv(&mut self, to_add: f64) {
		self.inventory += to_add;
	}

	fn add_order(&mut self,	 order: Order) {
		let mut orders = self.orders.lock().expect("Couldn't lock orders");
		// Add the order info to the sent_orders to track orders to mempool
		self.sent_orders.lock().expect("investor add_order").push((order.order_id, order.order_type.clone()));
		orders.push(order);
	} 

	// Checks if a cancel order has already been sent to the mempool
	fn check_double_cancel(&self, o_id: u64) -> bool {
		let sent = self.sent_orders.lock().unwrap();
		for order in sent.iter() {
			if order.0 == o_id && order.1 == OrderType::Cancel {
				return true;
			}
		}
		false
	}


	fn add_to_sent(&self, o_id: u64, order_type: OrderType) {
		let mut sent = self.sent_orders.lock().expect("add_to_sent");
		sent.push((o_id, order_type));
	}

	fn num_orders(&self) -> usize {
		self.orders.lock().unwrap().len()
	}

	fn get_enter_order_ids(&self) -> Vec<u64> {
		let orders = self.orders.lock().expect("get_enter_order_ids");
		let mut ids = Vec::new();
		for o in orders.iter() {
			if o.order_type == OrderType::Enter {
				ids.push(o.order_id);
			}
		}
		ids
	}

	// Creates a cancel order for the specified order id
	fn gen_cancel_order(&mut self, o_id: u64) -> Result<Order, &'static str> {
		// Get the lock on the player's orders
		let orders = self.orders.lock().expect("couldn't acquire lock cancelling order");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
			let order = orders.get(i).expect("investor cancel_order");
			let mut copied = order.clone();
			copied.order_type = OrderType::Cancel;
			return Ok(copied.clone());
        } else {
        	return Err("ERROR: order not found to cancel");
        }
	}


	// Removes the cancel order from the player's active orders
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

	// Updates the order's volume and removes it if the vol <= 0
	fn update_order_vol(&mut self, o_id: u64, vol_to_add: f64) -> Result<(), &'static str> {
		// Get the lock on the player's orders
		let mut orders = self.orders.lock().expect("couldn't acquire lock on orders");
		// Find the index of the existing order using the order_id
		let order_index: Option<usize> = orders.iter().position(|o| &o.order_id == &o_id);
		
		if let Some(i) = order_index {
        	orders[i].quantity += vol_to_add;
        	// println!("new quantity: {}", orders[i].quantity);
        	if orders[i].quantity <= 0.0 {
        		orders.remove(i);
        	}
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

	fn log_to_csv(&self, reason: UpdateReason) -> String {
		format!("{:?},{:?},{},{:?},{},{},", 
				get_time(), 
				reason,
				self.trader_id.clone(),
				self.player_type.clone(),
				self.balance,
				self.inventory)
	}

}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new_investor() {
		let mut i = Investor::new(format!("{:?}", "BillyBob"));
		i.update_bal(55.0);
		i.update_inv(100.0);

		assert_eq!(i.get_bal(), 55.0);
		assert_eq!(i.get_inv(), 100.0);

	}


}
