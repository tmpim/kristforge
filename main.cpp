#include "network.h"
#include "miner.h"
#include "utils.h"

#include <iostream>
#include <thread>
#include <vector>
#include <set>
#include <algorithm>
#include <random>
#include <atomic>
#include <tclap/CmdLine.h>

class AddressConstraint : public TCLAP::Constraint<std::string> {
public:
	std::string description() const override { return "krist address"; }

	std::string shortID() const override { return "krist address"; }

	bool check(const std::string &value) const override { return value.size() == 10; }
};

void printDeviceList() {
	const char *fmtString = "%-30.30s | %-15.15s | %-7.7s\n";
	printf(fmtString, "Device", "ID", "Score");
	std::vector devs = kristforge::getAllDevices();

	for (const cl::Device &d : devs) {
		auto devName = d.getInfo<CL_DEVICE_NAME>();
		auto id = kristforge::uniqueID(d);
		auto score = kristforge::scoreDevice(d);

		printf(fmtString, devName.data(), id.value_or("(n/a)").data(), std::to_string(score).data());
	}
}

struct DeviceComparator {
	bool operator()(const cl::Device &a, const cl::Device &b) const {
		return a() == b();
	}
};

std::string formatHashrate(long hashesPerSecond) {
	static const char *suffixes[] = {"h/s", "kh/s", "Mh/s", "Gh/s", "Th/s"};

	auto scale = std::max(0, static_cast<int>(0, log(hashesPerSecond) / log(1000)));
	double value = hashesPerSecond / pow(1000, scale);

	std::stringstream out;
	out << std::fixed << std::setprecision(2) << value << " " << suffixes[scale];
	return out.str();
}

