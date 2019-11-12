pub mod exchange_logic;
pub mod clearing_house;

#[derive(Debug)]
pub enum MarketType {
	CDA,
	FBA,
	KLF,
}

impl Clone for MarketType {
	fn clone(&self) -> MarketType { 
		match self {
			MarketType::CDA => MarketType::CDA,
			MarketType::FBA => MarketType::FBA,
			MarketType::KLF => MarketType::KLF,
		}
	}
}