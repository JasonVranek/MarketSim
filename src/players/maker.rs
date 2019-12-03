use crate::simulation::simulation_history::{PriorData, LikelihoodStats};
use crate::players::{Player, TraderT};
use std::sync::Mutex;
use crate::order::order::{Order};

use rand::Rng;

use std::any::Any;


#[derive(Debug, Clone)]
pub enum MakerT {
	Aggressive,
	RiskAverse,
	Random,
}


const NUM_TYPES: usize = MakerT::Random as usize + 1;



/// A struct for the Maker player. 
pub struct Maker {
	pub trader_id: String,
	pub orders: Mutex<Vec<Order>>,
	pub balance: f64,
	pub inventory: f64,
	pub player_type: TraderT,
	pub maker_type: MakerT,
}

/// Logic for Maker trading strategy
impl Maker {
	pub fn new(trader_id: String, maker_type: MakerT) -> Maker {
		Maker {
			trader_id: trader_id,
			orders: Mutex::new(Vec::<Order>::new()),
			balance: 0.0,
			inventory: 0.0,
			player_type: TraderT::Maker,
			maker_type: maker_type,
		}
	}

	pub fn copy_last_order(&self) -> Option<Order> {
		let orders = self.orders.lock().unwrap();
		match orders.last(){
			Some(order) => Some(order.clone()),
			None => None,
		}
	}

	pub fn gen_rand_type() -> MakerT {
		let mut rng = rand::thread_rng();
		match rng.gen_range(0, NUM_TYPES){
			0 => MakerT::Aggressive,
			1 => MakerT::RiskAverse,
			2 => MakerT::Random,
			_ => MakerT::Random,
		}
	}

	pub fn new_order(&self, data: &PriorData, inference: &LikelihoodStats) -> Option<Order> {
		// Match based on strategy to corresponding function
		match self.maker_type {
			MakerT::Aggressive => self.aggressive_order(data, inference),
			MakerT::RiskAverse => self.risk_averse_order(data, inference),
			MakerT::Random => self.random_order(data, inference),
		}
	}

	pub fn aggressive_order(&self, data: &PriorData, inference: &LikelihoodStats) -> Option<Order> {
		unimplemented!()
	}

	pub fn risk_averse_order(&self, data: &PriorData, inference: &LikelihoodStats) -> Option<Order> {
		unimplemented!()
	}

	pub fn random_order(&self, data: &PriorData, inference: &LikelihoodStats) -> Option<Order> {
		unimplemented!()
	}
}



impl Player for Maker {
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
		let mut m = Maker::new(format!("{:?}", "BillyBob"), Maker::gen_rand_type());
		m.update_bal(55.0);
		m.update_inv(100.0);

		assert_eq!(m.get_bal(), 55.0);
		assert_eq!(m.get_inv(), 100.0);

	}


}