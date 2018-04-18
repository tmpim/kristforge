#include <iostream>
#include <thread>

#include "network.h"

extern const char _binary_kristforge_cl_start, _binary_kristforge_cl_end;

int main() {
	std::string cl(&_binary_kristforge_cl_start, &_binary_kristforge_cl_end - &_binary_kristforge_cl_start);

	std::cout << cl << std::endl;

	std::cout << "Hello, World!" << std::endl;

	kristforge::network::Options opts;
	opts.verbose = true;

	opts.onConnect = [] {
		std::cout << "Connected" << std::endl;
	};

	auto state = std::make_shared<kristforge::State>();

	std::thread net([&] {
		kristforge::network::run("https://krist.ceriat.net/ws/start", state, opts);
	});

	std::thread ping([&] {
		while (true) {
			std::this_thread::sleep_for(std::chrono::seconds(5));
			std::cout << "Pinging" << std::endl;

			state->pushSolution(kristforge::Solution(kristforge::Target("0123456789ab", 100), "k5ztameslf", "aaaa"));
		}
	});

	std::string it = "0123456789ab";

	std::cout << std::to_string(it.size()) << std::endl;

	net.join();
	ping.join();
}