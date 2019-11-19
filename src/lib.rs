pub mod io;
pub mod exchange;
pub mod simulation;
pub mod order;
pub mod controller;
pub mod utility;
pub mod blockchain;
pub mod players;

use crate::order::order_book::Book;
use crate::order::order::TradeType;
use crate::blockchain::mem_pool::MemPool;
use crate::controller::State;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate log;
extern crate log4rs;

extern crate statrs;

use std::sync::{Mutex, Arc};

// use crate::libmath;

pub fn setup_exchange() -> (Arc<MemPool>, Arc<Book>, Arc<Book>, Arc<Mutex<State>>) {
	let queue = Arc::new(MemPool::new());
	let bids_book = Arc::new(Book::new(TradeType::Bid));
	let asks_book = Arc::new(Book::new(TradeType::Ask));
	(queue, bids_book, asks_book, Arc::new(Mutex::new(State::Process)))
}

















