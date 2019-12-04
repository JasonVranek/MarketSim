extern crate flow_rs;
extern crate tokio;

use flow_rs::exchange::MarketType;
use flow_rs::simulation::simulation_config::Constants;
use flow_rs::controller::Controller;
use flow_rs::simulation::simulation::{Simulation};
use flow_rs::utility::setup_logging;
use flow_rs::simulation::config_parser::*;

#[macro_use]
extern crate log;
extern crate log4rs;

use std::sync::Arc;
use std::env;

fn main() {
	let mut args = env::args();
	assert!(args.len() > 0);
	args.next(); // consume file name
	let filename = match args.next() {
		Some(arg) => arg,
		None => {
			println!("Supply log file!");
			std::process::exit(1);
		}
	};

	// Initialize the logger
	let _logger_handle = setup_logging(&filename);

	// Create a new Controller to dispatch our tasks
	let mut controller = Controller::new();

	// Create a vector to hold the handles to the threads
	let mut thread_handles = Vec::new();


	let distributions = parse_config_csv().expect("Couldn't parse config");

	let consts = Constants {
			batch_interval: 500,
			num_investors: 100,
			num_makers: 5,
			block_size: 1000,
			num_blocks: 1,
			market_type: MarketType::KLF,
			front_run_perc: 1.0,
			flow_order_offset: 5.0,
			maker_prop_delay: 200,	// 200 ms delay after block for makers to act
			tick_size: 1.0,
			maker_enter_prob: 0.25,
		};


	// Initial state of the sim
	let (simulation, miner) = Simulation::init_simulation(distributions, consts.clone());

	
	// Initialize an investor thread to repeat at intervals based on supplied distributions
	let investor_task = Simulation::investor_task(simulation.dists.clone(), 
												  Arc::clone(&simulation.house),
												  Arc::clone(&simulation.mempool),
												  Arc::clone(&simulation.history), 
												  Arc::clone(&simulation.block_num), 
												  consts.clone());

	thread_handles.push(investor_task);


	// Initialize an maker task to repeat to be repeated on a fixed interval
	let maker_task = Simulation::maker_task(simulation.dists.clone(), 
												  Arc::clone(&simulation.house),
												  Arc::clone(&simulation.mempool), 
												  Arc::clone(&simulation.history), 
												  Arc::clone(&simulation.block_num), 
												  consts.clone());

	// controller.push(maker_task);
	controller.start_task(maker_task);


	// Initalize a miner task to be repeated on a fixed interval
	let miner_task = Simulation::miner_task(miner, simulation.dists.clone(), 
												   Arc::clone(&simulation.house), 
												   Arc::clone(&simulation.mempool),
												   Arc::clone(&simulation.bids_book),
												   Arc::clone(&simulation.asks_book), 
												   Arc::clone(&simulation.history),
												   Arc::clone(&simulation.block_num), 
												   consts.clone());
	
	// controller.push(miner_task);
	controller.start_task(miner_task);

	// controller.run();

	// controller.start_tasks();

	for h in thread_handles {
		h.join().unwrap();
	}

	controller.shutdown();


	info!("Done running simulation. Saving data...");

	// Loop forever asynchronously running tasks
	// controller.run();
}









