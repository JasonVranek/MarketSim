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


impl Maker {

}



impl Player for Maker {
	fn new(trader_id: String) -> Maker {
		Maker {
			trader_id: trader_id,
			orders: Mutex::new(Vec::<Order>::new()),
			balance: 0.0,
			inventory: 0.0,
		}
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