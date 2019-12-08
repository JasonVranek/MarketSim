extern crate flow_rs;

use flow_rs::exchange::MarketType;
use flow_rs::simulation::simulation_config::{Constants, DistReason};
use flow_rs::simulation::config_parser::*;


#[macro_use]
extern crate log;
extern crate log4rs;

use log::{log, Level};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() {
	// Get the const file name
	let mut args = env::args();
	assert!(args.len() > 0);
	args.next(); // consume file name arg[0]
	let consts_name = match args.next() {
		Some(arg) => arg,
		None => {
			println!("Supply consts csv file!");
			std::process::exit(1);
		}
	};

	let consts = parse_consts_config_csv(format!("configs/{}.csv", consts_name)).expect(&format!("Couldn't parse consts config {}", consts_name));

	let(cda, fba, klf) = consts.copy3();

	let mut cda_file = File::create(format!("configs/{}_CDA.csv", consts_name)).expect("Already made cda config");
	let mut fba_file = File::create(format!("configs/{}_FBA.csv", consts_name)).expect("Already made fba config");
	let mut klf_file = File::create(format!("configs/{}_KLF.csv", consts_name)).expect("Already made klf config");

	write!(cda_file, "{}", cda).expect("Error copying cda file");
	write!(fba_file, "{}", fba).expect("Error copying fba file");
	write!(klf_file, "{}", klf).expect("Error copying klf file");

}

	





