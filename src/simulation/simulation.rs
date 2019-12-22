use crate::simulation::simulation_config::{Constants, Distributions, DistReason};
use crate::controller::Task;
use crate::exchange::clearing_house::ClearingHouse;
use crate::order::order::{Order, TradeType, ExchangeType, OrderType};
use crate::order::order_book::Book;
use crate::blockchain::mem_pool::MemPool;
use crate::players::{TraderT};
use crate::players::miner::Miner;
use crate::players::investor::Investor;
use crate::players::maker::{Maker, MakerT};
use crate::exchange::MarketType;
use crate::blockchain::order_processor::OrderProcessor;
use crate::utility::{gen_trader_id, get_time};
use crate::simulation::simulation_history::History;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Arc;
use std::{time, thread};
use std::thread::JoinHandle;

use log::{Level};


pub struct BlockNum {pub num: Mutex<u64>}
impl BlockNum {
	pub fn new() -> BlockNum {
		BlockNum {
			num: Mutex::new(0),
		}
	}

	pub fn inc_count(&self) {
		let mut count = self.num.lock().unwrap();
		*count += 1;
	}

	pub fn read_count(&self) -> u64 {
		*self.num.lock().unwrap()
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
	pub fn setup_investors(_dists: &Distributions, consts: &Constants) -> Vec<Investor> {
		let mut invs = Vec::new();
		for _ in 1..consts.num_investors {
			let i = Investor::new(gen_trader_id(TraderT::Investor));
			// if let Some(bal) = dists.sample_dist(DistReason::InvestorBalance) {
			// 	i.balance = bal;
			// } else {
			// 	panic!("Couldn't setup investor balance");
			// }
			// if let Some(inv) = dists.sample_dist(DistReason::InvestorInventory) {
			// 	i.inventory = inv;
			// } else {
			// 	panic!("Couldn't setup investor inventory");
			// }
			invs.push(i);
		}
		invs
	}

	/// Initializes Maker players. Randomly samples the maker's initial balance and inventory
	/// using the distribution configs. Number of makers saved in consts.
	pub fn setup_makers(_dists: &Distributions, consts: &Constants) -> Vec<Maker> {
		let mut mkrs = Vec::new();
		for _ in 1..consts.num_makers {
			// random id
			let id = gen_trader_id(TraderT::Maker);
			// random behavioral type for strategy
			let maker_type = Maker::gen_rand_type();

			let m = Maker::new(id, maker_type);
			// if let Some(bal) = dists.sample_dist(DistReason::MakerBalance) {
			// 	m.balance = bal;
			// } else {
			// 	panic!("Couldn't setup maker balance");
			// }
			// if let Some(inv) = dists.sample_dist(DistReason::MakerInventory) {
			// 	m.inventory = inv;
			// } else {
			// 	panic!("Couldn't setup maker inventory");
			// }
			mkrs.push(m);
		}
		mkrs
	}

	/// A repeating task. Will randomly select an Investor from the ClearingHouse,
	/// generate a bid/ask order priced via bid/ask distributions, send the order to 
	/// the mempool, and then sleep until the next investor_arrival time.
	pub fn investor_task(dists: Distributions, house: Arc<ClearingHouse>, mempool: Arc<MemPool>, history: Arc<History>, block_num: Arc<BlockNum>, consts: Constants) -> JoinHandle<()> {
		thread::spawn(move || {       
			loop {
				// Check if the simulation is ending
				if block_num.read_count() > consts.num_blocks {
					// exit the thread
					println!("Exiting investor_task");
					break;
				}

				// Randomly select an investor
				let trader_id = house.get_rand_player_id(TraderT::Investor).expect("Couldn't get rand investor");

				// Only add a new order if they dont already have one in the book
				if house.get_player_order_count(&trader_id) == 0 {
					// Decide bid or ask
					let trade_type = match Distributions::fifty_fifty() {
						true => TradeType::Ask,
						false => TradeType::Bid,
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

				// Sample from InvestorEnter distribution how long to wait to send next investor
				let sleep_time = dists.sample_dist(DistReason::InvestorEnter).expect("Couldn't get enter time sample").abs();	
				let sleep_time = time::Duration::from_millis(sleep_time as u64);
				thread::sleep(sleep_time);
			}
		})
	}

	pub fn miner_task(mut miner: Miner, dists: Distributions, house: Arc<ClearingHouse>, 
		mempool: Arc<MemPool>, bids: Arc<Book>, asks: Arc<Book>, history: Arc<History>, block_num: Arc<BlockNum>, consts: Constants) -> Task {
		Task::rpt_task(move || {
			// println!("in miner task, {:?}", block_num.read_count());
			
			// Check if the simulation is ending
			if block_num.read_count() > consts.num_blocks {
				// exit the thread
				println!("Exiting miner_task");
				// std::process::exit(1)
			}

			// Collect the gas from the frame
			let (gas_changes, total_gas) = miner.collect_gas();
			// Update the players' gas amounts
			house.apply_gas_fees(gas_changes, total_gas);

			// Publish the miner's current frame
			if let Some(vec_results) = miner.publish_frame(Arc::clone(&bids), Arc::clone(&asks), consts.market_type) {
				let copied_bids = bids.copy_orders();
				let copied_asks = asks.copy_orders();

				let clearing_price = vec_results.last().expect("vec_results").uniform_price;
				log_order_book!(format!("{:?},{},{:?},{:?},{:?},",
					get_time(),
					block_num.read_count(),
					clearing_price,
					copied_bids,
					copied_asks,
					));

				// Save new book state to the history
				history.clone_book_state(copied_bids, TradeType::Bid, *block_num.num.lock().unwrap());
				history.clone_book_state(copied_asks, TradeType::Ask, *block_num.num.lock().unwrap());

				for res in vec_results {
					// Update the clearing house and history
					history.save_results(res.clone());
					house.update_house(res);
				}
			}

			// Update the block num
			block_num.inc_count();

			// Tax the makers holding inventory
			house.tax_makers(consts.maker_inv_tax);


			// Sleep for miner frame delay to simulate multiple miners
			let sleep_time = dists.sample_dist(DistReason::MinerFrameForm).expect("Couldn't get miner frame form delay").abs();	
			let sleep_time = time::Duration::from_millis(sleep_time as u64);
			thread::sleep(sleep_time);

			// Make the next frame after simulated propagation delay expires
			miner.make_frame(Arc::clone(&mempool), consts.block_size);

			// Miner will front-run with some probability: 
			match Distributions::do_with_prob(consts.front_run_perc) {
				true => {
					let (best_bid_price, best_ask_price) = history.get_best_prices();
					match miner.strategic_front_run(best_bid_price, best_ask_price) {
						Ok(order) => {
							println!("Miner inserted a front-run order: {}", order.order_id);
							// Log the order as if it were sent to the mempool
							history.mempool_order(order.clone());

							// Register the new order to the ClearingHouse
							house.new_order(order).expect("Couldn't add front-run order to CH");
							
						},
						Err(_e) => {
							// println!("{:?}", _e);
						}
					}
				}
				false => {},
			}

			// Wait until the next block publication time

		}, consts.batch_interval)
	}


	pub fn maker_task(dists: Distributions, house: Arc<ClearingHouse>, mempool: Arc<MemPool>, history: Arc<History>, block_num: Arc<BlockNum>, consts: Constants) -> Task {
		Task::rpt_task(move || {

			// Check if the simulation is ending
			if block_num.read_count() > consts.num_blocks {
				// exit the thread
				println!("Exiting maker_task");
				// std::process::exit(1)
			}

			// Select all Makers
			let maker_ids = house.get_filtered_ids(TraderT::Maker);

			// Copy the current mempool
			let pool;
			{
				pool = mempool.items.lock().expect("maker task pool").clone();
			}

			// use History to produce inference and decision data
			let (decision_data, inference_data) = history.produce_data(pool);


			// iterate through each maker and produce an order using the decision and inference data
			for id in maker_ids {
				// Wait until the maker_cold_start number of blocks has passed before entering orders to 
				// allow more information to arrive from investors.
				if block_num.read_count() < consts.maker_cold_start {break;}
				// Only make new orders if the maker currently has none in the book
				if house.get_player_order_count(&id) != 0 {continue;}
				// Randomly choose whether the maker should try to trade this block
				match Distributions::do_with_prob(consts.maker_enter_prob) {
					true => {},
					false => continue,	// Don't trade this batch
				}

				// Each maker interprets the data to produce their pair of orders based on their type 
				if let Some((bid_order, ask_order)) = house.maker_new_orders(id.clone(), &decision_data, &inference_data, &dists, &consts) {
					// Add the order to the ClearingHouse which will register to the correct maker
					match house.new_order(bid_order.clone()) {
						Ok(()) => {
							// Add the bid_order to the simulation's history
							history.mempool_order(bid_order.clone());
							// Send the bid_order to the MemPool
							OrderProcessor::conc_recv_order(bid_order, Arc::clone(&mempool)).join().expect("Failed to send maker bid order");
							
						},
						Err(e) => {
							// If we failed to add the order to the player, don't send it to mempool
							println!("{:?}", e);
						},
					}

					// Add the order to the ClearingHouse which will register to the correct maker
					match house.new_order(ask_order.clone()) {
						Ok(()) => {
							// println!("{:?}", ask_order);
							// Add the ask_order to the simulation's history
							history.mempool_order(ask_order.clone());
							// Send the ask_order to the MemPool
							OrderProcessor::conc_recv_order(ask_order, Arc::clone(&mempool)).join().expect("Failed to send maker ask order");
							
						},
						Err(e) => {
							// If we failed to add the ask_order to the player, don't send it to mempool
							println!("{:?}", e);
						},
					}
				}	
			}
			// Wait until the next batch + maker propagation delay to rerun the maker task
		}, consts.batch_interval + consts.maker_prop_delay)
	}

	// Calculates performance metrics for the simulation and returns a CSV formatted string of the results
	// init_player_s = a hashmap of the initial player balances and inventories
	// fund_val: the fixed fundamental value for the simulation
	pub fn calc_performance_results(&self, fund_val: f64, init_player_s: HashMap<String, (f64, f64)>) -> String {
		let volatility = self.calc_price_volatility();
		let rmsd = self.calc_rmsd(fund_val);
		let (maker_profit, investor_profit, miner_profit) = self.calc_total_profit(init_player_s);
		let (total_gas, avg_gas, total_tax, dead_weight) = self.calc_social_welfare(maker_profit, investor_profit, miner_profit);
		
		// The cummulative profit made by all of the makers
		let mkr_profits = self.house.maker_profits.lock().unwrap();
		// The cummulative profits made by all Aggressive type makers
		let agg_profit = mkr_profits[MakerT::Aggressive as usize];
		// The cummulative profits made by all the RiskAverse type makers
		let riskav_profit = mkr_profits[MakerT::RiskAverse as usize];
		// The cummulative profits made by all the Random type makers
		let rand_profit = mkr_profits[MakerT::Random as usize];
		// The number of each type of maker in the simulation
		let (num_agg, num_riska, num_rand) = self.house.get_maker_counts();

		let (inv_welf, mkr_welf, min_welf) = self.calc_welfare();

		format!("{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},", fund_val, total_gas, avg_gas, total_tax, maker_profit, investor_profit, miner_profit, dead_weight, volatility, rmsd, agg_profit, riskav_profit, rand_profit, num_agg, num_riska, num_rand, inv_welf, mkr_welf, min_welf)
	}

	// standard deviation of transaction price differences relative to the fundamental value
	pub fn calc_rmsd(&self, fund_val: f64) -> f64{
		// Results saved in history.clearings
		let mut num = 0.0;
		let mut sum_of_diffs_squared = 0.0;
		let clearings = self.history.clearings.lock().unwrap();
		for (trade_results, _timestamp) in clearings.iter() {
			if trade_results.uniform_price.is_none() {
				// CDA look at price of each transaction
				match &trade_results.cross_results {
					Some(player_updates) => {
						for p_u in player_updates {
							let p = p_u.price;
							sum_of_diffs_squared += (p - fund_val).powi(2);
							num += 1.0;
						}
					},
					None => {},
				}
				
			} else {
				// FBA or KLF just need to look at uniform clearing price
				let p = trade_results.uniform_price.unwrap();
				sum_of_diffs_squared += (p - fund_val).powi(2);
				num += 1.0;
			}
		}

		assert!(num > 0.0);
		let mean = sum_of_diffs_squared / num;
		let rsmd = mean.sqrt();

		rsmd
	}

	// standard deviation of transaction price differences relative to different orders
	pub fn calc_price_volatility(&self) -> f64{
		// Results saved in history.clearings
		let mut num = 0.0;
		let mut mean = 0.0;
		let mut sum_of_diffs_squared = 0.0;
		let clearings = self.history.clearings.lock().unwrap();

		// calc avg
		for (trade_results, _timestamp) in clearings.iter() {
			if trade_results.uniform_price.is_none() {
				// CDA look at price of each transaction
				match &trade_results.cross_results {
					Some(player_updates) => {
						for p_u in player_updates {
							let p = p_u.price;
							mean += p;
							num += 1.0;
						}
					},
					None => {},
				}
				
			} else {
				// FBA or KLF just need to look at uniform clearing price
				let p = trade_results.uniform_price.unwrap();
				mean += p;
				num += 1.0;
			}
		}
		assert!(num > 0.0);	
		mean = mean / num;
		
		//calc std dev
		for (trade_results, _timestamp) in clearings.iter() {
			if trade_results.uniform_price.is_none() {
				// CDA look at price of each transaction
				match &trade_results.cross_results {
					Some(player_updates) => {
						for p_u in player_updates {
							let p = p_u.price;
							sum_of_diffs_squared += (p - mean).powi(2);
							num += 1.0;
						}
					},
					None => {},
				}
				
			} else {
				// FBA or KLF just need to look at uniform clearing price
				let p = trade_results.uniform_price.unwrap();
				sum_of_diffs_squared += (p - mean).powi(2);
				num += 1.0;
			}
		}

		assert!(num > 0.0);
		let mean = sum_of_diffs_squared / num;
		let volatility = mean.sqrt();

		volatility
	}


	pub fn calc_social_welfare(&self, maker_profit: f64, _investor_profit: f64, miner_profit: f64) -> (f64, f64, f64, f64) {
		// cummulative gas fees
		let avg_gas: f64;
		let mut total_gas = 0.0;
		{
			let all_gas = self.house.gas_fees.lock().unwrap();
			let mut num = 0.0;
			// Average all gas, ignore 0.0 entries where no orders were entered to block
			for g in all_gas.iter() {
				if g == &0.0 {continue;}
				total_gas += g;
				num += 1.0;
			}
			assert!(num > 0.0);
			avg_gas = total_gas / num;
		}

		// cummulative tax on maker inventory (Note, this is part of miner profits, so don't double count in social welfare)
		let total_tax = self.house.total_tax.lock().unwrap().clone();

		let dead_weight = total_gas + maker_profit + miner_profit;

		(total_gas, avg_gas, total_tax, dead_weight)
	}

	// Calculates the total profits final_bal - current_bal of each player
	// init_player_s = a hashmap of the initial player balances and inventories
	// returns (maker_profit, investor_profit, miner_profit)
	pub fn calc_total_profit(&self, init_player_s: HashMap<String, (f64, f64)>) -> (f64, f64, f64) {
		// Get final states
		let players = self.house.players.lock().unwrap();
		let mut investor_profit = 0.0;
		let mut maker_profit = 0.0;
		let mut miner_profit = 0.0;
		for (k, p) in players.iter() {
			match p.get_player_type() {
				TraderT::Maker => {
					// get initial bal and inv
					let (init_bal, _init_inv) = init_player_s.get(&k.clone()).expect("calc_total_profit");
					let cur_bal = p.get_bal();
					let _cur_inv = p.get_inv();
					let profit = cur_bal - init_bal;
					maker_profit += profit;
				},
				TraderT::Investor => {
					// get initial bal and inv
					let (init_bal, _init_inv) = init_player_s.get(&k.clone()).expect("calc_total_profit");
					// search current bal and inv
					let cur_bal = p.get_bal();
					let _cur_inv = p.get_inv();
					let profit = cur_bal - init_bal;
					investor_profit += profit;
				},
				TraderT::Miner => {
					// get initial bal and inv
					let (init_bal, _init_inv) = init_player_s.get(&k.clone()).expect("calc_total_profit");
					// search current bal and inv
					let cur_bal = p.get_bal();
					let _cur_inv = p.get_inv();
					let profit = cur_bal - init_bal;
					miner_profit += profit;
				},
			}
		}

		(maker_profit, investor_profit, miner_profit)
	}


	pub fn calc_welfare(&self) -> (f64, f64, f64){
		println!("in calc welfare");
		let history = &self.history;
		let house = &self.house;
		let txs = history.transactions.lock().unwrap();
		let pool = history.mempool_data.lock().unwrap();

		let mut inv_welf = 0.0;
		let mut mkr_welf = 0.0;
		let mut min_welf = 0.0;

		println!("{:?}", pool);

		println!("\n\n\n {:?}", txs);

		// For each transcaction 
		for tx in txs.iter() {
			// Find whether these tx were with an investor, maker, or miner
			let buyer_type = house.get_type(&tx.payer_id);
			let seller_type = house.get_type(&tx.vol_filler_id);
			let buyer_oid = tx.payer_order_id;
			let seller_oid = tx.vol_filler_order_id;

			let (bid_price, bid_plow, bid_phigh);

			// Get the price parameters from the original bid order
			match pool.get(&buyer_oid) {
				Some((order, _time)) => {	
					bid_price = order.price;
					bid_plow = order.p_low;
					bid_phigh = order.p_high;
				},
				None => continue,
			}

			let (ask_price, ask_plow, ask_phigh);
			// Get the price parameters from the original ask order
			match pool.get(&seller_oid) {
				Some((order, _time)) => {	
					ask_price = order.price;
					ask_plow = order.p_low;
					ask_phigh = order.p_high;
				},
				None => continue,
			}

			// Determine the amount of welfare gained from order
			match self.consts.market_type {
				MarketType::KLF => {
					println!("lalalalal");
					// Calculate the 
					// inv_welf += welfare;
				},
				MarketType::FBA|MarketType::CDA => {
					// Positive welfare if they bought at a lower price than they bid
					let welfare = (bid_price - tx.price) * tx.volume;
					println!("Bidder: {:?}{}, p_old: {}, p_tx: {}, welfare: {}", buyer_type, buyer_oid, bid_price, tx.price, welfare);
					match buyer_type {
						TraderT::Investor => {
							inv_welf += welfare;
						},
						TraderT::Maker => {
							mkr_welf += welfare;
						},
						TraderT::Miner => {
							min_welf += welfare;
						},
					}
					
					// Positive welfare if they sold at a higher price than they asked
					let welfare = (tx.price - ask_price) * tx.volume;
					println!("Asker: {:?}{}, p_old: {}, p_tx: {}, welfare: {}", seller_type, seller_oid, ask_price, tx.price, welfare);
					match seller_type {
						TraderT::Investor => {
							inv_welf += welfare;
						},
						TraderT::Maker => {
							mkr_welf += welfare;
						},
						TraderT::Miner => {
							min_welf += welfare;
						},
					}
				},
			}
		}

		(inv_welf, mkr_welf, min_welf)
	}

}




































