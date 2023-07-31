use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    B,
    KB,
    MB,
    GB,
    TB,
    PB,
    EB,
    ZB,
    YB,
}

#[derive(Clone, PartialEq)]
pub struct DataSize {
    pub size: usize,
    pub unit: Unit,
}

impl std::fmt::Debug for DataSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.unit {
            Unit::B => write!(f, "{}B", self.size),
            Unit::KB => write!(f, "{}KB", self.size),
            Unit::MB => write!(f, "{}MB", self.size),
            Unit::GB => write!(f, "{}GB", self.size),
            Unit::TB => write!(f, "{}TB", self.size),
            Unit::PB => write!(f, "{}PB", self.size),
            Unit::EB => write!(f, "{}EB", self.size),
            Unit::ZB => write!(f, "{}ZB", self.size),
            Unit::YB => write!(f, "{}YB", self.size),
        }
    }
}

impl std::fmt::Display for DataSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.unit {
            Unit::B => write!(f, "{}B", self.size),
            Unit::KB => write!(f, "{}KB", self.size),
            Unit::MB => write!(f, "{}MB", self.size),
            Unit::GB => write!(f, "{}GB", self.size),
            Unit::TB => write!(f, "{}TB", self.size),
            Unit::PB => write!(f, "{}PB", self.size),
            Unit::EB => write!(f, "{}EB", self.size),
            Unit::ZB => write!(f, "{}ZB", self.size),
            Unit::YB => write!(f, "{}YB", self.size),
        }
    }
}

impl DataSize {
    pub fn to_bytes(&self) -> usize {
        match self.unit {
            Unit::B => self.size,
            Unit::KB => self.size * 1024,
            Unit::MB => self.size * 1024 * 1024,
            Unit::GB => self.size * 1024 * 1024 * 1024,
            Unit::TB => self.size * 1024 * 1024 * 1024 * 1024,
            Unit::PB => self.size * 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::EB => self.size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::ZB => self.size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::YB => self.size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
        }
    }
}

pub fn parse_data_size(s: &str) -> Result<DataSize> {
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
        "zb" | "ZB" => Unit::ZB,
        "yb" | "YB" => Unit::YB,
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