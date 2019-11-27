use std::sync::Mutex;
use crate::simulation::simulation_config::{Constants, Distributions, DistReason};
use crate::controller::Task;
use crate::exchange::clearing_house::ClearingHouse;
use crate::order::order::{Order, TradeType, ExchangeType, OrderType};
use crate::order::order_book::Book;
use crate::blockchain::mem_pool::MemPool;
use crate::players::TraderT;
use crate::players::miner::Miner;
use crate::players::investor::Investor;
use crate::players::maker::Maker;
use crate::exchange::MarketType;
use crate::blockchain::order_processor::OrderProcessor;
use crate::utility::gen_trader_id;
use crate::simulation::simulation_history::History;


use std::sync::Arc;
use std::{time, thread};
use std::thread::JoinHandle;


pub struct BlockNum {pub num: Mutex<u64>}
impl BlockNum {
	pub fn new() -> BlockNum {
		BlockNum {
			num: Mutex::new(0),
		}
	}

	pub fn inc_count(&self) {
		*self.num.lock().unwrap() + 1;
	}
}


pub struct Simulation {
	pub dists: Distributions,
	pub consts: Constants,
	pub house: Arc<ClearingHouse>,
	pub mempool: Arc<MemPool>,
	pub bids_book: Arc<Book>,
	pub asks_book: Arc<Book>,
	pub history: Arc<History>,
	pub block_num: Arc<BlockNum>,
}



impl Simulation {
	pub fn new(dists: Distributions, consts: Constants, house: ClearingHouse, 
			   mempool: MemPool, bids_book: Book, asks_book: Book, history: History) -> Simulation {
		Simulation {
			dists: dists,
			consts: consts,
			house: Arc::new(house),
			mempool: Arc::new(mempool),
			bids_book: Arc::new(bids_book),
			asks_book: Arc::new(asks_book),
			history: Arc::new(history),
			block_num: Arc::new(BlockNum::new()),
		}
	}

	pub fn init_simulation(dists: Distributions, consts: Constants) -> (Simulation, Miner) {
		// Initialize the state for the simulation
		let house = ClearingHouse::new();
		let bids_book = Book::new(TradeType::Bid);
		let asks_book = Book::new(TradeType::Ask);
		let mempool = MemPool::new();
		let history = History::new(consts.market_type);

		// Initialize and register the miner to CH
		let ch_miner = Miner::new(gen_trader_id(TraderT::Miner));
		let miner_id = ch_miner.trader_id.clone();
		house.reg_miner(ch_miner);

		// Initialize copy of miner for the miner task
		let mut miner = Miner::new(gen_trader_id(TraderT::Miner));
		miner.trader_id = miner_id;

		// Initialize and register the Investors
		let invs = Simulation::setup_investors(&dists, &consts);
		house.reg_n_investors(invs);

		// Initialize and register the Makers
		let mkrs = Simulation::setup_makers(&dists, &consts);
		house.reg_n_makers(mkrs);
		
		(Simulation::new(dists, consts, house, mempool, bids_book, asks_book, history), miner)
	}

	/// Initializes Investor players. Randomly samples the maker's initial balance and inventory
	/// using the distribution configs. Number of makers saved in consts.
	pub fn setup_investors(dists: &Distributions, consts: &Constants) -> Vec<Investor> {
		let mut invs = Vec::new();
		for _ in 1..consts.num_investors {
			let mut i = Investor::new(gen_trader_id(TraderT::Investor));
			if let Some(bal) = dists.sample_dist(DistReason::InvestorBalance) {
				i.balance = bal;
			} else {
				panic!("Couldn't setup investor balance");
			}
			if let Some(inv) = dists.sample_dist(DistReason::InvestorInventory) {
				i.inventory = inv;
			} else {
				panic!("Couldn't setup investor inventory");
			}
			invs.push(i);
		}
		invs
	}

	/// Initializes Maker players. Randomly samples the maker's initial balance and inventory
	/// using the distribution configs. Number of makers saved in consts.
	pub fn setup_makers(dists: &Distributions, consts: &Constants) -> Vec<Maker> {
		let mut mkrs = Vec::new();
		for _ in 1..consts.num_makers {
			let mut m = Maker::new(gen_trader_id(TraderT::Maker));
			if let Some(bal) = dists.sample_dist(DistReason::MakerBalance) {
				m.balance = bal;
			} else {
				panic!("Couldn't setup maker balance");
			}
			if let Some(inv) = dists.sample_dist(DistReason::MakerInventory) {
				m.inventory = inv;
			} else {
				panic!("Couldn't setup maker inventory");
			}
			mkrs.push(m);
		}
		mkrs
	}

