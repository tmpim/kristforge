#include <iostream>
#include <thread>

#include "network.h"

int main() {
	kristforge::network::Options opts;
	opts.verbose = true;

	opts.onConnect = [] {
		std::cout << "Connected" << std::endl;
	};

	auto state = std::make_shared<kristforge::State>();

	std::thread net([&] {
		kristforge::network::run("https://krist.ceriat.net/ws/start", state, opts);
	});

	net.join();
}