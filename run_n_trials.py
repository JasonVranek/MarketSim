import os





def main():
	exp_name = raw_input("Enter experiment name: \n")

	n = raw_input("Enter number of trials: \n")

	dists = raw_input("Enter <your_distributions_config_name>.csv: \n")

	consts = raw_input("Enter name of <your_consts_config_name>.csv (of any market type): \n")

	enable_log = raw_input("Log meta data (player data, mempool data, orderbook data)?  'y' or 'n' \n")

	os.system("mkdir log")

	call = "cargo run --example copy_csvs {}".format(consts)
	os.system(call)


	# write the header to the total_results.csv file
	header = "market type,liquidated?,fund val,total gas,avg gas,total tax,maker profit,investor profit,miner profit,dead weight,volatility,rmsd,aggressive mkr prof,riskaverse mkr prof,random mkr profit,num agg,num riska,num rand,inv_welf,mkr_welf,min_welf,\n"
	f = open("log/results.csv".format(exp_name), "w")# write header to total_results.csv
	f.write(header)
	f.close()


	for i in xrange(0, int(n)):

		call = "cargo run {}_{}_KLF {}.csv {}_KLF.csv {}".format(i, exp_name, dists, consts, enable_log)
		os.system(call)

		call = "cargo run {}_{}_FBA {}.csv {}_FBA.csv {}".format(i, exp_name, dists, consts, enable_log)
		os.system(call)

		call = "cargo run {}_{}_CDA {}.csv {}_CDA.csv {}".format(i, exp_name, dists, consts, enable_log)
		os.system(call)
	

	os.system("mkdir results")
	os.system("mkdir results/{}".format(exp_name))
	os.system("mkdir results/{}/log".format(exp_name))
	os.system("mv log/results.csv results/{}/".format(exp_name, exp_name))
	os.system("mv log/* results/{}/log".format(exp_name))
	os.system("cp configs/{}.csv results/{}".format(dists, exp_name))
	os.system("cp configs/{}.csv results/{}".format(consts, exp_name))



if __name__ == "__main__":
	main()