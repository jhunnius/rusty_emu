use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue, DriveStrength};

pub struct GenericBus {
    base: BaseComponent,
    connected_pins: Vec<Arc<Mutex<Pin>>>,
    bus_value: PinValue,
    last_update: Instant,
    settlement_time: Duration,
    is_active: bool,
}

impl GenericBus {
    pub fn new(name: String) -> Self {
        // Bus typically has bidirectional data pins
        let pin_names = vec!["DATA"];
        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        GenericBus {
            base: BaseComponent::new(name, pins),
            connected_pins: Vec::new(),
            bus_value: PinValue::HighZ,
            last_update: Instant::now(),
            settlement_time: Duration::from_nanos(10),
            is_active: true,
        }
    }

    pub fn with_settlement_time(mut self, time: Duration) -> Self {
        self.settlement_time = time;
        self
    }

    pub fn connect_pin(&mut self, pin: Arc<Mutex<Pin>>) -> Result<(), String> {
        if !self.connected_pins.iter().any(|p| Arc::ptr_eq(p, &pin)) {
            self.connected_pins.push(pin);
            Ok(())
        } else {
            Err("Pin already connected to bus".to_string())
        }
    }

    pub fn disconnect_pin(&mut self, pin: &Arc<Mutex<Pin>>) -> Result<(), String> {
        let initial_len = self.connected_pins.len();
        self.connected_pins.retain(|p| !Arc::ptr_eq(p, pin));

        if self.connected_pins.len() < initial_len {
            Ok(())
        } else {
            Err("Pin not found on bus".to_string())
        }
    }

    pub fn get_connected_pins(&self) -> &Vec<Arc<Mutex<Pin>>> {
        &self.connected_pins
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        if !active {
            // Tri-state the bus when deactivated
            self.bus_value = PinValue::HighZ;
            self.propagate_bus_value();
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn get_bus_value(&self) -> PinValue {
        self.bus_value
    }

    fn read_bus_state(&self) -> PinValue {
        if !self.is_active {
            return PinValue::HighZ;
        }

        if self.connected_pins.is_empty() {
            return PinValue::HighZ;
        }

        // Collect all active drivers from connected pins
        let mut drivers = Vec::new();

        for pin in &self.connected_pins {
            if let Ok(pin_guard) = pin.lock() {
                let pin_drivers = pin_guard.get_drivers();
                for (_, (value, strength)) in pin_drivers {
                    if *strength != DriveStrength::HighImpedance && *value != PinValue::HighZ {
                        drivers.push((*value, *strength));
                    }
                }
            }
        }

        if drivers.is_empty() {
            return PinValue::HighZ;
        }

        // Find the strongest driver
        let max_strength = drivers.iter()
            .map(|(_, strength)| *strength)
            .max()
            .unwrap_or(DriveStrength::HighImpedance);

        if max_strength == DriveStrength::HighImpedance {
            return PinValue::HighZ;
        }

        // Get values from strongest drivers
        let strong_drivers: Vec<PinValue> = drivers.iter()
            .filter(|(_, strength)| *strength == max_strength)
            .map(|(value, _)| *value)
            .collect();

        // Resolve conflicts: Low dominates
        if strong_drivers.iter().any(|v| *v == PinValue::Low) {
            PinValue::Low
        } else if strong_drivers.iter().any(|v| *v == PinValue::High) {
            PinValue::High
        } else {
            PinValue::HighZ
        }
    }

    fn propagate_bus_value(&self) {
        if !self.is_active {
            return;
        }

        for pin in &self.connected_pins {
            if let Ok(mut pin_guard) = pin.lock() {
                // The bus acts as a driver for connected pins
                pin_guard.set_driver_with_strength(
                    Some(self.base.get_name().to_string()),
                    self.bus_value,
                    DriveStrength::Standard
                );
            }
        }
    }

    pub fn simulate_bus_contention(&self) -> Result<(), String> {
        // Simulate bus contention detection
        let mut high_drivers = 0;
        let mut low_drivers = 0;

        for pin in &self.connected_pins {
            if let Ok(pin_guard) = pin.lock() {
                let value = pin_guard.read();
                match value {
                    PinValue::High => high_drivers += 1,
                    PinValue::Low => low_drivers += 1,
                    PinValue::HighZ => {}
                }
            }
        }

        if high_drivers > 0 && low_drivers > 0 {
            // Bus contention detected
            Err("Bus contention: multiple drivers conflict".to_string())
        } else {
            Ok(())
        }
    }
}

impl Component for GenericBus {
    fn name(&self) -> String {
        self.base.name()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.base.get_pin(name)
    }

    fn update(&mut self) {
        if !self.is_active {
            return;
        }

        // Respect settlement timing
        if self.last_update.elapsed() < self.settlement_time {
            return;
        }

        let new_bus_value = self.read_bus_state();

        if new_bus_value != self.bus_value {
            self.bus_value = new_bus_value;
            self.propagate_bus_value();
            self.last_update = Instant::now();
        }

        // Check for bus contention (optional - could be expensive)
        // if let Err(e) = self.simulate_bus_contention() {
        //     eprintln!("Bus contention warning: {}", e);
        // }
    }

    fn run(&mut self) {
        self.base.set_running(true);

        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(1));
        }
    }

