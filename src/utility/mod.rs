use crate::exchange::MarketType;
use crate::players::TraderT;
use std::time::{Duration, SystemTime};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use std::iter;

use log::{LevelFilter, Level};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root, Logger};


#[macro_export]
macro_rules! log_order_book {
    ($message:expr) => {
        log!(target: "app::order_books", Level::Warn, "{}", $message);
    }   
}

#[macro_export]
macro_rules! log_player_data {
    ($message:expr) => {
        log!(target: "app::player_data", Level::Warn, "{}", $message);
    }   
}

#[macro_export]
macro_rules! log_mempool_data {
    ($message:expr) => {
        log!(target: "app::mempool_data", Level::Warn, "{}", $message);
    }   
}

#[macro_export]
macro_rules! log_results {
    ($message:expr) => {
        log!(target: "app::results", Level::Warn, "{}", $message);
    }   
}


pub fn get_time() -> Duration {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                         .expect("SystemTime::duration_since failed")
}


// Generate a random 64b order id
pub fn gen_order_id() -> u64 {
    let mut rng = thread_rng();
    let p: u64 = rng.gen();
    p

}

pub fn gen_rand_f64() -> f64 {
     let mut rng = thread_rng();
    let p: f64 = rng.gen();
    p
}

/// Generate a trader id for a specific type of trader
pub fn gen_trader_id(tt: TraderT) -> String {
	let mut rng = thread_rng();
	let id: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(10)
        .collect();

    match tt {
    	TraderT::Maker => format!("MKR{}", id),
    	TraderT::Investor => format!("INV{}", id),
    	TraderT::Miner => format!("MIN{}", id),
    }
}



pub fn setup_logging(file_name: &str, enable_log: bool) -> log4rs::Handle {
    let stdout = ConsoleAppender::builder().build();

    let order_books_name;
    let player_data_name;
    let mempool_data_name;

    match enable_log {
        true => {
            order_books_name = format!("log/order_books_{}.csv", file_name);
            player_data_name = format!("log/player_data_{}.csv", file_name);
            mempool_data_name = format!("log/mempool_data_{}.csv", file_name);
        },
        false => {
            // Write logs to /dev/null if logging is disabled
            order_books_name = format!("/dev/null");
            player_data_name = format!("/dev/null");
            mempool_data_name = format!("/dev/null");
        },
    }
    
    let results_name = format!("log/results.csv");

    let order_books_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{m}\n")))
        .build(order_books_name).expect("Couldn't set up appender");

    let player_data_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{m}\n")))
        .build(player_data_name).expect("Couldn't set up appender");

    let mempool_data_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{m}\n")))
        .build(mempool_data_name).expect("Couldn't set up appender");

    let results_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{m}\n")))
        .build(results_name).expect("Couldn't set up appender");


    // Use builder instead of yaml file
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("order_books", Box::new(order_books_file)))
        .appender(Appender::builder().build("player_data", Box::new(player_data_file)))
        .appender(Appender::builder().build("mempool_data", Box::new(mempool_data_file)))
        .appender(Appender::builder().build("results", Box::new(results_file)))
        // the logger for the order book data. use log!(target: "app::order_books", Level::Warn, "message here");
        .logger(Logger::builder()       
            .appender("order_books")
            .additive(false)
            .build("app::order_books", LevelFilter::Info))
          // the logger for the player data. use log!(target: "app::player_data", Level::Warn, "message here");
        .logger(Logger::builder()
            .appender("player_data")
            .additive(false)
            .build("app::player_data", LevelFilter::Info))
         // the logger for the mempool data. use log!(target: "app::mempool_data", Level::Warn, "message here");
        .logger(Logger::builder()
            .appender("mempool_data")
            .additive(false)
            .build("app::mempool_data", LevelFilter::Info))
        .logger(Logger::builder()
            .appender("results")
            .additive(false)
            .build("app::results", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .expect("Couldn't set up builder");


    let handle = log4rs::init_config(config).expect("Couldn't config");

    info!("Setup Logger @{:?}", get_time());
    
    handle
}



// Write the headers to the csv logs
pub fn setup_log_headers(market_type: MarketType) {
    // Setup the logfile headers
    log_player_data!(format!("time,reason,trader_id,player_type,balance,inventory,orders,"));
    log_mempool_data!(format!("time,trader_id,order_id,order_type,trade_type,ex_type,p_low,p_high,price,quantity,gas,"));

    match market_type {
        MarketType::CDA => {
            log_order_book!("time,new_order_trader_id,new_order_order_id,new_order_order_type,new_order_trade_type,new_order_ex_type,new_order_p_low,new_order_p_high,new_order_price,new_order_quantity,new_order_gas,bids_after,asks_after");
        },
        _ => log_order_book!(format!("time,block_num,book_type,clearing_price,book_before,book_after,")),
    }
}















