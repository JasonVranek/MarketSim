use crate::simulation::simulation_config::{Distributions, Constants};
use crate::simulation::simulation_config::{DistType, DistReason};


use std::io;
use std::error::Error;
use serde::Deserialize;
use csv;

#[derive(Debug, Deserialize)]
struct TempDist{
	reason: DistReason,
	v1: f64,
	v2: f64,
	scalar: f64,
	dist_type: DistType,
}

impl TempDist {
	pub fn unpack(&mut self) -> (DistReason, f64, f64, f64, DistType){
		(self.reason, self.v1, self.v2, self.scalar, self.dist_type)
	}
}


pub fn parse_config_csv() -> Result<Distributions, Box<dyn Error>> {
    let mut lines: Vec<(DistReason, f64, f64, f64, DistType)> = Vec::new();
    let mut rdr = csv::Reader::from_reader(io::stdin());
    println!("Reading in config file...");
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let mut csv_line: TempDist = result?;
        println!("{:?}", csv_line);
        lines.push(csv_line.unpack());

    }
    Ok(Distributions::new(lines))
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parser() {
		parse_config_csv();
		assert_eq!(1, 2);
	}
}