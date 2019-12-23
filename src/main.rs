extern crate flow_rs;
extern crate tokio;

use flow_rs::simulation::simulation_config::{DistReason};
use flow_rs::simulation::simulation_history::UpdateReason;
use flow_rs::controller::Controller;
use flow_rs::simulation::simulation::{Simulation};
use flow_rs::simulation::config_parser::*;


use flow_rs::utility::{setup_logging, get_time, setup_log_headers};
use flow_rs::{log_order_book, log_player_data, log_mempool_data, log_results};


#[macro_use]
extern crate log;
extern crate log4rs;

use std::collections::HashMap;
use log::{log, Level};
use std::sync::Arc;
use std::env;

fn main() {
	// Get the log file names
	let mut args = env::args();
	assert!(args.len() > 0);
	args.next(); // consume file name arg[0]
	let filename = match args.next() {
		Some(arg) => arg,
		None => {
			println!("Supply log file!");
			std::process::exit(1);
		}
	};

	let dists_name = match args.next() {
		Some(arg) => arg,
		None => {
			println!("Supply distributions csv file!");
			std::process::exit(1);
		}
	};

	let consts_name = match args.next() {
		Some(arg) => arg,
		None => {
			println!("Supply consts csv file!");
			std::process::exit(1);
		}
	};

	let enable_log: bool = match args.next() {
		Some(arg) => {
			if arg.to_lowercase() == "n" {
				false
			} else {
				true
			}
		},
		None => {
			true	// logging enabled by default
		},
	};

	// Initialize the logger
	let _logger_handle = setup_logging(&filename, enable_log);

	// Create a new Controller to dispatch our tasks
	let mut controller = Controller::new();

	// Create a vector to hold the handles to the threads
	let mut thread_handles = Vec::new();

	// Read the distribution parameters from the supplied csv file (arg2)
	let distributions = parse_dist_config_csv(format!("configs/{}", dists_name)).expect("Couldn't parse dists config");

	// Read the constant parameters from the supplied csv file (arg3)
	let consts = parse_consts_config_csv(format!("configs/{}", consts_name)).expect(&format!("Couldn't parse consts config {}", consts_name));

	// Write the headers to all of the log files
	setup_log_headers(consts.market_type.clone());    

	// Initial state of the sim
	let (simulation, miner) = Simulation::init_simulation(distributions, consts.clone());

	// Log and save the intial state of the players
	simulation.house.log_all_players(UpdateReason::Initial);
	// Save the initial balance and inventory of each player
	let mut initial_player_state = HashMap::<String, (f64, f64)>::new(); 
	{
		for (id, player) in simulation.house.players.lock().unwrap().iter() {
			initial_player_state.insert(id.clone(), (player.get_bal(), player.get_inv()));
		}
	}
	
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
	
	controller.start_task(miner_task);

	// Wait for investor task to finish
	for h in thread_handles {
		h.join().unwrap();
	}

	// End the tasks
	controller.shutdown();


	info!("Done running simulation. Saving data...");

	println!("{:?}", simulation.house.gas_fees);

	// Log the final state of the players
	simulation.house.log_all_players(UpdateReason::Final);

	// Calculate the fundamental value from the configs
	let (mean_bids, _dev_bids) = simulation.dists.read_dist_params(DistReason::BidsCenter);
	let (mean_asks, _dev_asks) = simulation.dists.read_dist_params(DistReason::AsksCenter);
	let fund_val = (mean_bids + mean_asks) / 2.0;
	println!("fund_val: {}", fund_val);

	

	let s = format!("Experiment ending at: {:?}", get_time());
	log_order_book!(s);
	log_mempool_data!(s);
	log_player_data!(s);

	// Calculate the pre liquidation performance results
	let res = simulation.calc_performance_results(fund_val, initial_player_state.clone());
	log_results!(format!("{:?},NO,{}", consts.market_type, res));

	// Each player transacts all non-zero inventory at the fundamental value
	simulation.house.liquidate(fund_val);

	// Calculate the post liquidation performance results
	let res = simulation.calc_performance_results(fund_val, initial_player_state);
	log_results!(format!("{:?},YES,{}", consts.market_type, res));

}











