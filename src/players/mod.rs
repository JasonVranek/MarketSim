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



/// 
pub struct Player {

}