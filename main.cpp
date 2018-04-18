#include <iostream>
#include <thread>

#include "network.h"

extern const char _binary_kristforge_cl_start, _binary_kristforge_cl_end;

int main() {
	std::string cl(&_binary_kristforge_cl_start, &_binary_kristforge_cl_end - &_binary_kristforge_cl_start);

	std::cout << cl << std::endl;

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