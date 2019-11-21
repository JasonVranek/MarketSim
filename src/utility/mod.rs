use crate::players::TraderT;
use std::time::{Duration, SystemTime};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use std::iter;

// use log::LevelFilter;
// use log4rs::append::file::FileAppender;
// use log4rs::encode::pattern::PatternEncoder;
// use log4rs::config::{Appender, Config, Root};

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


/// Generate a random trader id from random ascii chars
pub fn gen_rand_trader_id() -> String {
	let mut rng = thread_rng();
	let id: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(10)
        .collect();
    id

    // let index = rng.gen_range(0, 3);

    // match index {
    	// 0 => format!("MKR{}", id),
    	// 1 => format!("MIN{}", id),
    	// _ => format!("INV{}", id),
    // }
}


// pub fn setup_logging(logfile: &str) {
//     let logfile = FileAppender::builder()
//         .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
//         .build(format!("log/{}", logfile)).expect("Couldn't set up appender");

//     // Use builder instead of yaml file
//     let config = Config::builder()
//         .appender(Appender::builder().build("logfile", Box::new(logfile)))
//         .build(Root::builder()
//                    .appender("logfile")
//                    .build(LevelFilter::Info)).expect("Couldn't set up builder");

//     log4rs::init_config(config).expect("Couldn't config");

//     info!("Setup Logger @{:?}", get_time());

// }