int main(int argc, char **argv) {
	TCLAP::CmdLine cmd("Mine krist using OpenCL devices");

	// @formatter:off
	TCLAP::UnlabeledValueArg<std::string> addressArg("address", "Address to mine for", false, "k5ztameslf", new AddressConstraint, cmd);
	TCLAP::SwitchArg listDevicesArg("l", "list-devices", "List OpenCL devices and exit", cmd);
	TCLAP::SwitchArg allDevicesArg("a", "all-devices", "Use all OpenCL devices to mine", cmd);
	TCLAP::SwitchArg bestDeviceArg("b", "best-device", "Use best OpenCL device to mine", cmd);
	TCLAP::MultiArg<std::string> deviceIDsArg("d", "device-id", "Use OpenCL devices by ID to mine", false, "device id", cmd);
	TCLAP::MultiArg<int> deviceNumsArg("", "device-num", "Use OpenCL devices by position in list (not recommended)", false, "device num", cmd);
	TCLAP::ValueArg<std::string> kristNode("", "node", "Use custom krist node", false, "https://krist.ceriat.net/ws/start", "WS init url", cmd);
	TCLAP::ValueArg<int> vecsizeArg("V", "vector-width", "Manually set vector width for all devices", false, 1, "1 | 2 | 4 | 8 | 16", cmd);
	TCLAP::ValueArg<size_t> worksizeArg("w", "worksize", "Manually set work group size for all devices", false, 1, "size", cmd);
	TCLAP::SwitchArg onlyTestArg("t", "only-test", "Run tests on selected miners and then exit", cmd);
	TCLAP::ValueArg<std::string> clCompilerArg("", "cl-opts", "Extra options for the OpenCL compiler", false, "", "options", cmd);
	TCLAP::MultiSwitchArg verboseArg("v", "verbose", "Enable extra logging (can be repeated up to two times)", cmd);
	TCLAP::ValueArg<int> exitAfterArg("", "exit-after", "Stop after mining for given number of seconds", false, 0, "seconds", cmd);
	TCLAP::ValueArg<int> demoArg("", "demo", "Use a fake krist network with a fixed given work value", false, 10000, "work", cmd);
	TCLAP::ValueArg<int> prefixArg("", "prefix", "Prefix number (will be incremented for successive devices)", false, 0, "0-255", cmd);
	// @formatter:on

	cmd.parse(argc, argv);

	if (listDevicesArg.isSet()) {
		printDeviceList();
		return 0;
	}

	// collect selected devices
	std::vector allDevs = kristforge::getAllDevices();
	std::vector<cl::Device> selectedDevices;

	if (allDevicesArg.isSet()) {
		for (const cl::Device &d : allDevs) {
			selectedDevices.push_back(d);
		}
	}

	if (bestDeviceArg.isSet()) {
		auto best = std::max_element(allDevs.begin(), allDevs.end(), [](const cl::Device &a, const cl::Device &b) {
			return kristforge::scoreDevice(a) < kristforge::scoreDevice(b);
		});

		if (best == allDevs.end()) {
			throw std::range_error("No devices available");
		}

		selectedDevices.push_back(*best);
	}

	for (const std::string &id : deviceIDsArg.getValue()) {
		auto it = std::find_if(allDevs.begin(), allDevs.end(), [&id](const cl::Device &d) {
			return kristforge::uniqueID(d) == id;
		});

		if (it == allDevs.end()) {
			throw std::invalid_argument("No device with ID: " + id);
		}

		selectedDevices.push_back(*it);
	}

	for (const int n : deviceNumsArg.getValue()) {
		if (n > allDevs.size()) {
			throw std::range_error("Value out of range:" + n);
		}

		selectedDevices.push_back(allDevs[n - 1]);
	}

	std::cout << std::to_string(selectedDevices.size()) << " device(s) selected" << std::endl;

	if (selectedDevices.empty()) {
		std::cerr << "No devices selected" << std::endl;
		return 1;
	}

	uint8_t prefix = prefixArg.getValue(); // defaults to 0

	if (!prefixArg.isSet()) {
		// replace with random prefix
		static std::random_device rd;
		static std::mt19937 rng(rd());
		static std::uniform_int_distribution<uint8_t> dist(0, 255);

		prefix = dist(rng);
	}

	// create miners using selected devices
	std::vector<kristforge::Miner> miners;

	for (const cl::Device &d : selectedDevices) {
		kristforge::MinerOptions opts(
				toHex(&prefix, 1), // prefix
				worksizeArg.isSet() ? std::optional(worksizeArg.getValue()) : std::nullopt,
				vecsizeArg.isSet() ? std::optional(vecsizeArg.getValue()) : std::nullopt,
				clCompilerArg.getValue());

		const kristforge::Miner &m = miners.emplace_back(d, opts);
		std::cout << "Created miner: " << m << std::endl;
		prefix++;
	}

	// run tests
	for (kristforge::Miner &m : miners) {
		m.runTests();
	}

	std::cout << "Tests completed successfully" << std::endl;
	if (onlyTestArg.isSet()) return 0;

	// init state
	std::shared_ptr<kristforge::State> state = std::make_shared<kristforge::State>(addressArg.getValue());

	// start miners
	for (kristforge::Miner &m : miners) {
		std::thread t([&m, state] {
			m.run(state);
		});
		t.detach();
	}

	// init network options and callbacks
	kristforge::network::Options netOpts;
	netOpts.verbose = verboseArg.getValue() >= 2;
	netOpts.autoReconnect = true;

	std::atomic<long> blocksMined = 0;
	std::atomic<long> kstMined = 0;

	netOpts.onConnect = [] {
		std::cout << "\nConnected!" << std::endl;
	};

	netOpts.onDisconnect = [&state](bool reconnecting) {
		if (reconnecting) {
			std::cout << "\nDisconnected - trying to reconnect..." << std::endl;
		} else {
			std::cout << "\nDisconnected, stopping miners and exiting" << std::endl;
			state->stop();
		}
	};

	netOpts.onSolved = [&](kristforge::Solution s, long height, long value) {
		blocksMined++;
		kstMined += value;

		std::cout << "\nSuccessfully mined block #" << height <<
		          " (nonce " << s.nonce <<
		          ", value " << value << ")" << std::endl;
	};

	netOpts.onRejected = [](kristforge::Solution s, const std::string &message) {
		std::cout << "\nSolution (nonce " << s.nonce << ") rejected: " << message << std::endl;
	};

	if (verboseArg.isSet()) {
		netOpts.onSubmitted = [](kristforge::Solution s) {
			std::cout << "\nSubmitting solution (nonce " << s.nonce << ")" << std::endl;
		};
	}

	// thread to show status
	std::thread status([&, state] {
		while (!state->isStopped()) {
			long completed = state->hashesCompleted;
			std::this_thread::sleep_for(std::chrono::seconds(3));
			std::cout << "\r"
			          << formatHashrate((state->hashesCompleted - completed) / 3) << " - "
			          << blocksMined.load() << " " << (blocksMined == 1 ? "block" : "blocks") << "/"
			          << kstMined.load() << " KST"
			          << "      " << std::flush;
		}
	});
	status.detach();

	if (exitAfterArg.isSet()) {
		std::thread exitThread([&] {
			std::this_thread::sleep_for(std::chrono::seconds(exitAfterArg.getValue()));
			std::cout << "\nStopping" << std::endl;
			exit(0);
		});
		exitThread.detach();
	}

	// run networking
	if (demoArg.isSet()) {
		kristforge::network::runDemo(demoArg.getValue(), state, netOpts);
	} else {
		kristforge::network::run(kristNode.getValue(), state, netOpts);
	}
}