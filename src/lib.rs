mod systems;
mod components;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Pin value representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinValue {
    High,
    Low,
    HighZ,  // High impedance (disconnected)
}
// Pin structure with output enable
#[derive(Clone)]
pub struct Pin {
    pub name: String,
    pub value: Arc<RwLock<PinValue>>,  // Use RwLock for multiple readers
    pub output_enable: Arc<Mutex<bool>>, // Whether this pin is driving the line
    pub connections: Arc<Mutex<Vec<Arc<Pin>>>>,
}

// Component trait
impl Pin {
    // Get the effective value considering all connected pins
    pub fn read(&self) -> PinValue {
        let connections = self.connections.lock().unwrap();

        // Check if any connected pin is actively driving low
        for connected_pin in connections.iter() {
            let output_enable = connected_pin.output_enable.lock().unwrap();
            let value = connected_pin.value.read().unwrap();

            if *output_enable && *value == PinValue::Low {
                return PinValue::Low;
            }
        }

        // If no one is driving low, check if anyone is driving high
        for connected_pin in connections.iter() {
            let output_enable = connected_pin.output_enable.lock().unwrap();
            let value = connected_pin.value.read().unwrap();

            if *output_enable && *value == PinValue::High {
                return PinValue::High;
            }
        }

        // If no one is driving, return HighZ
        PinValue::HighZ
    }

    // Set this pin's value and output enable
    pub fn write(&self, value: PinValue, output_enable: bool) {
        *self.value.write().unwrap() = value;
        *self.output_enable.lock().unwrap() = output_enable;
    }
}

// Component trait
pub trait Component: Send {
    fn name(&self) -> &str;
    fn pins(&self) -> &HashMap<String, Arc<Pin>>;
    fn get_pin(&self, name: &str) -> Option<Arc<Pin>>;
    fn connect_pin(&mut self, pin_name: &str, other_pin: Arc<Pin>) -> Result<(), String>;
    fn update(&mut self) -> Result<(), String>;
    fn run(&mut self);
    fn stop(&mut self);
}
pub fn connect_pins(source: &Arc<Pin>, destination: &Arc<Pin>) -> Result<(), String> {
    let mut source_connections = source.connections.lock().unwrap();
    source_connections.push(destination.clone());

    let mut dest_connections = destination.connections.lock().unwrap();
    dest_connections.push(source.clone());

    Ok(())
}
// Helper macro for easy connections
#[macro_export]
macro_rules! connect_pin {
    ($source:expr, $dest:expr) => {
        crate::connect_pins($source, $dest).unwrap()
    };
}// Base component implementation
#[derive(Clone)]
pub struct BaseComponent {
    pub name: String,
    pub pins: HashMap<String, Arc<Pin>>,
    pub running: Arc<Mutex<bool>>,
}

impl BaseComponent {
    pub fn new(name: String) -> Self {
        Self {
            name,
            pins: HashMap::new(),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn add_pin(&mut self, name: String, initial_value: PinValue, initial_output_enable: bool) -> Arc<Pin> {
        let pin = Arc::new(Pin {
            name: name.clone(),
            value: Arc::new(RwLock::new(initial_value)),
            output_enable: Arc::new(Mutex::new(initial_output_enable)),
            connections: Arc::new(Mutex::new(Vec::new())),
        });

        self.pins.insert(name, pin.clone());
        pin
    }
}

impl Component for BaseComponent {
    fn name(&self) -> &str {
        &self.name
    }

    fn pins(&self) -> &HashMap<String, Arc<Pin>> {
        &self.pins
    }

    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.pins.get(name).cloned()
    }

    fn connect_pin(&mut self, pin_name: &str, other_pin: Arc<Pin>) -> Result<(), String> {
        if let Some(pin) = self.pins.get_mut(pin_name) {
            let mut connections = pin.connections.lock().unwrap();
            connections.push(other_pin);
            Ok(())
        } else {
            Err(format!("Pin {} not found", pin_name))
        }
    }

    fn update(&mut self) -> Result<(), String> {
        // Base implementation does nothing
        Ok(())
    }

    fn run(&mut self) {
        *self.running.lock().unwrap() = true;
        let running = self.running.clone();
        let mut self_clone = self.clone();

        thread::spawn(move || {
            while *running.lock().unwrap() {
                if let Err(e) = self_clone.update() {
                    eprintln!("Error in {}: {}", self_clone.name, e);
                }
                thread::sleep(Duration::from_micros(10));
            }
        });
    }

    fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 4;
        assert_eq!(result, 4);
    }

    #[test]
    pub fn list_ports() {
        let ports = serialport::available_ports().expect("No ports found!");
        for port in ports {
            let mut port = serialport::new(port.port_name, 115_200)
                .timeout(std::time::Duration::from_millis(10))
                .open().expect("Failed to open port");

            let output = "This is a test. This is only a test.".as_bytes();
            let result = port.write(output);
            assert!(result.is_ok());
        }
    }
}