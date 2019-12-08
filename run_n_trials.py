import os





def main():
	exp_name = raw_input("Enter experiment name: \n")

	n = raw_input("Enter number of trials: \n")

	dists = raw_input("Enter <your_distributions_config_name>.csv: \n")

	consts = raw_input("Enter name of <your_consts_config_name>.csv (of any market type): \n")

	call = "cargo run --example copy_csvs {}".format(consts)
	os.system(call)

	for i in xrange(0, int(n)):

		call = "cargo run {}_{}_KLF {}.csv {}_KLF.csv".format(i, exp_name, dists, consts)
		os.system(call)

		call = "cargo run {}_{}_FBA {}.csv {}_FBA.csv".format(i, exp_name, dists, consts)
		os.system(call)

		call = "cargo run {}_{}_CDA {}.csv {}_CDA.csv".format(i, exp_name, dists, consts)
		os.system(call)
	



if __name__ == "__main__":
	main()