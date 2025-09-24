use std::fmt;

/// 12-bit unsigned integer for MCS-4 address space
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct U12(u16);

impl U12 {
    pub fn new(value: u16) -> Self {
        U12(value & 0xFFF)
    }

    pub fn value(&self) -> u16 {
        self.0
    }

    pub fn inc(&mut self) {
        self.0 = (self.0 + 1) & 0xFFF;
    }

    pub fn set(&mut self, value: u16) {
        self.0 = value & 0xFFF;
    }

    pub fn wrapping_add(&self, value: u16) -> Self {
        U12::new(self.0.wrapping_add(value))
    }
}

impl fmt::Display for U12 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:03X}", self.0)
    }
}

impl From<u16> for U12 {
    fn from(value: u16) -> Self {
        U12::new(value)
    }
}

impl From<U12> for u16 {
    fn from(value: U12) -> Self {
        value.value()
    }
}