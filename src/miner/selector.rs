use crate::ext::DeviceExt;
use ascii_tree::Tree;
use failure::Fail;
use lazy_static::lazy_static;
use ocl::error::Result as OclResult;
use ocl::{Device, Platform};
use ocl_extras::full_device_info::FullDeviceInfo;
use regex::Regex;
use std::collections::HashSet;
use std::iter::once;
use std::str::FromStr;

/// A selector used to choose OpenCL devices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selector {
    /// Matches all non-custom OpenCL devices
    All,

    /// Matches all non-custom devices for a given platform
    PlatformIndex(usize),

    /// Matches a single device by platform and device index
    DeviceIndex(usize, usize),
}

impl Selector {
    /// Get the pool of all devices to select from
    fn get_pool() -> OclResult<Vec<Vec<Device>>> {
        let mut pool = vec![];

        for platform in Platform::list() {
            let mut devices = vec![];

            for device in Device::list_all(platform)? {
                devices.push(device);
            }

            pool.push(devices);
        }

        Ok(pool)
    }

    /// Output an ascii tree of the available OpenCL devices
    pub fn ascii_tree() -> OclResult<Tree> {
        let mut platforms = vec![];

        for (pi, platform) in Platform::list().into_iter().enumerate() {
            let mut devices = vec![];

            for (di, device) in Device::list_all(platform)?.into_iter().enumerate() {
                devices.push(Tree::Leaf(vec![
                    format!("Device {} [selector: p{}d{}]", device.human_name()?, pi, di),
                    format!("Max compute units: {}", device.max_compute_units()?),
                    format!("Max clock frequency: {} MHz", device.max_clock_frequency()?),
                    format!(
                        "Preferred char vector width: {}",
                        device.preferred_vector_width_char()?
                    ),
                ]));
            }

            platforms.push(Tree::Node(
                format!("Platform {} [selector: p{}]", platform.name()?, pi),
                devices,
            ));
        }

        Ok(Tree::Node(
            "OpenCL Devices [selector: all]".to_owned(),
            platforms,
        ))
    }

    fn select(self, pool: &[Vec<Device>]) -> Vec<Device> {
        use self::Selector::*;

        match self {
            // valid selectors
            All => pool.iter().flat_map(|p| p.iter().cloned()).collect(),
            PlatformIndex(p) if p < pool.len() => pool[p].to_vec(),
            DeviceIndex(p, d) if p < pool.len() && d < pool[p].len() => once(pool[p][d]).collect(),

            // invalid selectors
            PlatformIndex(p) => panic!("Platform index {} out of range", p),
            DeviceIndex(p, d) if p < pool.len() => {
                panic!("Device index {} out of range for platform index {}", p, d)
            }
            DeviceIndex(p, _) => panic!("Platform index {} out of range", p),
        }
    }

    /// Get a set of devices matched by any of the given `selectors`
    pub fn select_all(selectors: &[Self]) -> OclResult<HashSet<Device>> {
        let pool = Self::get_pool()?;

        let mut selected = HashSet::new();

        for selector in selectors {
            selected.extend(selector.select(&pool));
        }

        Ok(selected)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Fail)]
#[fail(display = "invalid device selector: {}", 0)]
pub struct InvalidSelector(String);

impl FromStr for Selector {
    type Err = InvalidSelector;

    fn from_str(s: &str) -> Result<Selector, InvalidSelector> {
        lazy_static! {
            static ref PLATFORM_RE: Regex = Regex::new("^p(\\d+)$").unwrap();
            static ref DEVICE_RE: Regex = Regex::new("^p(\\d+)d(\\d+)$").unwrap();
        }

        if s == "all" {
            Ok(Selector::All)
        } else if let Some(c) = PLATFORM_RE.captures(s) {
            Ok(Selector::PlatformIndex(c[1].parse().unwrap()))
        } else if let Some(c) = DEVICE_RE.captures(s) {
            Ok(Selector::DeviceIndex(
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
            ))
        } else {
            Err(InvalidSelector(s.into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selectors() {
        assert_eq!(Ok(Selector::All), Selector::from_str("all"));
        assert_eq!(Ok(Selector::PlatformIndex(1)), Selector::from_str("p1"));
        assert_eq!(Ok(Selector::DeviceIndex(1, 2)), Selector::from_str("p1d2"));
        assert_eq!(
            Err(InvalidSelector("blah".into())),
            Selector::from_str("blah")
        );
    }
}