    fn stop(&mut self) {
        self.base.set_running(false);
        self.set_active(false);
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

// Active bus implementation for test equipment
pub struct ActiveBus {
    base: GenericBus,
    test_pattern: Vec<PinValue>,
    pattern_index: usize,
    pattern_interval: Duration,
    last_pattern_update: Instant,
}

impl ActiveBus {
    pub fn new(name: String) -> Self {
        ActiveBus {
            base: GenericBus::new(name),
            test_pattern: Vec::new(),
            pattern_index: 0,
            pattern_interval: Duration::from_millis(1),
            last_pattern_update: Instant::now(),
        }
    }

    pub fn set_test_pattern(&mut self, pattern: Vec<PinValue>) {
        self.test_pattern = pattern;
        self.pattern_index = 0;
    }

    pub fn set_pattern_interval(&mut self, interval: Duration) {
        self.pattern_interval = interval;
    }

    pub fn drive_pattern(&mut self) {
        if self.test_pattern.is_empty() {
            return;
        }

        if self.last_pattern_update.elapsed() >= self.pattern_interval {
            if let Some(data_pin) = self.base.get_pin("DATA").ok() {
                if let Ok(mut pin_guard) = data_pin.lock() {
                    let value = self.test_pattern[self.pattern_index];
                    pin_guard.set_driver_with_strength(
                        Some(self.base.name() + "_pattern"),
                        value,
                        DriveStrength::Strong
                    );
                }
            }

            self.pattern_index = (self.pattern_index + 1) % self.test_pattern.len();
            self.last_pattern_update = Instant::now();
        }
    }
}

impl Component for ActiveBus {
    fn name(&self) -> String {
        self.base.name()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.base.get_pin(name)
    }

    fn update(&mut self) {
        self.base.update();
        self.drive_pattern();
    }

    fn run(&mut self) {
        self.base.base.set_running(true);

        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10));
        }
    }

    fn stop(&mut self) {
        self.base.stop();
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_creation() {
        let bus = GenericBus::new("TEST_BUS".to_string());
        assert_eq!(bus.name(), "TEST_BUS");
        assert!(!bus.is_running());
        assert!(bus.is_active());
    }

    #[test]
    fn test_bus_pin_connection() {
        let mut bus = GenericBus::new("TEST_BUS".to_string());
        let pin = Arc::new(Mutex::new(Pin::new("TEST_PIN".to_string())));

        assert!(bus.connect_pin(pin.clone()).is_ok());
        assert_eq!(bus.get_connected_pins().len(), 1);

        // Try connecting same pin again
        assert!(bus.connect_pin(pin.clone()).is_err());
    }

    #[test]
    fn test_bus_value_propagation() {
        let mut bus = GenericBus::new("TEST_BUS".to_string());
        let pin1 = Arc::new(Mutex::new(Pin::new("PIN1".to_string())));
        let pin2 = Arc::new(Mutex::new(Pin::new("PIN2".to_string())));

        bus.connect_pin(pin1.clone()).unwrap();
        bus.connect_pin(pin2.clone()).unwrap();

        // Drive pin1 high
        {
            let mut pin_guard = pin1.lock().unwrap();
            pin_guard.set_driver(Some("test".to_string()), PinValue::High);
        }

        // Update bus
        bus.update();

        // Pin2 should see the high value
        let pin2_guard = pin2.lock().unwrap();
        assert_eq!(pin2_guard.read(), PinValue::High);
    }

    #[test]
    fn test_bus_contention_detection() {
        let mut bus = GenericBus::new("TEST_BUS".to_string());
        let pin1 = Arc::new(Mutex::new(Pin::new("PIN1".to_string())));
        let pin2 = Arc::new(Mutex::new(Pin::new("PIN2".to_string())));

        bus.connect_pin(pin1.clone()).unwrap();
        bus.connect_pin(pin2.clone()).unwrap();

        // Drive pins to conflicting states
        {
            let mut pin1_guard = pin1.lock().unwrap();
            pin1_guard.set_driver(Some("driver1".to_string()), PinValue::High);
        }
        {
            let mut pin2_guard = pin2.lock().unwrap();
            pin2_guard.set_driver(Some("driver2".to_string()), PinValue::Low);
        }

        // Should detect bus contention
        assert!(bus.simulate_bus_contention().is_err());
    }

    #[test]
    fn test_active_bus_pattern() {
        let mut active_bus = ActiveBus::new("ACTIVE_BUS".to_string());

        // Start the bus in a new thread
        let bus_arc = std::sync::Arc::new(std::sync::Mutex::new(bus));
        let bus_clone = bus_arc.clone();

        let handle = std::thread::spawn(move || {
            let mut bus = bus_clone.lock().unwrap();
            bus.start().unwrap();
            // Bus runs in this thread
        });

        // Give the thread a moment to start
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Check if bus is running
        {
            let bus = bus_arc.lock().unwrap();
            assert!(bus.is_running());
        }

        let pattern = vec![PinValue::High, PinValue::Low, PinValue::High, PinValue::Low];

        active_bus.set_test_pattern(pattern.clone());
        active_bus.set_pattern_interval(Duration::from_millis(10));

        // Give the thread a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Test that active bus can drive patterns
        active_bus.drive_pattern();
        assert!(active_bus.test_pattern.eq(&pattern));

        // Clean up - stop the bus and join the thread
        {
            let mut bus = bus_arc.lock().unwrap();
            bus.stop().unwrap();
        }

        handle.join().unwrap();
    }
}