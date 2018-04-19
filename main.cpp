#include <iostream>
#include <thread>
#include <vector>
#include <tclap/CmdLine.h>

#include "network.h"
#include "miner.h"

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
	// @formatter:on

	cmd.parse(argc, argv);

	if (listDevicesArg.isSet()) {
		printDeviceList();
		return 0;
	}
}