use crate::order::order::Order;
use crate::players::TraderT;
use crate::utility::gen_trader_id;
use crate::blockchain::mem_pool::MemPool;
use crate::blockchain::mempool_processor::MemPoolProcessor;
use crate::order::order_book::Book;
use crate::exchange::MarketType;
use crate::exchange::exchange_logic::{Auction, AuctionResults};

use std::sync::{Mutex, Arc};

/// A struct for the Miner player. 
pub struct Miner {
	pub trader_id: String,
	pub orders: Mutex<Vec<Order>>,
	pub frame: Vec<Order>,
	pub balance: f64,
	pub inventory: f64,
}

impl Miner {
	pub fn new(bal: f64, inv:f64) -> Miner {
		Miner {
			trader_id: gen_trader_id(TraderT::Miner),
			orders: Mutex::new(Vec::<Order>::new()),
			frame: Vec::<Order>::new(),
			balance: bal,
			inventory: inv,

		}
	}


	/// Miner grabs ≤ block_size orders from the MemPool to construct frame for next block
	/// sorted by gas price
	pub fn make_frame(&mut self, pool: Arc<MemPool>, block_size: usize) {
		let size = pool.length();
		if size == 0 {
			println!("No orders to grab from MemPool!");
			return
		}
		// Sort orders in the MemPool in decreasing order by gas price
		pool.sort_by_gas();

		if size <= block_size {
			self.frame = pool.pop_all();
		} 
		else {
			self.frame = pool.pop_n(block_size);
		}
	}

	/// 'Publishes' the Miner's frame by sequentially executing the orders in the frame
	pub fn publish_frame(&mut self, bids: Arc<Book>, asks: Arc<Book>, m_t: MarketType) -> Option<AuctionResults> {
		let handles = MemPoolProcessor::seq_process_orders(&mut self.frame, Arc::clone(&bids), Arc::clone(&asks), m_t.clone());
		for h in handles {
			h.join().expect("Failed to publish...");
		}

		// Run auction after book has been updated (CDA is prcessed in seq_process_orders)
		Auction::run_auction(bids, asks, m_t)
	}


}