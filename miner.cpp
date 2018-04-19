#include "miner.h"

#include <string>
#include <sstream>
#include "cl_amd.h"
#include "cl_nv.h"

extern const char _binary_kristforge_cl_start, _binary_kristforge_cl_end;
static const std::string clSource(&_binary_kristforge_cl_start,
                                  &_binary_kristforge_cl_end - &_binary_kristforge_cl_start);

std::vector<cl::Device> kristforge::getAllDevices() {
	std::vector<cl::Device> out;

	std::vector<cl::Platform> platforms;
	cl::Platform::get(&platforms);

	for (const cl::Platform &p : platforms) {
		std::vector<cl::Device> devs;
		p.getDevices(CL_DEVICE_TYPE_ALL, &devs);

		for (const cl::Device &d : devs) {
			out.push_back(d);
		}
	}

	return out;
}

std::optional<std::string> kristforge::uniqueID(const cl::Device &dev) {
	std::string exts = dev.getInfo<CL_DEVICE_EXTENSIONS>();

	const char *fmt = "PCIE:%0.2x:%0.2x:%0.2d";
	char out[] = "PCIE:00:00.0";

	if (exts.find("cl_amd_device_attribute_query") != std::string::npos) {
		cl_device_topology_amd topo;
		cl_int status = clGetDeviceInfo(dev(), CL_DEVICE_TOPOLOGY_AMD, sizeof(topo), &topo, nullptr);

		if (status == CL_SUCCESS && topo.raw.type == CL_DEVICE_TOPOLOGY_TYPE_PCIE_AMD) {
			snprintf(out, sizeof(out), fmt, topo.pcie.bus, topo.pcie.device, topo.pcie.function);
			return std::string(out, sizeof(out));
		}
	} else if (exts.find("cl_nv_device_attribute_query") != std::string::npos) {
		cl_uint bus, slot;

		if (clGetDeviceInfo(dev(), CL_DEVICE_PCI_BUS_ID_NV, sizeof(bus), &bus, nullptr) == CL_SUCCESS &&
		    clGetDeviceInfo(dev(), CL_DEVICE_PCI_SLOT_ID_NV, sizeof(slot), &slot, nullptr) == CL_SUCCESS) {

			snprintf(out, sizeof(out), fmt, bus, slot, 0);
			return std::string(out, sizeof(out));
		}
	}

	return std::nullopt;
}

long kristforge::scoreDevice(const cl::Device &dev) {
	return dev.getInfo<CL_DEVICE_MAX_COMPUTE_UNITS>() *
	       dev.getInfo<CL_DEVICE_MAX_CLOCK_FREQUENCY>() *
	       dev.getInfo<CL_DEVICE_PREFERRED_VECTOR_WIDTH_CHAR>();
}

kristforge::Miner::Miner(cl::Device dev, kristforge::MinerOptions opts) :
		dev(std::move(dev)),
		opts(std::move(opts)),
		ctx(cl::Context(this->dev)),
		queue(cl::CommandQueue(this->ctx, this->dev)),
		program(this->ctx, clSource) {}

void kristforge::Miner::ensureProgramBuilt() {
	if (program.getBuildInfo<CL_PROGRAM_BUILD_STATUS>(dev) == CL_BUILD_NONE) {
		// first, get compiler options
		std::ostringstream args;

		// vector type size
		args << "-D VECSIZE=" << opts.vecsize.value_or(dev.getInfo<CL_DEVICE_PREFERRED_VECTOR_WIDTH_CHAR>()) << " ";

		// custom extra compiler flags
		args << opts.extraOpts;

		try {
			program.build(args.str().data());
		} catch (const cl::Error &e) {
			if (e.err() == CL_BUILD_PROGRAM_FAILURE) {
				std::ostringstream msg;

				msg << "Program build failure for " << *this << " using arguments [" << args.str() << "]:" << std::endl
				    << program.getBuildInfo<CL_PROGRAM_BUILD_LOG>(dev);

				throw std::runtime_error(msg.str());
			} else {
				throw e;
			}
		}
	}
}


void kristforge::Miner::runTests() {
	ensureProgramBuilt();
}

void kristforge::Miner::run(std::shared_ptr<kristforge::State> state) {
	ensureProgramBuilt();
}
