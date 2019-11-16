use crate::players::Player;
use std::sync::Mutex;
use crate::order::order::{Order, TradeType};



/// A struct for the Investor player. 
pub struct Investor {
	pub trader_id: String,
	pub orders: Mutex<Vec<Order>>,
	pub balance: f64,
	pub inventory: f64,
}

/// The 
impl Investor {
	pub fn new(trader_id: String) -> Investor {
		Investor {
			trader_id: trader_id,
			orders: Mutex::new(Vec::<Order>::new()),
			balance: 0.0,
			inventory: 0.0,
		}
	}

	pub fn new_limit_order() -> Order {
		unimplemented!();
	}
}

impl Player for Investor {
	

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
