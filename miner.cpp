#include "miner.h"

#include <string>

extern const char _binary_kristforge_cl_start, _binary_kristforge_cl_end;
static const std::string clSource(&_binary_kristforge_cl_start, &_binary_kristforge_cl_end - &_binary_kristforge_cl_start);
