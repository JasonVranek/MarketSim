use crate::simulation::simulation_history::UpdateReason;
use crate::utility::get_time;
use crate::simulation::simulation_config::{Distributions, Constants};
use crate::simulation::simulation_history::{PriorData, LikelihoodStats};
use crate::exchange::MarketType;
use crate::players::{Player, TraderT};
use crate::order::order::{Order, TradeType, ExchangeType, OrderType};
use std::sync::Mutex;

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

	// Calculates gas price based on maker type
	pub fn calc_gas(&self, mean_gas: f64, _dists: &Distributions, consts: &Constants) -> f64 {
		match self.maker_type {
			MakerT::Aggressive => {
			// Aggressive players will place new gas price > mean
				mean_gas + Distributions::sample_uniform(0.01, consts.maker_base_spread, None)
			},
			MakerT::RiskAverse => {
			// RiskAverse players will place new gas price = mean
				mean_gas
			},
			MakerT::Random => {
			// Random players will place new gas price centered around mean
				Distributions::sample_normal(mean_gas, 0.05, None).abs()
			},
		}
	}

	pub fn normalize_inv(&self, consts: &Constants) -> f64 {
		let inv = self.inventory;
		if inv < 0.0 {
			// return a ratio between [0.5, 1.0]
			let ratio = 0.5 + (inv * 0.5) / consts.max_held_inventory;
			if ratio > 1.0 {
				return 1.0;
			}
			return ratio;

		} else {
			// return a ratio between [0.0, 0.5]
			let ratio = 0.0 + (inv * 0.5) / consts.max_held_inventory;
			if ratio > 0.5 {
				return 0.5;
			}
			return ratio;
		}
	}

	// Calculates a price offset based on the makers type
	// Given a price calculates the bid ask prices using maker type to determine spread
	// returns tuple (bid_price, ask_price, bid_inv, ask_inv)
	pub fn calc_price_inv(&self, price: Option<f64>, _dists: &Distributions, consts: &Constants, _ask_vol: f64, _bid_vol: f64) -> Option<(f64, f64, f64, f64)> {
		match price {
			// inf_fv = the inferred fundamental value
			Some(inf_fv) => {
				let spread;
				match self.maker_type {
					MakerT::Aggressive => {
						spread = consts.maker_base_spread;
					},
					MakerT::RiskAverse => {
						// Slightly bigger spread
						spread = 2.0 * consts.maker_base_spread;
					},
					MakerT::Random => {
						spread = Distributions::sample_normal(0.1 * consts.maker_base_spread, consts.maker_base_spread, None).abs();
					},
				}

				// Calculate the prices based on inventory and spreads
				let cur_inv = self.inventory;
				if cur_inv == 0.0 {
					// Maker has no inventory so center prices around inferred fund value
					let bid_price = inf_fv - (spread / 2.0);
					let ask_price = inf_fv + (spread / 2.0);
					// let bid_inv = dists.sample_dist(DistReason::MakerOrderVolume).expect("MakerOrderVolume");
					// let ask_inv = bid_inv;
					let bid_inv = 0.5;
					let ask_inv = 0.5;
					Some((bid_price, ask_price, bid_inv, ask_inv))
				} else if cur_inv < 0.0 {
					// Maker has negative inventory, so shift spread for better bid price, worse ask price
					let ratio = self.normalize_inv(&consts); 
					let bid_spread = ratio * spread;
					let ask_spread = (1.0 - ratio) * spread;
					let bid_price = inf_fv - bid_spread;
					let ask_price = inf_fv + ask_spread;
					// let inv_amt = dists.sample_dist(DistReason::MakerOrderVolume).expect("MakerOrderVolume");
					// let bid_inv = ratio * inv_amt;
					// let ask_inv = (1.0 - ratio) * inv_amt;
					let bid_inv = ratio;
					let ask_inv = 1.0 - ratio;
					Some((bid_price, ask_price, bid_inv, ask_inv))

				} else {
					// Maker has positive inventory, so shift spread for better ask price, worse bid price
					let ratio = self.normalize_inv(&consts); 
					let bid_spread = ratio * spread;
					let ask_spread = (1.0 - ratio) * spread;
					let bid_price = inf_fv - bid_spread;
					let ask_price = inf_fv + ask_spread;
					// let inv_amt = dists.sample_dist(DistReason::MakerOrderVolume).expect("MakerOrderVolume");
					// let bid_inv = ratio * inv_amt;
					// let ask_inv = (1.0 - ratio) * inv_amt;
					let bid_inv = ratio;
					let ask_inv = 1.0 - ratio;
					Some((bid_price, ask_price, bid_inv, ask_inv))
				}
			},
			None => None,	// No price was supplied to determine maker's price
		}
		
	}


	pub fn new_orders(&self, data: &PriorData, inference: &LikelihoodStats, dists: &Distributions, consts: &Constants) -> Option<(Order, Order)> {
		// look at the weighted average price of the mempool, exit if no orders have been sent to pool
		let wtd_pool_price = match inference.weighted_price {
			Some(price) => price,
			None => return None,
		};
			
		// Look at the last public order book average and mean gas
		// let _wtd_last_book_price = data.current_wtd_price;
		let wtd_gas = data.mean_pool_gas;
		let ask_vol = data.asks_volume;
		let bid_vol = data.bids_volume;


		// type of order (FlowOrder or LimitOrder)
		let ex_type = match consts.market_type {
			MarketType::CDA|MarketType::FBA => ExchangeType::LimitOrder,
			MarketType::KLF => ExchangeType::FlowOrder,
		};

		// Calculate the bid and ask prices offset from weighted avg price of all seen orders based on maker type
		// And the respective quantity for each order
		let (bid_price, ask_price, bid_amt, ask_amt) = match self.calc_price_inv(Some(wtd_pool_price), dists, consts, ask_vol, bid_vol) {
			Some((bp, ap, ba, aa)) => (bp, ap, ba, aa),
			None => return None,
		};

		// Need to set p_low and p_high (unused in limit orders)
		let bid_p_low = bid_price;
		let bid_p_high = bid_price + consts.flow_order_offset;
		let ask_p_low = ask_price - consts.flow_order_offset;
		let ask_p_high = ask_price;
		
		// gas
		let gas = self.calc_gas(wtd_gas, dists, consts);

		let bid_order = Order::new(self.trader_id.clone(), 
									   OrderType::Enter,
							   	       TradeType::Bid,
								       ex_type.clone(),
								       bid_p_low,
								       bid_p_high,
								       bid_price,
								       bid_amt,
								       gas
		);

		let ask_order = Order::new(self.trader_id.clone(), 
									   OrderType::Enter,
							   	       TradeType::Ask,
								       ex_type,
								       ask_p_low,
								       ask_p_high,
								       bid_price,
								       ask_amt,
								       gas
		);

		Some((bid_order, ask_order))
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
	fn test_new_maker() {
		let mut m = Maker::new(format!("{:?}", "BillyBob"), Maker::gen_rand_type());
		m.update_bal(55.0);
		m.update_inv(100.0);

		assert_eq!(m.get_bal(), 55.0);
		assert_eq!(m.get_inv(), 100.0);

	}


}