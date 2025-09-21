use std::collections::HashMap;
use crate::{PinValue, BaseComponent, Component, Pin};
use std::sync::Arc;

// Simple 4-bit and 12-bit types for the 4004
#[derive(Debug, Clone, Copy)]
struct u4(u8);
#[derive(Debug, Clone, Copy)]
struct u12(u16);

impl u4 {
    fn wrapping_add(self, other: u4) -> u4 {
        u4((self.0 + other.0) & 0xF)
    }
}

impl u12 {
    fn wrapping_add(self, other: u12) -> u12 {
        u12((self.0 + other.0) & 0xFFF)
    }
}

#[derive(Clone)]
pub struct Intel4004 {
    base: BaseComponent,
    program_counter: u12,
    accumulator: u4,
    last_clock_state: PinValue,
}

impl Intel4004 {
    pub fn new(name: String) -> Self {
        let mut base = BaseComponent::new(name);

        // Add essential pins
        base.add_pin("clk".to_string(), PinValue::Low, false); // Input pin
        base.add_pin("reset".to_string(), PinValue::Low, false); // Input pin
        base.add_pin("data".to_string(), PinValue::HighZ, false); // Bidirectional

        Self {
            base,
            program_counter: u12(0),
            accumulator: u4(0),
            last_clock_state: PinValue::Low,
        }
    }

    pub fn clk(&self) -> Option<Arc<Pin>> {
        self.base.get_pin("clk")
    }

    pub fn reset(&self) -> Option<Arc<Pin>> {
        self.base.get_pin("reset")
    }

    pub fn data(&self) -> Option<Arc<Pin>> {
        self.base.get_pin("data")
    }
}

impl Component for Intel4004 {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn pins(&self) -> &HashMap<String, Arc<Pin>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.base.get_pin(name)
    }

    fn connect_pin(&mut self, pin_name: &str, other_pin: Arc<Pin>) -> Result<(), String> {
        self.base.connect_pin(pin_name, other_pin)
    }

    fn update(&mut self) -> Result<(), String> {
        // Read clock pin (check if any connected component is driving it low)
        let clk_pin = self.base.pins.get("clk").unwrap();
        let clk_value = clk_pin.read();

        // Detect rising edge
        if clk_value == PinValue::High && self.last_clock_state == PinValue::Low {
            // Simple fetch-execute cycle
            self.program_counter = self.program_counter.wrapping_add(u12(1));

            // Read reset pin
            let reset_pin = self.base.pins.get("reset").unwrap();
            let reset_value = reset_pin.read();

            if reset_value == PinValue::High {
                self.program_counter = u12(0);
                self.accumulator = u4(0);
                println!("CPU {}: RESET", self.name());
            } else {
                // For now, just print state
                println!("CPU {}: PC={:03X}, ACC={:X}",
                         self.name(), self.program_counter.0, self.accumulator.0);
            }
        }

        self.last_clock_state = clk_value;
        Ok(())
    }

    fn run(&mut self) {
        self.base.run();
    }

    fn stop(&mut self) {
        self.base.stop();
    }
}