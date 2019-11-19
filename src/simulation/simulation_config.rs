// File for loading in all the parameters for the simulation and then
// setting up the appropriate constants and distributions.
// Using distribution libary: https://docs.rs/statrs/0.5.1/statrs/distribution
use statrs::distribution::{Normal, Uniform, Poisson, Exponential};


pub struct Constants {
	pub batch_interval: u64,
	pub num_investors: u64,
	pub num_makers: u64,
	pub block_size: u64,
}

pub enum DistType {
	Uniform,
	Normal,
	Poisson,
	Exponential,
}

// Each distribution is in the form (µ: f64, std_dev: f64, DistType)
pub struct Distributions {
	pub asks_price: 		(f64, f64, DistType),
	pub bids_center: 		(f64, f64, DistType),
	pub miner_front_run: 	(f64, f64, DistType),
	pub miner_frame_form: 	(f64, f64, DistType),
	pub propagation_delay: 	(f64, f64, DistType),
	pub investor_gas: 		(f64, f64, DistType),
	pub investor_enter: 	(f64, f64, DistType),
	pub maker_type: 		(f64, f64, DistType),
	pub maker_inventory: 	(f64, f64, DistType),
	pub maker_balance: 		(f64, f64, DistType),
}


impl Distributions {
	pub fn sample_uniform() {

	}
}