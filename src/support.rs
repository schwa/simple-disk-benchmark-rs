use anyhow::{anyhow, Result};
use core::fmt::{Debug, Display};
use num_traits::{Num, NumCast};
use serde::{Deserialize, Serialize};
use strum::*;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, EnumIter, Display)]
pub enum Unit {
    B,
    KB,
    MB,
    GB,
    TB,
    PB,
    EB,
}

impl Unit {
    // TODO: Really need a generic version.
    fn bytes(&self) -> u64 {
        match self {
            Unit::B => 1,
            Unit::KB => 1024,
            Unit::MB => 1024 * 1024,
            Unit::GB => 1024 * 1024 * 1024,
            Unit::TB => 1024 * 1024 * 1024 * 1024,
            Unit::PB => 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::EB => 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct DataSize<N>
where
    N: Num,
{
    pub size: N,
    pub unit: Unit,
}

// impl DataSize<u64> {
//     fn bytes_u64(self) -> u64 {
//         self.size * self.unit.bytes()
//     }
// }

impl<N: Num> DataSize<N> {
    pub fn new(size: N, unit: Unit) -> Self {
        DataSize { size, unit }
    }

    pub fn from_bytes(size: N) -> Self {
        DataSize {
            size,
            unit: Unit::B,
        }
    }
}

impl<N: Num + NumCast> DataSize<N> {
    fn bytes(self) -> N {
        let bytes: N =
            NumCast::from(self.unit.bytes()).expect("Invalid cast: self.unit.bytes to N.");
        self.size * bytes
    }
}

#[test]
fn test_bytes() {
    assert_eq!(
        DataSize::<f64> {
            size: 1.0,
            unit: Unit::KB
        }
        .bytes(),
        1024.0
    );
}

impl<N: Num + Display> Serialize for DataSize<N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<N: Num + Debug> std::fmt::Debug for DataSize<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.size, self.unit)
    }
}

impl<N: Num + Display> std::fmt::Display for DataSize<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.size, self.unit)
    }
}

impl<N: Num> From<N> for DataSize<N> {
    fn from(size: N) -> Self {
        return DataSize {
            size: size,
            unit: Unit::B,
        };
    }
}

// TODO: Can we make this generic?
impl From<DataSize<usize>> for usize {
    fn from(data_size: DataSize<usize>) -> usize {
        data_size.unit.bytes() as usize * data_size.size
    }
}

#[test]
fn test_1() {
    let make = |size: usize, unit: Unit| -> usize {
        let d = DataSize {
            size: size,
            unit: unit,
        };
        d.into()
    };
    assert_eq!(make(1, Unit::B), 1);
    assert_eq!(make(1, Unit::KB), 1024);
    assert_eq!(make(1, Unit::EB), 1024 * 1024 * 1024 * 1024 * 1024 * 1024);
}

impl<N: Num + PartialOrd + NumCast + Copy> DataSize<N> {
    pub fn lowest_f64_size(self) -> DataSize<f64> {
        if self.size == N::zero() {
            return DataSize::new(0.0, Unit::B);
        }
        let bytes = self.bytes();
        let unit = Unit::iter()
            .rev()
            .find(|unit: &Unit| {
                let unit_bytes = NumCast::from(unit.bytes());
                match unit_bytes {
                    Some(unit_bytes) => bytes >= unit_bytes,
                    None => false,
                }
            })
            .expect(&format!("Couldn't find unit"));

        let size: f64 = self.bytes().to_f64().expect("Size to f64.") / unit.bytes() as f64;
        return DataSize::new(size, unit);
    }

    pub fn to_human_string(self) -> String {
        let size = self.lowest_f64_size();
        return format!("{:.1} {}", size.size, size.unit.to_string());
    }
}

#[test]
fn test_lowest_f64_size() {
    assert_eq!(
        DataSize::new(1, Unit::B).lowest_f64_size(),
        DataSize::new(1.0, Unit::B)
    );
    assert_eq!(
        DataSize::new(128, Unit::MB).lowest_f64_size(),
        DataSize::new(128.0, Unit::MB)
    );
    assert_eq!(
        DataSize::new(2000, Unit::MB).lowest_f64_size(),
        DataSize::new(1.953125, Unit::GB)
    );
}

#[test]
fn test_to_human_string() {
    println!("{}", DataSize::new(1, Unit::B).to_human_string());
    assert_eq!(DataSize::new(1, Unit::B).to_human_string(), "1.0 B");
    assert_eq!(DataSize::new(128, Unit::MB).to_human_string(), "128.0 MB");
}

pub fn parse_data_size(s: &str) -> Result<DataSize<usize>> {
    let re = regex::Regex::new(r"^(\d+)([a-zA-Z]+)$").expect("Invalid regex");
    let caps = re.captures(s).ok_or_else(|| anyhow!("Invalid data size"))?;
    let size = caps[1].parse::<usize>()?;
    let unit = match &caps[2] {
        "b" | "B" => Unit::B,
        "kb" | "KB" => Unit::KB,
        "mb" | "MB" => Unit::MB,
        "gb" | "GB" => Unit::GB,
        "tb" | "TB" => Unit::TB,
        "pb" | "PB" => Unit::PB,
        "eb" | "EB" => Unit::EB,
        _ => return Err(anyhow!("Invalid data size")),
    };
    Ok(DataSize { size, unit })
}

/// A max function for f64's without NaNs
pub fn max(vals: &[f64]) -> f64 {
    *vals
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap()
}

/// A min function for f64's without NaNs
pub fn min(vals: &[f64]) -> f64 {
    *vals
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap()
}
