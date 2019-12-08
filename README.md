# MarketSim
Continuous Double Auction, Frequent Batch Auctions, and "Continuous" Scaled Limit Order simulation. 

The purpose of this project is to compare single vs uniform prices in a "decentralized" environment.



### Usage
- Setup Rust: <https://www.rust-lang.org/tools/install>
- Make sure binary is compiled to your operating system, with "cargo build".
- Setup distributions file in configs/ folder. Example dist shown inside configs/ folder.
- Setup consts file in the same way


- Running a series of trials:
- python run_n_trials.py
	- Enter experiment name:  test_exp
	- Enter number of trials: 5
	- Enter <\your_distributions_config_name>.csv: test_dists
	- Enter name of <\your_consts_config_name>.csv (of any market type): test_consts

The above inputs will run 15 total trials of the same configs, 5 of each market type (CDA, FBA, KLF). The distribution parameters are loaded from the file: configs/test_dists.csv and the constant parameters are loaded from configs/test_consts.csv . (Note that you omit the .csv and path when responding to python input. 
The cummulative results of all 15 trials will be stored in results/test_exp_total_results.csv


If just a single trial of a specified market type wishes to be run:
cargo run test_exp test_dists.csv test_consts.csv 

(Note to include the .csv this time.) This will load the file test_dists.csv from the config folder and the test_consts.csv file from the config directory. The specified market type in the consts file is what will be run. The results will be the file log/total_results.csv 



