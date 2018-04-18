#pragma once

#include "state.h"

#define __CL_ENABLE_EXCEPTIONS

#include <CL/cl.hpp>
#include <memory>
#include <optional>

namespace kristforge {
	struct MinerOptions {
	public:
		explicit MinerOptions(std::string prefix,
		                      std::optional<size_t> worksize = std::nullopt,
		                      std::optional<unsigned short> vecsize = std::nullopt) :
				prefix(std::move(prefix)),
				worksize(std::move(worksize)),
				vecsize(std::move(vecsize)) {
			if (this->prefix.size() != 2) throw std::range_error("Prefix length must be 2");
			if (vecsize) {
				if (!(vecsize == 1 || vecsize == 2 || vecsize == 4 || vecsize == 8 || vecsize == 16)) {
					throw std::range_error("Invalid vector size: " + *vecsize);
				}
			}
		}

	private:
		const std::string prefix;
		std::optional<size_t> worksize;
		std::optional<unsigned short> vecsize;
	};
}