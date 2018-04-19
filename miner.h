#pragma once

#include "state.h"

#define __CL_ENABLE_EXCEPTIONS

#include <CL/cl.hpp>
#include <memory>
#include <optional>
#include <iostream>

namespace kristforge {
	/** Get all standard OpenCL devices from all platforms */
	std::vector<cl::Device> getAllDevices();

	/** Get a unique ID for this device, if possible */
	std::optional<std::string> uniqueID(const cl::Device &dev);

	/** Calculate a score for this device, estimating how effective it will be for mining - higher is better */
	long scoreDevice(const cl::Device &dev);

	/** Options for a specific miner */
	struct MinerOptions {
	public:
		explicit MinerOptions(std::string prefix,
		                      std::optional<size_t> worksize = std::nullopt,
		                      std::optional<unsigned short> vecsize = std::nullopt,
		                      std::string extraOpts = "") :
				prefix(std::move(prefix)),
				worksize(std::move(worksize)),
				vecsize(std::move(vecsize)),
				extraOpts(std::move(extraOpts)) {
			if (this->prefix.size() != 2) throw std::range_error("Prefix length must be 2");
		}

	private:
		const std::string prefix;
		const std::optional<size_t> worksize;
		const std::optional<unsigned short> vecsize;
		const std::string extraOpts;

		friend class Miner;

		friend std::ostream &operator<<(std::ostream &os, const MinerOptions &opts);
	};

	inline std::ostream &operator<<(std::ostream &os, const MinerOptions &opts) {
		return os << "MinerOptions (prefix " << opts.prefix
		          << " worksize " << (opts.worksize ? std::to_string(*opts.worksize) : "auto")
		          << " vecsize " << (opts.vecsize ? std::to_string(*opts.vecsize) : "auto") << ")";
	}

	/** An OpenCL miner */
	class Miner {
	public:
		/** Create a miner using a given OpenCL device */
		Miner(cl::Device dev, MinerOptions opts);

		/** Runs tests to ensure mining will work properly */
		void runTests();

		/** Runs the miner synchronously using the given state */
		void run(std::shared_ptr<State> state);

	private:
		const cl::Device dev;
		const MinerOptions opts;

		const cl::Context ctx;
		const cl::CommandQueue queue;
		const cl::Program program;

		/** If the OpenCL program hasn't been built yet, build it now */
		void ensureProgramBuilt();

		friend std::ostream &operator<<(std::ostream &os, const Miner &m);
	};

	inline std::ostream &operator<<(std::ostream &os, const Miner &m) {
		return os << "Miner " << m.dev.getInfo<CL_DEVICE_NAME>().data()
		          << " (score " << std::to_string(scoreDevice(m.dev))
		          << " id " << uniqueID(m.dev).value_or("n/a")
		          << " " << m.opts << ")";
	}
}