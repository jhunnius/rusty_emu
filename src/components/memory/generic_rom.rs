use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use crate::{Component, Pin, PinValue};
use crate::bus::BusDevice;

pub struct GenericRom<const SIZE: usize, const DATA_WIDTH: usize> {
    pub(crate) base: crate::component::BaseComponent,
    memory: HashMap<u64, u64>, // Generic memory storage
    data_width: usize,
    address_range: std::ops::Range<u64>,
    pub latched_address: ()
}

impl<const SIZE: usize, const DATA_WIDTH: usize> GenericRom<SIZE, DATA_WIDTH> {
    pub fn new(name: String, base_address: u64) -> Self {
        let mut base = crate::component::BaseComponent::new(name);

        // Add data pins based on width
        for i in 0..DATA_WIDTH {
            base.add_pin(format!("data_{}", i), PinValue::HighZ, false);
        }

        // Add control pins
        base.add_pin("cs".to_string(), PinValue::Low, false);
        base.add_pin("oe".to_string(), PinValue::Low, false);

        let address_range = base_address..base_address + SIZE as u64;

        Self {
            base,
            memory: HashMap::new(),
            data_width: DATA_WIDTH,
            address_range,
            latched_address: (),
        }
    }

    pub fn load_data(&mut self, data: &[u64]) {
        for (i, &word) in data.iter().enumerate() {
            if i < SIZE {
                self.memory.insert(self.address_range.start + i as u64, word);
            }
        }
    }
    pub fn is_selected(&self, address: u64) -> bool {
        if let Some(cs_pin) = self.base.get_pin("cs") {
            cs_pin.read() == PinValue::High && self.address_range.contains(&address)
        } else {
            false
        }
    }
    fn get_current_address(&self) -> u64 {
        // In a real system, this would read address from connected address pins
        // For now, return a default address
        0
    }

    fn output_data(&self, data: u64) {
        for i in 0..DATA_WIDTH {
            if let Some(pin) = self.base.get_pin(&format!("data_{}", i)) {
                let bit_value = (data >> i) & 1;
                let pin_value = if bit_value == 1 { PinValue::High } else { PinValue::Low };
                pin.write(pin_value, true); // Drive the pin
            }
        }
    }

    fn set_data_pins_high_z(&self) {
        for i in 0..DATA_WIDTH {
            if let Some(pin) = self.base.get_pin(&format!("data_{}", i)) {
                pin.write(PinValue::HighZ, false); // High impedance, not driving
            }
        }
    }
}

impl<const SIZE: usize, const DATA_WIDTH: usize> BusDevice for GenericRom<SIZE, DATA_WIDTH> {
    fn update(&mut self) {
        // Check if we're selected and output is enabled
        if let (Some(cs_pin), Some(oe_pin)) = (self.base.get_pin("cs"), self.base.get_pin("oe")) {
            let is_selected = cs_pin.read() == PinValue::High;
            let output_enabled = oe_pin.read() == PinValue::Low; // Active low

            if is_selected && output_enabled {
                // Read address from address pins (simplified - in real system, address would come from bus)
                // For now, we'll use a dummy address - this would be connected to address pins in a real system
                let address = self.get_current_address();

                // Read data from ROM
                let data = self.read(address);

                // Output data to data pins
                self.output_data(data);
            } else {
                // Put data pins in high impedance when not selected
                self.set_data_pins_high_z();
            }
        }
    }

    fn connect_to_bus(&mut self, bus_pins: Vec<Arc<Pin>>) {
        // Connect data pins
        for (i, bus_pin) in bus_pins.iter().enumerate() {
            if let Some(rom_pin) = self.base.get_pin(&format!("data_{}", i)) {
                crate::connection::connect_pins(&rom_pin, bus_pin).unwrap();
            }
        }
    }

    fn get_address_range(&self) -> Option<std::ops::Range<u64>> {
        Some(self.address_range.clone())
    }

    fn get_name(&self) -> &str {
        todo!()
    }
}