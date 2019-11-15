pub mod investor;
pub mod maker;
pub mod miner;


/// Enum for matching over trader types
#[derive(Debug, PartialEq)]
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
	fn new(trader_id: String) -> Self;

	fn get_bal(&self) -> f64;

	fn get_inv(&self) -> f64;

	fn update_bal(&mut self, to_add: f64);

	fn update_inv(&mut self, to_add: f64);
}