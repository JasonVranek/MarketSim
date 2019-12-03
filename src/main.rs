extern crate flow_rs;
extern crate tokio;

use flow_rs::exchange::MarketType;
use flow_rs::simulation::simulation_config::Constants;
// use flow_rs::io::ws_json::ws_listener;
// use flow_rs::io::tcp_json::tcp_listener;
use flow_rs::controller::Controller;
use flow_rs::simulation::simulation::{Simulation};
// use flow_rs::utility::setup_logging;

use std::sync::Arc;
use flow_rs::simulation::config_parser::*;

pub struct BlockNum{num:Arc<u64>}

fn main() {
	// Create a new Controller to dispatch our tasks
	let mut controller = Controller::new();


	// Create a vector to hold the handles to the threads
	let mut thread_handles = Vec::new();


	let distributions = parse_config_csv().expect("Couldn't parse config");

	let consts = Constants {
			batch_interval: 3000,
			num_investors: 100,
			num_makers: 5,
			block_size: 1000,
			market_type: MarketType::KLF,
			front_run_perc: 1.0,
			flow_order_offset: 5.0,
			maker_prop_delay: 200,	// 200 ms delay after block for makers to act
		};


	// Initial state of the sim
	let (simulation, miner) = Simulation::init_simulation(distributions, consts.clone());

	
	// Initialize an investor thread to repeat at intervals based on supplied distributions
	let investor_task = Simulation::investor_task(simulation.dists.clone(), 
												  Arc::clone(&simulation.house),
												  Arc::clone(&simulation.mempool),
												  Arc::clone(&simulation.history), 
												  consts.clone());

	thread_handles.push(investor_task);

	// Initialize an maker task to repeat to be repeated on a fixed interval
	let maker_task = Simulation::maker_task(simulation.dists.clone(), 
												  Arc::clone(&simulation.house),
												  Arc::clone(&simulation.mempool), 
												  Arc::clone(&simulation.history), 
												  consts.clone());

	controller.push(maker_task);

	// Initialize an maker task to repeat to be repeated on a fixed interval
	let maker_task2 = Simulation::maker_task2(simulation.dists.clone(), 
												  Arc::clone(&simulation.house),
												  Arc::clone(&simulation.mempool), 
												  Arc::clone(&simulation.history), 
												  consts.clone());

	controller.push(maker_task2);


	// Initalize a miner task to be repeated on a fixed interval
	let miner_task = Simulation::miner_task(miner, simulation.dists.clone(), 
												   Arc::clone(&simulation.house), 
												   Arc::clone(&simulation.mempool),
												   Arc::clone(&simulation.bids_book),
												   Arc::clone(&simulation.asks_book), 
												   Arc::clone(&simulation.history),
												   Arc::clone(&simulation.block_num), 
												   consts.clone());
	
	controller.push(miner_task);

	controller.run();

	for h in thread_handles {
		h.join().unwrap();
	}



	// Loop forever asynchronously running tasks
	// controller.run();
}









