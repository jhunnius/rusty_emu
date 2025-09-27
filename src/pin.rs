use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinValue {
    Low,
    High,
    HighZ, // Tri-state
}

impl PinValue {
    pub fn to_str(&self) -> &'static str {
        match self {
            PinValue::Low => "Low",
            PinValue::High => "High",
            PinValue::HighZ => "HighZ",
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            PinValue::Low => '0',
            PinValue::High => '1',
            PinValue::HighZ => 'Z',
        }
    }

    pub fn from_bool(value: bool) -> Self {
        if value {
            PinValue::High
        } else {
            PinValue::Low
        }
    }

    pub fn to_bool(&self) -> Option<bool> {
        match self {
            PinValue::Low => Some(false),
            PinValue::High => Some(true),
            PinValue::HighZ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveStrength {
    HighImpedance = 0,
    Weak = 1,
    Standard = 2,
    Strong = 3,
}

impl Ord for DriveStrength {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

impl PartialOrd for DriveStrength {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
pub struct Pin {
    name: String,
    drivers: HashMap<String, (PinValue, DriveStrength)>,
    settled_value: PinValue,
    last_update: Instant,
    settlement_time: Duration,
    connected_pins: Vec<Arc<Mutex<Pin>>>,
}
impl Pin {
    pub fn new(name: String) -> Self {
        Pin {
            name,
            drivers: HashMap::new(),
            settled_value: PinValue::HighZ,
            last_update: Instant::now(),
            settlement_time: Duration::from_nanos(10), // 10ns settlement time
            connected_pins: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_driver(&mut self, driver_name: Option<String>, value: PinValue) {
        let strength = DriveStrength::Standard;
        self.set_driver_with_strength(driver_name, value, strength);
    }

    pub fn set_driver_with_strength(
        &mut self,
        driver_name: Option<String>,
        value: PinValue,
        strength: DriveStrength,
    ) {
        let driver_id = driver_name.unwrap_or_else(|| "anonymous".to_string());

        if value == PinValue::HighZ && strength == DriveStrength::HighImpedance {
            self.drivers.remove(&driver_id);
        } else {
            self.drivers.insert(driver_id, (value, strength));
        }

        self.last_update = Instant::now();
        self.recalculate_value();
    }

    pub fn remove_driver(&mut self, driver_name: &str) {
        self.drivers.remove(driver_name);
        self.last_update = Instant::now();
        self.recalculate_value();
    }

    pub fn read(&self) -> PinValue {
        // If we're still within settlement time, return the previous value
        if self.last_update.elapsed() < self.settlement_time {
            return self.settled_value;
        }
        self.settled_value
    }

    pub fn read_immediate(&self) -> PinValue {
        self.settled_value
    }

    pub fn get_drivers(&self) -> &HashMap<String, (PinValue, DriveStrength)> {
        &self.drivers
    }

    pub fn is_settled(&self) -> bool {
        self.last_update.elapsed() >= self.settlement_time
    }

    pub fn get_settlement_time(&self) -> Duration {
        self.settlement_time
    }

    pub fn set_settlement_time(&mut self, time: Duration) {
        self.settlement_time = time;
    }

    pub fn connect_to(&mut self, other_pin: Arc<Mutex<Pin>>) {
        if !self
            .connected_pins
            .iter()
            .any(|p| Arc::ptr_eq(p, &other_pin))
        {
            self.connected_pins.push(other_pin);
        }
    }

    pub fn disconnect_from(&mut self, other_pin: &Arc<Mutex<Pin>>) {
        self.connected_pins.retain(|p| !Arc::ptr_eq(p, other_pin));
    }

    pub fn get_connected_pins(&self) -> &Vec<Arc<Mutex<Pin>>> {
        &self.connected_pins
    }

    pub fn propagate(&self) {
        for connected_pin in &self.connected_pins {
            if let Ok(mut pin) = connected_pin.lock() {
                // Copy our drivers to the connected pin (simulate electrical connection)
                let mut new_drivers = self.drivers.clone();

                // Merge with existing drivers on the connected pin
                for (driver, value) in &pin.drivers {
                    new_drivers.insert(driver.clone(), *value);
                }

                pin.drivers = new_drivers;
                pin.last_update = Instant::now();
                pin.recalculate_value();
            }
        }
    }

    fn recalculate_value(&mut self) {
        if self.drivers.is_empty() {
            self.settled_value = PinValue::HighZ;
            return;
        }

        // Find the strongest driver strength manually
        let mut max_strength = DriveStrength::HighImpedance;
        for (_, strength) in self.drivers.values() {
            if *strength > max_strength {
                max_strength = *strength;
            }
        }
        if max_strength == DriveStrength::HighImpedance {
            self.settled_value = PinValue::HighZ;
            return;
        }

        // Get all drivers with the strongest strength
        let strong_drivers: Vec<PinValue> = self
            .drivers
            .values()
            .filter(|(_, strength)| *strength == max_strength)
            .map(|(value, _)| *value)
            .collect();

        // Resolve conflicts: Low dominates, then High, HighZ is ignored
        if strong_drivers.iter().any(|v| *v == PinValue::Low) {
            self.settled_value = PinValue::Low;
        } else if strong_drivers.iter().any(|v| *v == PinValue::High) {
            self.settled_value = PinValue::High;
        } else {
            self.settled_value = PinValue::HighZ;
        }

        // Propagate to connected pins
        self.propagate();
    }
    pub fn clear_connections(&mut self) {
        self.connected_pins.clear();
    }

    pub fn disconnect_from_pin(&mut self, other_pin: &Arc<Mutex<Pin>>) {
        self.connected_pins.retain(|p| !Arc::ptr_eq(p, other_pin));
    }

    pub fn is_connected_to(&self, other_pin: &Arc<Mutex<Pin>>) -> bool {
        self.connected_pins
            .iter()
            .any(|p| Arc::ptr_eq(p, other_pin))
    }

    pub fn get_connection_count(&self) -> usize {
        self.connected_pins.len()
    }

    pub fn to_string(&self) -> String {
        format!("{}: {}", self.name, self.settled_value.to_str())
    }
}

impl Default for Pin {
    fn default() -> Self {
        Pin::new("unnamed".to_string())
    }
}

// Helper implementations for easier testing and debugging
impl std::fmt::Display for PinValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl std::fmt::Display for Pin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.settled_value)?;

        if !self.drivers.is_empty() {
            write!(f, " [drivers: ")?;
            for (i, (driver, (value, strength))) in self.drivers.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}={}({})", driver, value.to_char(), *strength as u8)?;
            }
            write!(f, "]")?;
        }

        if !self.is_settled() {
            write!(f, " (settling)")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn test_pin_creation() {
        let pin = Pin::new("TEST".to_string());
        assert_eq!(pin.name(), "TEST");
        assert_eq!(pin.read(), PinValue::HighZ);
        assert!(pin.drivers.is_empty());
    }

    #[test]
    fn test_pin_driving() {
        let mut pin = Pin::new("TEST".to_string());

        // Drive high
        pin.set_driver(Some("driver1".to_string()), PinValue::High);
        assert_eq!(pin.read(), PinValue::High);

        // Drive low - should override
        pin.set_driver(Some("driver2".to_string()), PinValue::Low);
        assert_eq!(pin.read(), PinValue::Low);

        // Remove driver
        pin.remove_driver("driver2");
        assert_eq!(pin.read(), PinValue::High);
    }

    #[test]
    fn test_pin_strength() {
        let mut pin = Pin::new("TEST".to_string());

        // Weak high
        pin.set_driver_with_strength(
            Some("weak".to_string()),
            PinValue::High,
            DriveStrength::Weak,
        );
        assert_eq!(pin.read(), PinValue::High);

        // Strong low should override weak high
        pin.set_driver_with_strength(
            Some("strong".to_string()),
            PinValue::Low,
            DriveStrength::Strong,
        );
        assert_eq!(pin.read(), PinValue::Low);
    }

    #[test]
    fn test_pin_connection() {
        let pin1 = Arc::new(Mutex::new(Pin::new("PIN1".to_string())));
        let pin2 = Arc::new(Mutex::new(Pin::new("PIN2".to_string())));

        // Connect pins
        {
            let mut p1 = pin1.lock().unwrap();
            p1.connect_to(pin2.clone());
        }

        // Drive pin1
        {
            let mut p1 = pin1.lock().unwrap();
            p1.set_driver(Some("test".to_string()), PinValue::High);
        }

        // Wait for propagation
        std::thread::sleep(Duration::from_millis(1));

        // Check pin2
        let p2 = pin2.lock().unwrap();
        assert_eq!(p2.read(), PinValue::High);
    }

    #[test]
    fn test_pin_conflict_resolution() {
        let mut pin = Pin::new("TEST".to_string());

        // Multiple drivers with same strength - Low should dominate
        pin.set_driver(Some("driver1".to_string()), PinValue::High);
        pin.set_driver(Some("driver2".to_string()), PinValue::Low);
        pin.set_driver(Some("driver3".to_string()), PinValue::High);

        assert_eq!(pin.read(), PinValue::Low);

        // Remove low driver - should settle to High
        pin.remove_driver("driver2");
        assert_eq!(pin.read(), PinValue::High);
    }

    #[test]
    fn test_pin_tri_state() {
        let mut pin = Pin::new("TEST".to_string());

        // Drive then tri-state
        pin.set_driver(Some("driver".to_string()), PinValue::High);
        assert_eq!(pin.read(), PinValue::High);

        pin.set_driver(Some("driver".to_string()), PinValue::HighZ);
        assert_eq!(pin.read(), PinValue::HighZ);
    }
}
