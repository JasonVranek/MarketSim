// File for loading in all the parameters for the simulation and then
// setting up the appropriate constants and distributions.
use crate::exchange::MarketType;

use rand::thread_rng;
use rand::distributions::{Distribution};

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Constants {
	pub batch_interval: u64,
	pub num_investors: u64,
	pub num_makers: u64,
	pub block_size: usize,
	pub num_blocks: u64,
	pub market_type: MarketType,
	pub front_run_perc: f64,
	pub flow_order_offset: f64,
	pub maker_prop_delay: u64,
	pub maker_base_spread: f64,
	pub maker_enter_prob: f64,
	pub max_held_inventory: f64,
	pub maker_inv_tax: f64,
}

impl Constants {
	pub fn new(b_i: u64, n_i: u64, n_m: u64, b_s: usize, n_b: u64, 
		m_t: MarketType, f_r: f64, f_o_o: f64, m_p_d: u64, t_s: f64, 
		mep: f64, mhi: f64, mit: f64) -> Constants {
		Constants {
			batch_interval: b_i,
			num_investors: n_i,
			num_makers: n_m,
			block_size: b_s,
			num_blocks: n_b,
			market_type: m_t,
			front_run_perc: f_r,
			flow_order_offset: f_o_o,
			maker_prop_delay: m_p_d,
			maker_base_spread: t_s,
			maker_enter_prob: mep,
			max_held_inventory: mhi,
			maker_inv_tax: mit,
		}
	}

	pub fn log(&self) -> String {
		let h = format!("\nbatch_interval,num_investors,num_makers,block_size,num_blocks,market_type,front_run_perc,flow_order_offset,maker_prop_delay,maker_base_spread,maker_enter_prob,max_held_inventory,maker_inv_tax,");
		let d = format!("{},{},{},{},{},{:?},{},{},{},{},{},{},{},",
			self.batch_interval,
			self.num_investors,
			self.num_makers,
			self.block_size,
			self.num_blocks,
			self.market_type,
			self.front_run_perc,
			self.flow_order_offset,
			self.maker_prop_delay,
			self.maker_base_spread,
			self.maker_enter_prob,
			self.max_held_inventory,
			self.maker_inv_tax);
		format!("{}\n{}", h, d)
	}
}

#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
pub enum DistType {
	Uniform,
	Normal,
	Poisson,
	Exponential,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum DistReason {
	AsksCenter,
	BidsCenter,
	MinerFrontRun,
	InvestorVolume,
	MinerFrameForm,
	PropagationDelay,
	InvestorGas,
	InvestorEnter,
	MakerType,
	MakerInventory,
	MakerBalance,
	MakerOrderVolume,
	InvestorBalance,
	InvestorInventory,
}

const NUM_DISTS: usize = DistReason::InvestorInventory as usize + 1;

// Each distribution is in the form (µ: f64, std_dev: f64, scalar: f64, DistType)
#[derive(Debug, Deserialize, Clone)]
pub struct Distributions {
	pub dists: Vec<(f64, f64, f64, DistType)>,
}


impl Distributions {
	// Takes in a configuration vector of (DistReason, v1: f64, v2: f64, scalar: f64, DistType),
	// Indexes the dists array by the DistReason
	pub fn new(config: Vec<(DistReason, f64, f64, f64, DistType)>) -> Distributions {
		assert!(config.len() > 0);
		// initialize the vec to be same size as number of distreasons
		let mut v = vec![(0.0, 0.0, 0.0, DistType::Uniform); NUM_DISTS];
		for entry in config {
			v[entry.0 as usize] = (entry.1, entry.2, entry.3, entry.4.clone());
		}
		Distributions {
			dists: v,
		}
	}

	// Samples from a uniform distribution, based on supplied params
	pub fn sample_uniform(low: f64, high: f64, scalar: Option<f64>) -> f64 {
		if let Some(scalar) = scalar {
			Distributions::sample(low, high, scalar, DistType::Uniform)
		} else {
			Distributions::sample(low, high, 1.0, DistType::Uniform)
		}
	}

	// Samples from a normal distribution, based on supplied params
	pub fn sample_normal(mean: f64, std_dev: f64, scalar: Option<f64>) -> f64 {
		if let Some(scalar) = scalar {
			Distributions::sample(mean, std_dev, scalar, DistType::Normal)
		} else {
			Distributions::sample(mean, std_dev, 1.0, DistType::Normal)
		}
	}

	// Samples from a poisson distribution, based on supplied params
	pub fn sample_poisson(lambda: f64, scalar: Option<f64>) -> f64 {
		if let Some(scalar) = scalar {
			Distributions::sample(lambda, lambda, scalar, DistType::Poisson) 
		} else {
			Distributions::sample(lambda, lambda, 1.0, DistType::Poisson)
		}
	}


	// Samples the distribution based on the config for the respsective DistReason
	pub fn sample_dist(&self, which_dist: DistReason) -> Option<f64> {
		// Get the config: (f64, f64, DistType) from our list of configs
		if let Some(_config) = self.dists.get(which_dist as usize) {
			Some(Distributions::sample(_config.0, _config.1, _config.2, _config.3.clone()))
		} else {
			None
		}
	}

