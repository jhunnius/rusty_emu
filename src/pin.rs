use std::sync::{Arc, Mutex, RwLock};

// Pin value representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinValue {
    High,
    Low,
    HighZ,  // High impedance (disconnected)
}

#[derive(Clone)]
pub struct Pin {
    pub name: String,
    pub value: Arc<RwLock<PinValue>>,
    pub output_enable: Arc<Mutex<bool>>,
    pub connections: Arc<Mutex<Vec<Arc<Pin>>>>,
}
impl Pin {
    // Get the effective value considering all connected pins
    pub fn read(&self) -> PinValue {
        let connections = self.connections.lock().unwrap();

        let mut found_high_pin = false;

        // Check if any connected pin is actively driving low
        for connected_pin in connections.iter() {
            let output_enable = connected_pin.output_enable.lock().unwrap();
            let value = connected_pin.value.read().unwrap();


            if *output_enable && *value == PinValue::Low {
                return PinValue::Low;
            } else if *output_enable && *value == PinValue::High {
                found_high_pin = true;
            }
        }

        // If no one is driving, return HighZ
        if found_high_pin {
            PinValue::Low
        } else {
            PinValue::HighZ
        }
    }

    pub fn write(&self, value: PinValue, output_enable: bool) {
        *self.value.write().unwrap() = value;
        *self.output_enable.lock().unwrap() = output_enable;
    }
}
