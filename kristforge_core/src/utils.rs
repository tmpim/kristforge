use std::cmp::min;
use std::fmt::{self, Debug, Display, Formatter};
use std::time::Duration;

/// A type representing a hash rate, which can be printed in a human-readable
/// format.
#[derive(Debug, Clone, Copy)]
pub struct HashRate {
    pub hashes: usize,
    pub elapsed: Duration,
}

impl Display for HashRate {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let per_second = self.hashes as f64 / self.elapsed.as_secs_f64();

        const PREFIXES: &[&str] = &["", "k", "M", "G", "T"];
        let mag = min(PREFIXES.len() - 1, per_second.log(1000.).floor() as usize);
        let n = per_second / 1000f64.powf(mag as f64);

        let precision = f.precision().unwrap_or(2);
        write!(f, "{:.*} {}h/s", precision, n, PREFIXES[mag])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hash_rate_formatting() {
        assert_eq!(
            HashRate {
                hashes: 1_000,
                elapsed: Duration::from_secs(1)
            }
            .to_string(),
            "1.00 kh/s"
        );

        assert_eq!(
            format!(
                "{:.1}",
                HashRate {
                    hashes: 500_000_000_000,
                    elapsed: Duration::from_secs(1)
                }
            ),
            "500.0 Gh/s"
        );
    }
}