	// Samples the distribution based on the config for the respsective DistReason
	pub fn read_dist_params(&self, which_dist: DistReason) -> (f64, f64) {
		// Get the config: (f64, f64, DistType) from our list of configs
		let dist_entry = self.dists.get(which_dist as usize).expect("read_dist_params");
		let v1 = dist_entry.0;
		let v2 = dist_entry.1;
		(v1, v2)
	}

	pub fn fifty_fifty() -> bool {
		let val = rand::distributions::Uniform::new(0.0, 1.0).sample(&mut thread_rng());
		if val > 0.50 {
			return true;
		} else {
			return false;
		}
	}

	// ex: prob = 0.10 -> 10% chance true, 90% chance false
	pub fn do_with_prob(prob: f64) -> bool {
		assert!(prob <= 1.0);
		assert!(prob >= 0.0);

		let val = rand::distributions::Uniform::new(0.0, 1.0).sample(&mut thread_rng());
		if val <= prob {
			return true;
		} else {
			return false;
		}
	}

	// Normal:  v1 = mean, v2 = std_dev
	// Uniform: v1 = low, v2 = high
	// Poisson: v1 = lambda, v2 = lambda
	// Exp:		v1 = lambda, v2 = lambda
	pub fn sample(v1: f64, v2: f64, scalar: f64, dtype: DistType) -> f64 {
		match dtype {
			DistType::Uniform => 	 scalar * rand::distributions::Uniform::new(v1, v2).sample(&mut thread_rng()),
			DistType::Normal =>  	 scalar * rand::distributions::Normal::new(v1, v2).sample(&mut thread_rng()),
			DistType::Poisson => 	 scalar * rand::distributions::Poisson::new(v1).sample(&mut thread_rng()) as f64,
			DistType::Exponential => scalar * rand::distributions::Exp::new(v1).sample(&mut thread_rng()),
		}
	}
}


#[cfg(test)]
mod tests {
	use crate::simulation::simulation_config::{DistReason, DistType, Distributions};

	#[test]
	fn test_index_by_enum() {
		let a = vec!(5,6,7);
		assert_eq!(&5, a.get(DistReason::AsksCenter as usize).unwrap());
		assert_eq!(&6, a.get(DistReason::BidsCenter as usize).unwrap());
		assert_eq!(&7, a.get(DistReason::MinerFrontRun as usize).unwrap());
	}


	#[test]
	fn test_config() {
		let v = vec!(
		(DistReason::AsksCenter, 110.0, 20.0, 1.0, DistType::Normal),
		(DistReason::BidsCenter, 90.0, 20.0, 1.0, DistType::Normal),
		(DistReason::MinerFrontRun, 0.0, 1.0, 1.0, DistType::Uniform),
		(DistReason::MinerFrameForm, 50.0, 20.0, 1.0, DistType::Normal),
		(DistReason::PropagationDelay, 20.0, 5.0, 1.0, DistType::Normal),
		(DistReason::InvestorGas, 0.0, 1.0, 1.0, DistType::Uniform),
		(DistReason::InvestorEnter, 50.0, 50.0, 1.0, DistType::Poisson),
		(DistReason::MakerType, 0.0, 4.0, 1.0, DistType::Uniform),
		(DistReason::MakerInventory, 0.0, 100.0, 1.0, DistType::Uniform),
		(DistReason::MakerBalance, 50.0, 100.0, 1.0, DistType::Uniform),
		);

		let d = Distributions::new(v);

		let d_conf = d.dists.get(DistReason::AsksCenter as usize).unwrap();
		assert_eq!(d_conf.0, 110.0);
		assert_eq!(d_conf.1, 20.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Normal);

		let d_conf = d.dists.get(DistReason::BidsCenter as usize).unwrap();
		assert_eq!(d_conf.0, 90.0);
		assert_eq!(d_conf.1, 20.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Normal);

		let d_conf = d.dists.get(DistReason::MinerFrontRun as usize).unwrap();
		assert_eq!(d_conf.0, 0.0);
		assert_eq!(d_conf.1, 1.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Uniform);

		let d_conf = d.dists.get(DistReason::MinerFrameForm as usize).unwrap();
		assert_eq!(d_conf.0, 50.0);
		assert_eq!(d_conf.1, 20.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Normal);

		let d_conf = d.dists.get(DistReason::PropagationDelay as usize).unwrap();
		assert_eq!(d_conf.0, 20.0);
		assert_eq!(d_conf.1, 5.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Normal);

		let d_conf = d.dists.get(DistReason::InvestorGas as usize).unwrap();
		assert_eq!(d_conf.0, 0.0);
		assert_eq!(d_conf.1, 1.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Uniform);

		let d_conf = d.dists.get(DistReason::InvestorEnter as usize).unwrap();
		assert_eq!(d_conf.0, 50.0);
		assert_eq!(d_conf.1, 50.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Poisson);

		let d_conf = d.dists.get(DistReason::MakerType as usize).unwrap();
		assert_eq!(d_conf.0, 0.0);
		assert_eq!(d_conf.1, 4.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Uniform);

		let d_conf = d.dists.get(DistReason::MakerInventory as usize).unwrap();
		assert_eq!(d_conf.0, 0.0);
		assert_eq!(d_conf.1, 100.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Uniform);

		let d_conf = d.dists.get(DistReason::MakerBalance as usize).unwrap();
		assert_eq!(d_conf.0, 50.0);
		assert_eq!(d_conf.1, 100.0);
		assert_eq!(d_conf.2, 1.0);
		assert_eq!(d_conf.3, DistType::Uniform);

	}
}















