use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::{Component, Pin, PinValue};
pub trait BusDevice: Send {
    /// Called on each simulation cycle to update device state based on pin states
    fn update(&mut self);

    /// Connect this device to bus pins
    fn connect_to_bus(&mut self, bus_pins: Vec<Arc<Pin>>);

    /// Optional: report address range for debugging/monitoring
    fn get_address_range(&self) -> Option<std::ops::Range<u64>> {
        None
    }

    /// Optional: get device name for debugging
    fn get_name(&self) -> &str;
}
#[derive(Clone)]
pub struct GenericBus {
    base: crate::component::BaseComponent,
    connected_devices: Vec<Arc<Mutex<dyn BusDevice>>>,
    bus_pins: HashMap<String, Arc<Pin>>, // Named bus lines (like "A0", "D0", "R/W")
}

impl GenericBus {
    pub fn new(name: String, pin_definitions: &[(&str, PinValue)]) -> Self {
        let mut base = crate::component::BaseComponent::new(name.clone());
        let mut bus_pins = HashMap::new();

        // Create bus pins
        for (pin_name, initial_value) in pin_definitions {
            let pin = base.add_pin(
                pin_name.to_string(),
                *initial_value,
                false, // Bus doesn't drive pins by default
            );
            bus_pins.insert(pin_name.to_string(), pin);
        }

        Self {
            base,
            connected_devices: Vec::new(),
            bus_pins,
        }
    }

    pub fn connect_device(&mut self, device: Arc<Mutex<dyn BusDevice>>) {
        // Get all bus pins and connect them to the device
        let bus_pins: Vec<Arc<Pin>> = self.bus_pins.values().cloned().collect();

        if let Ok(mut device) = device.lock() {
            device.connect_to_bus(bus_pins);
        }

        self.connected_devices.push(device);
    }

    pub fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.bus_pins.get(name).cloned()
    }

    /// The key function: resolve pin conflicts and propagate values
    fn resolve_bus_conflicts(&self) {
        for (pin_name, bus_pin) in &self.bus_pins {
            self.resolve_pin_state(bus_pin);
        }
    }

    fn resolve_pin_state(&self, bus_pin: &Arc<Pin>) {
        let connections = bus_pin.connections.lock().unwrap();

        // Check if any device is driving the pin low (wired-AND behavior)
        let mut any_driving_low = false;
        let mut any_driving_high = false;
        let mut any_driving = false;

        for connected_pin in connections.iter() {
            let output_enable = connected_pin.output_enable.lock().unwrap();
            let value = connected_pin.value.read().unwrap();

            if *output_enable {
                any_driving = true;
                match *value {
                    PinValue::Low => any_driving_low = true,
                    PinValue::High => any_driving_high = true,
                    PinValue::HighZ => {} // High-Z doesn't drive
                }
            }
        }

        // Determine the resolved bus state
        let resolved_value = if any_driving_low {
            // If any device pulls low, bus is low (wired-AND)
            PinValue::Low
        } else if any_driving_high && !any_driving_low {
            // If devices are driving high but no one is pulling low
            PinValue::High
        } else if any_driving {
            // Mixed driving states (shouldn't happen in proper design)
            PinValue::Low // Safe fallback: low wins
        } else {
            // No one is driving - high impedance
            PinValue::HighZ
        };

        // Update the bus pin to reflect the resolved state
        // BUT: the bus itself doesn't drive the pin - it just reflects the resolved state
        // In real hardware, the bus pin's state emerges from the connected devices
        drop(connections);

        // We need to update the pin's value without "driving" it
        // This is tricky because our Pin struct couples value with output enable
        // Let's create a helper method to update the resolved value
        self.update_pin_resolved_value(bus_pin, resolved_value);
    }

    fn update_pin_resolved_value(&self, pin: &Arc<Pin>, value: PinValue) {
        // This is a challenge: we want to set the pin's value without claiming to "drive" it
        // We need to extend our Pin struct to support bus resolution
        let mut current_value = pin.value.write().unwrap();
        *current_value = value;
        // Note: we don't touch output_enable - the bus isn't driving, just reflecting
    }
}

impl Component for GenericBus {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn pins(&self) -> &std::collections::HashMap<String, Arc<Pin>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.base.get_pin(name)
    }

    fn update(&mut self) -> Result<(), String> {
        // Bus update cycle:
        // 1. First, let all devices update based on current pin states
        for device in &self.connected_devices {
            if let Ok(mut device) = device.lock() {
                device.update();
            }
        }

        // 2. Then resolve bus conflicts and propagate pin states
        self.resolve_bus_conflicts();

        Ok(())
    }

    fn run(&mut self) {
        self.base.run();
    }

    fn stop(&mut self) {
        self.base.stop();
    }
}