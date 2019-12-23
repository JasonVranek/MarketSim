use crate::simulation::simulation_history::UpdateReason;
use crate::order::order::Order;
use std::any::Any;


pub mod investor;
pub mod maker;
pub mod miner;


/// Enum for matching over trader types
#[derive(Debug, PartialEq, Copy)]
pub enum TraderT {
    Maker,
    Investor,
    Miner,
}

impl Clone for TraderT {
	fn clone(&self) -> TraderT { 
		match self {
			TraderT::Maker => TraderT::Maker,
			TraderT::Investor => TraderT::Investor,
			TraderT::Miner => TraderT::Miner,
		}
	}
}



/// A trait common to Investors, Makers, and Miners
pub trait Player {
	fn get_id(&self) -> String;

	fn get_bal(&self) -> f64;

	fn get_inv(&self) -> f64;

	fn update_bal(&mut self, to_add: f64);

	fn update_inv(&mut self, to_add: f64);

	fn add_order(&mut self, order: Order);

	fn num_orders(&self) -> usize;

	fn gen_cancel_order(&mut self, o_id: u64) -> Result<Order, &'static str>;	

	fn cancel_order(&mut self, o_id: u64) -> Result<(), &'static str>;

	fn get_enter_order_ids(&self) -> Vec<u64>;

	fn update_order_vol(&mut self, o_id: u64, vol_to_add: f64) -> Result<(), &'static str>;

	fn copy_orders(&self) -> Vec<Order>;

	fn get_player_type(&self) -> TraderT;

	fn as_any(&self) -> &dyn Any;

	fn log_to_csv(&self, reason: UpdateReason) -> String;
}














