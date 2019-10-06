use ocl::error::Result as OclResult;
use ocl::Device;
use ocl_extras::full_device_info::FullDeviceInfo;

pub trait DeviceExt {
    fn human_name(&self) -> OclResult<String>;

    fn preferred_vecsize(&self) -> OclResult<u32>;
}

impl DeviceExt for Device {
    fn human_name(&self) -> OclResult<String> {
        const CL_DEVICE_BOARD_NAME_AMD: u32 = 0x4038;

        if self.extensions()?.contains("cl_amd_device_attribute_query") {
            // read raw name
            let mut raw = self.info_raw(CL_DEVICE_BOARD_NAME_AMD)?;

            // remove null byte(s)
            raw.retain(|&b| b != 0);

            // parse as UTF-8
            Ok(String::from_utf8_lossy(&raw).into_owned())
        } else {
            self.name()
        }
    }

    fn preferred_vecsize(&self) -> OclResult<u32> {
        Ok(1)
    }
}
