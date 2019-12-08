import os





def main():
	exp_name = raw_input("Enter experiment name: \n")

	n = raw_input("Enter number of trials: \n")

	dists = raw_input("Enter <your_distributions_config_name>.csv: \n")

	consts = raw_input("Enter name of <your_consts_config_name>.csv (of any market type): \n")

	os.system("mkdir log")

	call = "cargo run --example copy_csvs {}".format(consts)
	os.system(call)


	# write the header to the total_results.csv file
	header = "market type,liquidated?,fund val,total gas,avg gas,total tax,maker profit,investor profit,miner profit,dead weight,volatility,rmsd,aggressive mkr prof,riskaverse mkr prof,random mkr profit,\n"
	f = open("log/total_results.csv", "w")# write header to total_results.csv
	f.write(header)
	f.close()


	for i in xrange(0, int(n)):

		call = "cargo run {}_{}_KLF {}.csv {}_KLF.csv".format(i, exp_name, dists, consts)
		os.system(call)

		call = "cargo run {}_{}_FBA {}.csv {}_FBA.csv".format(i, exp_name, dists, consts)
		os.system(call)

		call = "cargo run {}_{}_CDA {}.csv {}_CDA.csv".format(i, exp_name, dists, consts)
		os.system(call)
	

	os.system("mkdir results")
	os.system("mkdir results/{}".format(exp_name))
	os.system("mkdir results/{}/log".format(exp_name))
	os.system("mv log/total_results.csv results/{}/{}_total_results.csv".format(exp_name, exp_name))
	os.system("mv log/* results/{}/log".format(exp_name))
	os.system("cp configs/{}.csv results/{}".format(dists, exp_name))
	os.system("cp configs/{}.csv results/{}".format(consts, exp_name))



if __name__ == "__main__":
	main()