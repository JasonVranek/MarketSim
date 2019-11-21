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


fn main() {
	// setup_logging("test.log");
	// info!("poop");

	// Initialize the Exchange
	// let (queue, bids_book, asks_book, state) = flow_rs::setup_exchange();

	// Create a new Controller to dispatch our tasks
	// let mut controller = Controller::new();
    
	// create a task run an auction every batch_interval (milliseconds)
	// let batch_interval = 3000;
	// let auction_task = Auction::async_auction_task(Arc::clone(&bids_book), 
	// 	                          Arc::clone(&asks_book), 
	// 	                          Arc::clone(&state), batch_interval);
	// controller.push(auction_task);

	// create a task that processes order queue every queue_interval (milliseconds)
	// let queue_interval = 10;
	// let queue_task = MemPoolProcessor::async_queue_task(Arc::clone(&queue), 
	// 	                                             Arc::clone(&bids_book), 
	// 	                                             Arc::clone(&asks_book),
	// 	                                             Arc::clone(&state),
	// 	                                             queue_interval);
	// controller.push(queue_task);

	// // Spawn the tcp server task that listens for incoming orders in JSON format
	// let tcp_server = tcp_listener(Arc::clone(&queue), format!("127.0.0.1:5000"));
	// controller.push(tcp_server);


	// // Spawn the websocket server thread that listens for incoming orders in JSON format
	// let address: &'static str = "127.0.0.1:3015";
	// let _ws_server = ws_listener(Arc::clone(&queue), &address);
	
	// // Loop forever asynchronously running tasks
	// controller.run();


	// Create a new Controller to dispatch our tasks
	let mut controller = Controller::new();

	let distributions = parse_config_csv().expect("Couldn't parse config");
	let consts = Constants {
			batch_interval: 300,
			num_investors: 5,
			num_makers: 0,
			block_size: 1000,
			market_type: MarketType::FBA,
			front_run_perc: 1.0,
		};

	// Initial state of the sim
	let (simulation, miner) = Simulation::init_simulation(distributions, consts.clone());

	

	let mut shit = Controller::new();
	let miner_task = Simulation::miner_task(miner, simulation.dists.clone(), 
												   Arc::clone(&simulation.house), 
												   Arc::clone(&simulation.mempool),
												   Arc::clone(&simulation.bids_book),
												   Arc::clone(&simulation.asks_book), 
												   consts.clone());
	shit.push(miner_task);

	let mut controller = Vec::new();

	let investor_task = Simulation::investor_task(simulation.dists.clone(), 
												  Arc::clone(&simulation.house),
												  Arc::clone(&simulation.mempool), 
												  consts.market_type);
	controller.push(investor_task);


	let investor_task2 = Simulation::investor_task(simulation.dists.clone(), 
												  Arc::clone(&simulation.house),
												  Arc::clone(&simulation.mempool), 
												  MarketType::CDA);
	controller.push(investor_task2);

	shit.run();

	for h in controller {
		h.join().unwrap();
	}



	// Loop forever asynchronously running tasks
	// controller.run();
}









