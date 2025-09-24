use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

/// MOS Technology 6502 CPU - 8-bit microprocessor
pub struct MOS6502 {
    base: BaseComponent,
    // Registers
    accumulator: u8,
    x_register: u8,
    y_register: u8,
    stack_pointer: u8,
    program_counter: u16,
    status_register: u8,

    // Internal state
    cycle_count: u64,
    is_reset: bool,
    is_running: bool,
}

impl MOS6502 {
    pub fn new(name: String) -> Self {
        let pin_names = vec![
            "A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10", "A11", "A12", "A13", "A14", "A15", // 16 address lines
            "D0", "D1", "D2", "D3", "D4", "D5", "D6", "D7", // 8 data lines
            "RW", // Read/Write
            "IRQ", // Interrupt Request
            "NMI", // Non-Maskable Interrupt
            "RES", // Reset
            "CLK", // Clock
            "SYNC", // Sync
            "RDY", // Ready
        ];

        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        MOS6502 {
            base: BaseComponent::new(name, pins),
            accumulator: 0,
            x_register: 0,
            y_register: 0,
            stack_pointer: 0xFD,
            program_counter: 0xFFFC, // Reset vector location
            status_register: 0x20, // Always set bit 5
            cycle_count: 0,
            is_reset: false,
            is_running: false,
        }
    }

    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.x_register = 0;
        self.y_register = 0;
        self.stack_pointer = 0xFD;
        self.program_counter = 0xFFFC;
        self.status_register = 0x20;
        self.is_reset = true;

        // Set initial pin states
        self.set_address_bus(0xFFFC);
        self.set_data_bus(0xFF);
        self.set_rw_pin(true); // Start in read mode
    }

    fn set_address_bus(&self, address: u16) {
        for i in 0..16 {
            if let Ok(pin) = self.base.get_pin(&format!("A{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    let bit_value = (address >> i) & 1;
                    let pin_value = if bit_value == 1 { PinValue::High } else { PinValue::Low };
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), pin_value);
                }
            }
        }
    }

    fn set_data_bus(&self, data: u8) {
        for i in 0..8 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    let bit_value = (data >> i) & 1;
                    let pin_value = if bit_value == 1 { PinValue::High } else { PinValue::Low };
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), pin_value);
                }
            }
        }
    }

    fn set_rw_pin(&self, read: bool) {
        if let Ok(pin) = self.base.get_pin("RW") {
            if let Ok(mut pin_guard) = pin.lock() {
                let pin_value = if read { PinValue::High } else { PinValue::Low };
                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), pin_value);
            }
        }
    }

    fn read_data_bus(&self) -> u8 {
        let mut data = 0;

        for i in 0..8 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(pin_guard) = pin.lock() {
                    if pin_guard.read() == PinValue::High {
                        data |= 1 << i;
                    }
                }
            }
        }

        data
    }

    pub(crate) fn read_control_pins(&self) -> (bool, bool, bool, bool) {
        let irq = if let Ok(pin) = self.base.get_pin("IRQ") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::Low // IRQ is active low
            } else {
                false
            }
        } else {
            false
        };

        let nmi = if let Ok(pin) = self.base.get_pin("NMI") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::Low // NMI is active low
            } else {
                false
            }
        } else {
            false
        };

        let reset = if let Ok(pin) = self.base.get_pin("RES") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::Low // RESET is active low
            } else {
                false
            }
        } else {
            false
        };

        let rdy = if let Ok(pin) = self.base.get_pin("RDY") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High // RDY is active high
            } else {
                true // Default to ready if pin doesn't exist
            }
        } else {
            true
        };

        (irq, nmi, reset, rdy)
    }

    fn execute_instruction(&mut self) {
        // Simplified instruction execution - just increment PC for compilation
        self.program_counter = self.program_counter.wrapping_add(1);
        self.cycle_count += 1;

        // Minimal implementation to satisfy compilation
        self.set_address_bus(self.program_counter);
        self.set_rw_pin(true); // Always reading for now
    }
}

impl Component for MOS6502 {
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
        if !self.is_running {
            return;
        }

        let (_irq, _nmi, reset, rdy) = self.read_control_pins();

        if reset && !self.is_reset {
            self.reset();
            return;
        }

        self.is_reset = reset;

        if rdy {
            self.execute_instruction();
        }
        // If RDY is low, the CPU waits
    }

    fn run(&mut self) {
        self.is_running = true;
        self.reset();

        while self.is_running {
            self.update();
            thread::sleep(Duration::from_micros(1));
        }
    }

    fn stop(&mut self) {
        self.is_running = false;
    }

    fn is_running(&self) -> bool {
        self.is_running
    }
}

// 6502-specific methods
impl MOS6502 {
    pub fn get_program_counter(&self) -> u16 {
        self.program_counter
    }

    pub fn set_program_counter(&mut self, address: u16) {
        self.program_counter = address;
    }

    pub fn get_accumulator(&self) -> u8 {
        self.accumulator
    }

    pub fn get_x_register(&self) -> u8 {
        self.x_register
    }

    pub fn get_y_register(&self) -> u8 {
        self.y_register
    }

    pub fn get_stack_pointer(&self) -> u8 {
        self.stack_pointer
    }

    pub fn get_status_register(&self) -> u8 {
        self.status_register
    }

    pub fn get_cycle_count(&self) -> u64 {
        self.cycle_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_6502_creation() {
        let cpu = MOS6502::new("CPU_6502".to_string());
        assert_eq!(cpu.name(), "CPU_6502");
        assert!(!cpu.is_running());
    }

    #[test]
    fn test_6502_registers() {
        let mut cpu = MOS6502::new("CPU_6502".to_string());

        cpu.set_program_counter(0x1234);
        assert_eq!(cpu.get_program_counter(), 0x1234);

        assert_eq!(cpu.get_accumulator(), 0);
        assert_eq!(cpu.get_x_register(), 0);
        assert_eq!(cpu.get_y_register(), 0);
        assert_eq!(cpu.get_stack_pointer(), 0xFD);
        assert_eq!(cpu.get_status_register(), 0x20);
    }

    #[test]
    fn test_6502_reset() {
        let mut cpu = MOS6502::new("CPU_6502".to_string());

        cpu.set_program_counter(0x1000);
        cpu.reset();

        assert_eq!(cpu.get_program_counter(), 0xFFFC);
    }
}