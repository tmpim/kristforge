#include "miner.h"
#include "cl_amd.h"
#include "cl_nv.h"

#include <string>
#include <sstream>
#include <openssl/sha.h>

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

	const char *fmt = "PCIE:%0.2x:%0.2x.%0.2d";
	char out[] = "PCIE:00:00.0";

	if (exts.find("cl_amd_device_attribute_query") != std::string::npos) {
		cl_device_topology_amd topo;
		cl_int status = clGetDeviceInfo(dev(), CL_DEVICE_TOPOLOGY_AMD, sizeof(topo), &topo, nullptr);

		if (status == CL_SUCCESS && topo.raw.type == CL_DEVICE_TOPOLOGY_TYPE_PCIE_AMD) {
			snprintf(out, sizeof(out), fmt, topo.pcie.bus, topo.pcie.device, topo.pcie.function);
			return std::string(out);
		}
	} else if (exts.find("cl_nv_device_attribute_query") != std::string::npos) {
		cl_uint bus, slot;

		if (clGetDeviceInfo(dev(), CL_DEVICE_PCI_BUS_ID_NV, sizeof(bus), &bus, nullptr) == CL_SUCCESS &&
		    clGetDeviceInfo(dev(), CL_DEVICE_PCI_SLOT_ID_NV, sizeof(slot), &slot, nullptr) == CL_SUCCESS) {

			snprintf(out, sizeof(out), fmt, bus, slot, 0);
			return std::string(out);
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
		cmd(cl::CommandQueue(this->ctx, this->dev)),
		program(this->ctx, clSource) {}

unsigned short kristforge::Miner::vecsize() {
	return opts.vecsize.value_or(dev.getInfo<CL_DEVICE_PREFERRED_VECTOR_WIDTH_CHAR>());
}

void kristforge::Miner::ensureProgramBuilt() {
	if (program.getBuildInfo<CL_PROGRAM_BUILD_STATUS>(dev) == CL_BUILD_NONE) {
		// first, get compiler options
		std::ostringstream args;

		// vector type size
		args << "-D VECSIZE=" << vecsize() << " ";

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

static const char hex[] = "0123456789abcdef";

/** Convert binary data to hex representation */
std::string toHex(const unsigned char *data, size_t len) {
	std::string output;

	for (int i = 0; i < len; i++) {
		output += hex[data[i] >> 4];
		output += hex[data[i] & 0xf];
	}

	return output;
}

/** Convert binary string to hex representation */
std::string toHex(const std::string &input) {
	std::string output;

	for (const unsigned char c : input) {
		output += hex[c >> 4];
		output += hex[c & 0xf];
	}

	return output;
}

/** Compute sha256 and return hex representation */
std::string sha256hex(const std::string &data) {
	unsigned char hashed[SHA256_DIGEST_LENGTH];
	SHA256(reinterpret_cast<const unsigned char *>(data.data()), data.size(), hashed);
	return toHex(hashed, SHA256_DIGEST_LENGTH);
}

/** Calculate the score for a given hash */
long scoreHash(const std::string &hash) {
	const auto *raw = reinterpret_cast<const unsigned char *>(hash.data());

	return ((long)raw[5]) + (((long)raw[4]) << 8) + (((long)raw[3]) << 16) + (((long)raw[2]) << 24) + (((long) raw[1]) << 32) + (((long) raw[0]) << 40);
}

/** Throw an exception if given inputs aren't equal */
template<typename T>
void assertEquals(const T &expected, const T &got, const std::string &message) {
	if (!(expected == got)) {
		std::ostringstream msgStream;
		msgStream << message << " - got " << got << ", expected " << expected;
		throw std::runtime_error(msgStream.str());
	}
}

/** Input strings for OpenCL tests */
const std::string testInputs[16] = {"abc", "def", "ghi", "jkl", "mno", "pqr", "stu", "vwx", "yzA", "BCD", // NOLINT
                                    "EFG", "HIJ", "KLM", "NOP", "QRS", "TUV"};

void kristforge::Miner::runTests() {
	ensureProgramBuilt();

	cl::Kernel testDigest55(program, "testDigest55");
	cl::Kernel testScore(program, "testScore");
	int vs = vecsize();

	// init data arrays
	unsigned char hashInputData[64 * vs] = {0}, hashOutputData[32 * vs] = {0};
	long scoreOutputData[vs] = {0};

	// interleave data according to vector size
	for (int i = 0; i < vs; i++) {
		std::string s = testInputs[i];
		for (int j = 0; j < s.size(); j++) hashInputData[vs * j + i] = static_cast<unsigned char>(s[j]);
	}

	// init OpenCL buffers
	cl::Buffer hashInput(ctx, CL_MEM_READ_WRITE | CL_MEM_HOST_WRITE_ONLY, sizeof(hashInputData));
	cl::Buffer hashOutput(ctx, CL_MEM_READ_WRITE | CL_MEM_HOST_READ_ONLY, sizeof(hashOutputData));
	cl::Buffer scoreOutput(ctx, CL_MEM_READ_WRITE | CL_MEM_HOST_READ_ONLY, sizeof(scoreOutputData));

	// set kernel args
	testDigest55.setArg(0, hashInput);
	testDigest55.setArg(1, 3);
	testDigest55.setArg(2, hashOutput);
	testScore.setArg(0, hashOutput);
	testScore.setArg(1, scoreOutput);

	// enqueue actions
	cmd.enqueueWriteBuffer(hashInput, CL_FALSE, 0, sizeof(hashInputData), hashInputData);
	cmd.enqueueTask(testDigest55);
	cmd.enqueueTask(testScore);
	cmd.enqueueReadBuffer(hashOutput, CL_FALSE, 0, sizeof(hashOutputData), hashOutputData);
	cmd.enqueueReadBuffer(scoreOutput, CL_FALSE, 0, sizeof(scoreOutputData), scoreOutputData);
	cmd.finish();

	// deinterleave and verify results
	for (int i = 0; i < vs; i++) {
		std::string clHash(32, ' ');
		for (int j = 0; j < clHash.size(); j++) clHash[j] = hashOutputData[vs * j + i];
		std::string expectedHash = sha256hex(testInputs[i]);

		assertEquals(expectedHash, toHex(clHash), "testDigest55 failed for input " + testInputs[i]);
		assertEquals(scoreHash(clHash), scoreOutputData[i], "testScore failed for input " + testInputs[i] + " (hash " + expectedHash + ")");
	}
}

void kristforge::Miner::run(std::shared_ptr<kristforge::State> state) {
	ensureProgramBuilt();
}