	/// A repeating task. Will randomly select an Investor from the ClearingHouse,
	/// generate a bid/ask order priced via bid/ask distributions, send the order to 
	/// the mempool, and then sleep until the next investor_arrival time.
	pub fn investor_task(dists: Distributions, house: Arc<ClearingHouse>, mempool: Arc<MemPool>, history: Arc<History>, consts: Constants) -> JoinHandle<()> {
		// Task::rpt_task(move || {
		thread::spawn(move || {       
			loop {
				println!("In inv task: {:?}", consts.market_type);
				// Randomly select an investor
				let trader_id = house.get_rand_player_id(TraderT::Investor).expect("Couldn't get rand investor");

				// Decide bid or ask
				let trade_type = match Distributions::fifty_fifty() {
					0 => TradeType::Ask,
					_ => TradeType::Bid,
				};

				// Sample order price from bid/ask distribution
				let price = match trade_type {
					TradeType::Ask => dists.sample_dist(DistReason::AsksCenter).expect("couldn't sample price"),
					TradeType::Bid => dists.sample_dist(DistReason::BidsCenter).expect("couldn't sample price"),
				};

				// Sample order volume from bid/ask distribution
				let quantity = dists.sample_dist(DistReason::InvestorVolume).expect("couldn't sample vol");

				// Determine if were using flow or limit order
				let ex_type = match consts.market_type {
					MarketType::CDA|MarketType::FBA => ExchangeType::LimitOrder,
					MarketType::KLF => ExchangeType::FlowOrder,
				};

				// Set the p_low and p_high to the price for limit orders
				let (p_l, p_h) = match ex_type {								
					ExchangeType::LimitOrder => (price, price),
					ExchangeType::FlowOrder => {
						// How to calculate flow order price?
						match trade_type {
							TradeType::Ask => (price, price + consts.flow_order_offset),
							TradeType::Bid => (price - consts.flow_order_offset, price),
						}
					}
				};

				// Generate the order
				let order = Order::new(trader_id.clone(), 
									   OrderType::Enter,
							   	       trade_type,
								       ex_type,
								       p_l,
								       p_h,
								       price,
								       quantity,
								       dists.sample_dist(DistReason::InvestorGas).expect("Couldn't sample gas")
				);

				// Add the order to the ClearingHouse which will register to the correct investor
				match house.new_order(order.clone()) {
					Ok(()) => {
						// println!("{:?}", order);
						// Add the order to the simulation's history
						history.mempool_order(order.clone());
						// Send the order to the MemPool
						OrderProcessor::conc_recv_order(order, Arc::clone(&mempool)).join().expect("Failed to send inv order");
						
					},
					Err(e) => {
						// If we failed to add the order to the player, don't send it to mempool
						println!("{:?}", e);
					},
				}

				// Sample from InvestorEnter distribution how long to wait to send next investor
				let sleep_time = dists.sample_dist(DistReason::InvestorEnter).expect("Couldn't get enter time sample").abs();	
				let sleep_time = time::Duration::from_millis(sleep_time as u64);
				thread::sleep(sleep_time);
			}
		})
	}

	pub fn miner_task(mut miner: Miner, dists: Distributions, house: Arc<ClearingHouse>, 
		mempool: Arc<MemPool>, bids: Arc<Book>, asks: Arc<Book>, history: Arc<History>, block_num: Arc<BlockNum>, consts: Constants) -> Task {
		println!("out miner task");
		Task::rpt_task(move || {
			println!("in miner task");
			
			// Publish the miner's current frame
			if let Some(vec_results) = miner.publish_frame(Arc::clone(&bids), Arc::clone(&asks), consts.market_type) {
				// Save new book state to the history
				history.clone_book_state(bids.copy_orders(), TradeType::Bid, *block_num.num.lock().unwrap());
				history.clone_book_state(asks.copy_orders(), TradeType::Bid, *block_num.num.lock().unwrap());
				block_num.inc_count();
				for res in vec_results {
					// Update the clearing house and history
					history.save_results(res.clone());
					house.update_house(res);
				}
			}

			// Sleep for miner frame delay to simulate multiple miners
			let sleep_time = dists.sample_dist(DistReason::MinerFrameForm).expect("Couldn't get miner frame form delay").abs();	
			let sleep_time = time::Duration::from_millis(sleep_time as u64);
			thread::sleep(sleep_time);

			// Make the next frame after simulated propagation delay expires
			miner.make_frame(Arc::clone(&mempool), consts.block_size);

			// Miner will front-run with some probability: 
			let sample = dists.sample_dist(DistReason::MinerFrontRun).expect("Couldn't get miner front run delay");	
			if sample <= consts.front_run_perc {
				match miner.front_run() {
					Ok(order) => {
						println!("Miner inserted a front-run order: {}", order.order_id);
						// Log the order as if it were sent to the mempool
						history.mempool_order(order.clone());

						// Register the new order to the ClearingHouse
						house.new_order(order).expect("Couldn't add front-run order to CH");
						
					},
					Err(e) => {
						println!("{:?}", e);
					}
				}
			}

			// Wait until the next block publication time

		}, consts.batch_interval)
	}



