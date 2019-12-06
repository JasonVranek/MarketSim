extern crate flow_rs;
extern crate tokio;

use flow_rs::exchange::MarketType;
use flow_rs::simulation::simulation_config::Constants;
use flow_rs::simulation::simulation_history::UpdateReason;
use flow_rs::controller::Controller;
use flow_rs::simulation::simulation::{Simulation};
use flow_rs::simulation::config_parser::*;


use flow_rs::utility::{setup_logging, get_time};
use flow_rs::{log_order_book, log_player_data, log_mempool_data};


#[macro_use]
extern crate log;
extern crate log4rs;

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

	// Initialize the logger
	let _logger_handle = setup_logging(&filename);

	// Create a new Controller to dispatch our tasks
	let mut controller = Controller::new();

	// Create a vector to hold the handles to the threads
	let mut thread_handles = Vec::new();


	let distributions = parse_config_csv().expect("Couldn't parse config");

	let consts = Constants {
			batch_interval: 500,
			num_investors: 10,
			num_makers: 5,
			block_size: 1000,
			num_blocks: 10,
			market_type: MarketType::KLF,
			front_run_perc: 1.0,
			flow_order_offset: 5.0,
			maker_prop_delay: 200,	// 200 ms delay after block for makers to act
			tick_size: 1.0,
			maker_enter_prob: 0.25,
			max_held_inventory: 1000.0,
			maker_inv_tax: 0.01,
		};

	
	setup_log_headers(&consts);    


	// Initial state of the sim
	let (simulation, miner) = Simulation::init_simulation(distributions, consts.clone());

	// Log the intial state of the players
	simulation.house.log_all_players(UpdateReason::Initial);

	
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

	for h in thread_handles {
		h.join().unwrap();
	}

	controller.shutdown();


	info!("Done running simulation. Saving data...");

	println!("{:?}", simulation.house.gas_fees);

	// Log the intial state of the players
	simulation.house.log_all_players(UpdateReason::Final);

	let s = format!("Experiment ending at: {:?}", get_time());
	log_order_book!(s);
	log_mempool_data!(s);
	log_player_data!(s);

}


fn setup_log_headers(consts: &Constants) {
	// Setup the logfile headers
	log_player_data!(format!("time,reason,trader_id,player_type,balance,inventory,orders,"));
    log_mempool_data!(format!("time,trader_id,order_id,order_type,trade_type,ex_type,p_low,p_high,price,quantity,gas,"));

    match consts.market_type {
    	MarketType::CDA => {
    		log_order_book!("time,new_order_trader_id,new_order_order_id,new_order_order_type,new_order_trade_type,new_order_ex_type,new_order_p_low,new_order_p_high,new_order_price,new_order_quantity,new_order_gas,bids_after,asks_after");
    	},
    	_ => log_order_book!(format!("time,block_num,book_type,clearing_price,book_before,book_after,")),
    }
}








