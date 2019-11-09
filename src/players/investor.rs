use std::sync::Mutex;
use crate::order::order::{Order, TradeType};



/// A struct for the Investor player. 
pub struct Investor {
	pub trader_id: String,
	pub orders: Mutex<Vec<Order>>,
	pub balance: f64,
	pub inventory: f64,
}