	pub fn maker_task(dists: Distributions, house: Arc<ClearingHouse>, mempool: Arc<MemPool>, history: Arc<History>, consts: Constants) -> Task {
		println!("out maker task");
		Task::rpt_task(move || {
			println!("in maker task");
			// Select all Makers
			let maker_ids = house.get_filtered_ids(TraderT::Maker);

			for trader_id in maker_ids {
				// Decide bid or ask
				let trade_type = match Distributions::fifty_fifty() {
					0 => TradeType::Ask,
					_ => TradeType::Bid,
				};

				// Sample order price from bid/ask distribution
				let price = match trade_type {
					TradeType::Ask => dists.sample_dist(DistReason::AsksCenter).expect("couldn't sample price"),
					TradeType::Bid => dists.sample_dist(DistReason::BidsCenter).expect("couldn't sample price"),
				};

				// Sample order volume from bid/ask distribution
				let quantity = dists.sample_dist(DistReason::InvestorVolume).expect("couldn't sample vol");

				// Determine if were using flow or limit order
				let ex_type = match consts.market_type {
					MarketType::CDA|MarketType::FBA => ExchangeType::LimitOrder,
					MarketType::KLF => ExchangeType::FlowOrder,
				};

				// Set the p_low and p_high to the price for limit orders
				let (p_l, p_h) = match ex_type {								
					ExchangeType::LimitOrder => (price, price),
					ExchangeType::FlowOrder => {
						// How to calculate flow order price?
						match trade_type {
							TradeType::Ask => (price, price + consts.flow_order_offset),
							TradeType::Bid => (price - consts.flow_order_offset, price),
						}
					}
				};

				// Generate the order
				let order = Order::new(trader_id.clone(), 
									   OrderType::Enter,
							   	       trade_type,
								       ex_type,
								       p_l,
								       p_h,
								       price,
								       quantity,
								       dists.sample_dist(DistReason::InvestorGas).expect("Couldn't sample gas")
				);


				// Add the order to the ClearingHouse which will register to the correct investor
				match house.new_order(order.clone()) {
					Ok(()) => {
						// println!("{:?}", order);
						// Add the order to the simulation's history
						history.mempool_order(order.clone());
						// Send the order to the MemPool
						OrderProcessor::conc_recv_order(order, Arc::clone(&mempool)).join().expect("Failed to send inv order");
						
					},
					Err(e) => {
						// If we failed to add the order to the player, don't send it to mempool
						println!("{:?}", e);
					},
				}

			}
			
		}, consts.batch_interval + consts.maker_prop_delay)
	}



	pub fn maker_task2(dists: Distributions, house: Arc<ClearingHouse>, mempool: Arc<MemPool>, history: Arc<History>, consts: Constants) -> Task {
		println!("out maker task");
		Task::rpt_task(move || {
			println!("in maker task");
			// Select all Makers
			let maker_ids = house.get_filtered_ids(TraderT::Maker);

			// Copy the current mempool
			let pool;
			{
				let mempool = mempool.items.lock().expect("maker task pool");
				pool = mempool.clone();

			}

			// use History to produce inference and decision data
			let (decision_data, inference_data) = history.produce_data(pool);

			// iterate through each maker and produce an order using the decision and inference data
			for id in maker_ids {
				// Each maker interprets the data to produce their order based on their type 
				// get mutable reference to the maker
				let maker_order = house.maker_new_order(id.clone());

				// register the order to the clearing house

				// register the order to the history
			}

			
			
		}, consts.batch_interval + consts.maker_prop_delay)
	}
}














