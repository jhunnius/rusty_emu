use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use crate::pin::Pin;

pub trait Component {
    fn name(&self) -> String;
    fn pins(&self) -> &HashMap<String, Arc<Mutex<Pin>>>;
    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String>;
    fn update(&mut self);
    fn run(&mut self);
    fn stop(&mut self);

    // Helper method to check if component is running
    fn is_running(&self) -> bool;
}

pub struct BaseComponent {
    name: String,
    pins: HashMap<String, Arc<Mutex<Pin>>>,
    running: AtomicBool,
}

impl BaseComponent {
    pub fn new(name: String, pins: HashMap<String, Arc<Mutex<Pin>>>) -> Self {
        BaseComponent {
            name,
            pins,
            running: AtomicBool::new(false),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }

    pub fn create_pin_map(pin_names: &[&str], component_name: &str) -> HashMap<String, Arc<Mutex<Pin>>> {
        let mut pins = HashMap::new();
        for pin_name in pin_names {
            pins.insert(
                pin_name.to_string(),
                Arc::new(Mutex::new(Pin::new(format!("{}_{}", component_name, pin_name))))
            );
        }
        pins
    }

    pub fn add_pin(&mut self, name: String, pin: Arc<Mutex<Pin>>) {
        self.pins.insert(name, pin);
    }

    pub fn remove_pin(&mut self, name: &str) -> Option<Arc<Mutex<Pin>>> {
        self.pins.remove(name)
    }

    // Helper method to get a mutable reference to pins (for components that need to modify pins)
    pub fn pins_mut(&mut self) -> &mut HashMap<String, Arc<Mutex<Pin>>> {
        &mut self.pins
    }
}

// Implement Clone manually without AtomicBool
impl Clone for BaseComponent {
    fn clone(&self) -> Self {
        // Create new pins with the same names but new Pin instances
        let mut new_pins = HashMap::new();
        for (name, pin) in &self.pins {
            let pin_guard = pin.lock().unwrap();
            new_pins.insert(
                name.clone(),
                Arc::new(Mutex::new(Pin::new(pin_guard.name().to_string())))
            );
        }

        BaseComponent {
            name: self.name.clone(),
            pins: new_pins,
            running: AtomicBool::new(false), // Always start as not running when cloned
        }
    }
}

// Default implementation for Component trait
impl Component for BaseComponent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn pins(&self) -> &HashMap<String, Arc<Mutex<Pin>>> {
        &self.pins
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.pins.get(name)
            .cloned()
            .ok_or_else(|| format!("Pin {} not found on component {}", name, self.name))
    }

    fn update(&mut self) {
        // Base implementation does nothing - components should override this
        // This is where pin state changes would be processed
    }

    fn run(&mut self) {
        self.set_running(true);

        // Simple run loop - components should override this for their specific behavior
        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10)); // Prevent busy waiting
        }
    }

    fn stop(&mut self) {
        self.set_running(false);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

// Helper function to create component pins easily
pub fn create_component_pins(component_name: &str, pin_definitions: &[(&str, &str)]) -> HashMap<String, Arc<Mutex<Pin>>> {
    let mut pins = HashMap::new();

    for (pin_name, pin_type) in pin_definitions {
        let full_pin_name = format!("{}_{}", component_name, pin_name);
        let mut pin = Pin::new(full_pin_name);

        // Set initial state based on pin type
        match *pin_type {
            "input" | "in" => {
                // Input pins start as HighZ (waiting for input)
                pin.set_driver(Some("internal".to_string()), crate::pin::PinValue::HighZ);
            }
            "output" | "out" => {
                // Output pins start as HighZ (tri-state)
                pin.set_driver(Some("internal".to_string()), crate::pin::PinValue::HighZ);
            }
            "power" => {
                // Power pins start high
                pin.set_driver(Some("internal".to_string()), crate::pin::PinValue::High);
            }
            "ground" => {
                // Ground pins start low
                pin.set_driver(Some("internal".to_string()), crate::pin::PinValue::Low);
            }
            _ => {
                // Default to HighZ
                pin.set_driver(Some("internal".to_string()), crate::pin::PinValue::HighZ);
            }
        }

        pins.insert(pin_name.to_string(), Arc::new(Mutex::new(pin)));
    }

    pins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_component_creation() {
        let pins = BaseComponent::create_pin_map(&["A0", "A1", "D0", "D1"], "TEST");
        let component = BaseComponent::new("TEST_COMP".to_string(), pins);

        assert_eq!(component.name(), "TEST_COMP");
        assert!(!component.is_running());
    }

    #[test]
    fn test_base_component_pin_management() {
        let pins = BaseComponent::create_pin_map(&["A0", "D0"], "TEST");
        let mut component = BaseComponent::new("TEST_COMP".to_string(), pins);

        // Test getting existing pin
        assert!(component.get_pin("A0").is_ok());

        // Test getting non-existent pin
        assert!(component.get_pin("NON_EXISTENT").is_err());

        // Test adding new pin
        let new_pin = Arc::new(Mutex::new(Pin::new("TEST_NEW".to_string())));
        component.add_pin("NEW_PIN".to_string(), new_pin);
        assert!(component.get_pin("NEW_PIN").is_ok());

        // Test removing pin
        let removed = component.remove_pin("A0");
        assert!(removed.is_some());
        assert!(component.get_pin("A0").is_err());
    }

    #[test]
    fn test_base_component_lifecycle() {
        let pins = BaseComponent::create_pin_map(&["TEST"], "TEST");
        let mut component = BaseComponent::new("TEST_COMP".to_string(), pins);

        assert!(!component.is_running());

        // Note: In real usage, run() would be called in a separate thread
        component.set_running(true);
        assert!(component.is_running());

        component.stop();
        assert!(!component.is_running());
    }

    #[test]
    fn test_base_component_clone() {
        let pins = BaseComponent::create_pin_map(&["A0", "D0"], "ORIG");
        let original = BaseComponent::new("ORIGINAL".to_string(), pins);
        original.set_running(true);

        let cloned = original.clone();

        // Cloned component should have same name but not running state
        assert_eq!(cloned.name(), "ORIGINAL");
        assert!(!cloned.is_running()); // Clone should not be running

        // Pins should exist but be different instances
        assert!(cloned.get_pin("A0").is_ok());
        assert!(cloned.get_pin("D0").is_ok());
    }

    #[test]
    fn test_create_component_pins() {
        let pin_definitions = vec![
            ("A0", "input"),
            ("D0", "output"),
            ("VCC", "power"),
            ("GND", "ground"),
        ];

        let pins = create_component_pins("TEST", &pin_definitions);

        assert_eq!(pins.len(), 4);
        assert!(pins.contains_key("A0"));
        assert!(pins.contains_key("D0"));
        assert!(pins.contains_key("VCC"));
        assert!(pins.contains_key("GND"));
    }
}