#pragma once

#define CL_DEVICE_TOPOLOGY_AMD 0x4037

#define CL_DEVICE_TOPOLOGY_TYPE_PCIE_AMD 1

typedef union {
	struct {
		cl_uint type;
		cl_uint data[5];
	} raw;

	struct {
		cl_uint type;
		cl_char unused[17];
		cl_char bus;
		cl_char device;
		cl_char function;
	} pcie;
} cl_device_topology_amd;