use crate::simulation::simulation_config::{Constants, DistType, Distributions, DistReason};
use crate::controller::Task;
use crate::exchange::clearing_house::ClearingHouse;
use crate::order::order::{Order, TradeType, ExchangeType, OrderType};
use crate::order::order_book::Book;
use crate::blockchain::mem_pool::MemPool;
use crate::players::TraderT;
use crate::players::Player;
use crate::players::miner::Miner;
use crate::players::investor::Investor;
use crate::players::maker::Maker;
use crate::exchange::MarketType;
use crate::blockchain::order_processor::OrderProcessor;
use crate::utility::gen_trader_id;


use std::sync::Arc;
use std::{time, thread};




pub struct Simulation {
	pub dists: Arc<Distributions>,
	pub consts: Arc<Constants>,
	pub house: Arc<ClearingHouse>,
	pub mempool: Arc<MemPool>,
	pub bids_book: Arc<Book>,
	pub asks_book: Arc<Book>,
}



impl Simulation {
	pub fn new(dists: Arc<Distributions>, consts: Arc<Constants>, house: Arc<ClearingHouse>, 
			   mempool: Arc<MemPool>, bids_book: Arc<Book>, asks_book: Arc<Book>) -> Simulation {
		Simulation {
			dists: dists,
			consts: consts,
			house: house,
			mempool: mempool,
			bids_book: bids_book,
			asks_book: asks_book,
		}
	}

	pub fn init_simulation(config: Vec<(DistReason, f64, f64, f64, DistType)>, consts: Constants) -> Simulation {
		// Read the config to setup the distributions
		let dists = Arc::new(Distributions::new(config));

		// Initialize the state for the simulation
		let house = Arc::new(ClearingHouse::new());
		let bids_book = Arc::new(Book::new(TradeType::Bid));
		let asks_book = Arc::new(Book::new(TradeType::Ask));
		let mempool = Arc::new(MemPool::new());

		// Initialize and register the miner
		let miner = Miner::new(gen_trader_id(TraderT::Miner));
		house.reg_miner(miner);

		// Initialize and register the Investors
		let invs = Simulation::setup_investors(&dists, &consts);
		house.reg_n_investors(invs);

		// Initialize and register the Makers
		let mkrs = Simulation::setup_makers(&dists, &consts);
		house.reg_n_makers(mkrs);
		
		Simulation::new(dists, Arc::new(consts), house, mempool, bids_book, asks_book)
	}

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
	pub fn investor_task(dists: Arc<Distributions>, house: Arc<ClearingHouse>, mempool: Arc<MemPool>, market_type: MarketType) -> Task {
		Task::new(move || {
			loop {
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
				let ex_type = match market_type {
					MarketType::CDA|MarketType::FBA => ExchangeType::LimitOrder,
					MarketType::KLF => ExchangeType::FlowOrder,
				};

				// Set the p_low and p_high to the price for limit orders
				let (p_l, p_h) = match ex_type {								
					ExchangeType::LimitOrder => (price, price),
					ExchangeType::FlowOrder => {
						// How to calculate flow order price?
						(0.0, 0.0)
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
						// Send the order to the MemPool
						OrderProcessor::conc_recv_order(order, Arc::clone(&mempool)).join().expect("Failed to send inv order");
					},
					Err(e) => {
						// If we failed to add the order to the player, don't send it to mempool
						println!("{:?}", e);
					},
				}

				// Sample from InvestorEnter distribution how long to wait to send next investor
				let sleep_time = dists.sample_dist(DistReason::InvestorGas).expect("Couldn't get enter time sample");	
				let sleep_time = time::Duration::from_millis(sleep_time as u64);
				thread::sleep(sleep_time);
			}
		})
	}

	pub fn miner_task(mut miner: Miner, dists: Arc<Distributions>, house: Arc<ClearingHouse>, 
		mempool: Arc<MemPool>, bids: Arc<Book>, asks: Arc<Book>, market_type: MarketType, consts: &'static Constants) -> Task {
		Task::rpt_task(move || {
			// Publish the miner's current frame
			if let Some(results) = miner.publish_frame(Arc::clone(&bids), Arc::clone(&asks), market_type.clone(), Arc::clone(&house)) {
				// Update the clearing house
				match market_type {
					MarketType::FBA => house.fba_batch_update(results),
					MarketType::KLF => house.flow_batch_update(results),
					MarketType::CDA => {},
				}
			}
			// Sleep for miner frame delay to simulate multiple miners
			let sleep_time = dists.sample_dist(DistReason::MinerFrameForm).expect("Couldn't get miner frame form delay");	
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

	pub fn maker_task() -> Task {
		unimplemented!();
	}
}














