use std::sync::{Arc, Mutex, RwLock};

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
