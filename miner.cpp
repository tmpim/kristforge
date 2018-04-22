#include "miner.h"
#include "cl_amd.h"
#include "cl_nv.h"
#include "utils.h"

#include <string>
#include <numeric>

#include "kristforge.cl.xxd"
static const std::string clSource(reinterpret_cast<const char*>(kristforge_cl), kristforge_cl_len);

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

size_t kristforge::Miner::worksize() {
	if (opts.worksize) return *opts.worksize;

	std::vector<size_t> sizes = dev.getInfo<CL_DEVICE_MAX_WORK_ITEM_SIZES>();
	return std::accumulate(sizes.begin(), sizes.end(), (size_t) 1, [](size_t a, size_t b) { return a * b; });
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

/** Calculate the score for a given hash */
long scoreHash(const std::string &hash) {
	const auto *raw = reinterpret_cast<const unsigned char *>(hash.data());

	return ((long)raw[5]) + (((long)raw[4]) << 8) + (((long)raw[3]) << 16) + (((long)raw[2]) << 24) + (((long) raw[1]) << 32) + (((long) raw[0]) << 40);
}

/** Input strings for OpenCL tests */
const std::string testInputs[16] = {"abc", "def", "ghi", "jkl", "mno", "pqr", "stu", "vwx", "yzA", "BCD", // NOLINT
                                    "EFG", "HIJ", "KLM", "NOP", "QRS", "TUV"};

void kristforge::Miner::runTests() {
	ensureProgramBuilt();

	cl::Kernel testDigest55(program, "testDigest55");
	cl::Kernel testScore(program, "testScore");
	int vs = vecsize();

	size_t hashInSize = sizeof(cl_uchar) * 64 * vs, hashOutSize = sizeof(cl_uchar) * 32 * vs, scoreOutSize = sizeof(cl_long) * vs;

	// init data arrays
	std::unique_ptr<unsigned char[]> hashInputData(new unsigned char[hashInSize]());
	std::unique_ptr<unsigned char[]> hashOutputData(new unsigned char[hashOutSize]());
	std::unique_ptr<long[]> scoreOutputData(new long[vs]());

	// interleave data according to vector size
	for (int i = 0; i < vs; i++) {
		std::string s = testInputs[i];
		for (int j = 0; j < s.size(); j++) hashInputData[vs * j + i] = static_cast<unsigned char>(s[j]);
	}

	// init OpenCL buffers
	cl::Buffer hashInput(ctx, CL_MEM_READ_WRITE | CL_MEM_HOST_WRITE_ONLY, hashInSize);
	cl::Buffer hashOutput(ctx, CL_MEM_READ_WRITE | CL_MEM_HOST_READ_ONLY, hashOutSize);
	cl::Buffer scoreOutput(ctx, CL_MEM_READ_WRITE | CL_MEM_HOST_READ_ONLY, scoreOutSize);

	// set kernel args
	testDigest55.setArg(0, hashInput);
	testDigest55.setArg(1, 3);
	testDigest55.setArg(2, hashOutput);
	testScore.setArg(0, hashOutput);
	testScore.setArg(1, scoreOutput);

	// enqueue actions
	cmd.enqueueWriteBuffer(hashInput, CL_FALSE, 0, hashInSize, hashInputData.get());
	cmd.enqueueTask(testDigest55);
	cmd.enqueueTask(testScore);
	cmd.enqueueReadBuffer(hashOutput, CL_FALSE, 0, hashOutSize, hashOutputData.get());
	cmd.enqueueReadBuffer(scoreOutput, CL_FALSE, 0, scoreOutSize, scoreOutputData.get());
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

	cl::Kernel miner(program, "kristMiner");

	unsigned short vs = vecsize();
	size_t ws = worksize();

	// init buffers
	cl::Buffer addressBuf(ctx, CL_MEM_READ_ONLY | CL_MEM_HOST_WRITE_ONLY, 10);
	cl::Buffer blockBuf(ctx, CL_MEM_READ_ONLY | CL_MEM_HOST_WRITE_ONLY, 12);
	cl::Buffer prefixBuf(ctx, CL_MEM_READ_ONLY | CL_MEM_HOST_WRITE_ONLY, 2);
	cl::Buffer solutionBuf(ctx, CL_MEM_WRITE_ONLY, 15);

	// set buffer args
	miner.setArg(0, addressBuf);
	miner.setArg(1, blockBuf);
	miner.setArg(2, prefixBuf);
	miner.setArg(5, solutionBuf);

	// copy address/prefix
	cmd.enqueueWriteBuffer(addressBuf, CL_FALSE, 0, 10, state->address.data());
	cmd.enqueueWriteBuffer(prefixBuf, CL_FALSE, 0, 2, opts.prefix.data());
	cmd.flush();

	while (!state->isStopped()) {
		kristforge::Target target = state->getTarget();

		// copy block buffer, blank solution buffer
		cmd.enqueueWriteBuffer(blockBuf, CL_FALSE, 0, 12, target.prevBlock.data());
		cmd.enqueueFillBuffer(solutionBuf, (cl_uchar) 0, 0, 15);
		cmd.flush();

		// set work
		miner.setArg(4, target.work);

		unsigned char solutionNonce[15] = {0};

		for (cl_long offset = 1; state->getTargetNow() == target; offset += ws * vs) {
			// set offset
			miner.setArg(3, offset);

			// run kernel and get results
			cmd.enqueueNDRangeKernel(miner, 0, ws);
			cmd.enqueueReadBuffer(solutionBuf, CL_FALSE, 0, 15, solutionNonce);
			cmd.finish();

			if (solutionNonce[0] != 0) {
				// submit solution
				kristforge::Solution solution(target, state->address, mkString(solutionNonce, 15));
				state->pushSolution(solution);

				// clear solution buffer
				cmd.enqueueFillBuffer(solutionBuf, (cl_uchar) 0, 0, 15);
				cmd.flush();
			}

			state->hashesCompleted += ws * vs;
		}
	}
}
