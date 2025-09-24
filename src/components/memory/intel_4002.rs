use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

/// Intel 4002 - 320-bit RAM (40 nibbles × 4 bits) with integrated output ports
/// Part of the MCS-4 family, designed to work with Intel 4004 CPU
pub struct Intel4002 {
    base: BaseComponent,
    memory: [u8; 40], // 40 nibbles of RAM (stored as bytes for simplicity)
    last_address: u8,
    access_time: Duration,
    last_access: Instant,
    output_ports: [u8; 4], // 4 output ports (4 bits each)
    input_latch: u8,       // Input data latch
    status_character: u8,  // Status character register
    bank_select: u8,       // RAM bank selection
}

impl Intel4002 {
    pub fn new(name: String) -> Self {
        // Intel 4002 has 40 nibbles of RAM (320 bits)

        // Intel 4002 pinout (similar to 4001 but with RAM-specific controls):
        // - 4 data pins (D0-D3) for multiplexed address/data
        // - 4 output port pins (O0-O3)
        // - Control pins: SYNC, CM-ROM, CM-RAM, RESET
        let pin_names = vec![
            "D0", "D1", "D2", "D3",    // Data/Address pins
            "O0", "O1", "O2", "O3",    // Output port pins
            "SYNC",                     // Sync signal
            "CM_ROM",                   // ROM Chip Select
            "CM_RAM",                   // RAM Chip Select
            "RESET",                    // Reset
        ];

        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        Intel4002 {
            base: BaseComponent::new(name, pins),
            memory: [0u8; 40],
            last_address: 0,
            access_time: Duration::from_nanos(500), // 500ns access time
            last_access: Instant::now(),
            output_ports: [0u8; 4],
            input_latch: 0,
            status_character: 0,
            bank_select: 0,
        }
    }

    pub fn initialize_ram(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() > 40 {
            return Err("Data exceeds RAM capacity (40 nibbles)".to_string());
        }

        for (i, &byte) in data.iter().enumerate() {
            self.memory[i] = byte & 0x0F; // Store only lower 4 bits
        }
        Ok(())
    }

    fn read_data_bus(&self) -> u8 {
        let mut data = 0;

        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(pin_guard) = pin.lock() {
                    if pin_guard.read() == PinValue::High {
                        data |= 1 << i;
                    }
                }
            }
        }

        data & 0x0F // Return only lower 4 bits
    }

    fn write_data_bus(&self, data: u8) {
        let nibble = data & 0x0F; // Only lower 4 bits

        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    let bit_value = (nibble >> i) & 1;
                    let pin_value = if bit_value == 1 {
                        PinValue::High
                    } else {
                        PinValue::Low
                    };
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), pin_value);
                }
            }
        }
    }

    fn update_output_ports(&self) {
        for port in 0..4 {
            let port_data = self.output_ports[port];

            if let Ok(pin) = self.base.get_pin(&format!("O{}", port)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    let bit_value = (port_data >> port) & 1; // Each port drives one pin
                    let pin_value = if bit_value == 1 {
                        PinValue::High
                    } else {
                        PinValue::Low
                    };
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), pin_value);
                }
            }
        }
    }

    fn tri_state_data_bus(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                }
            }
        }
    }

    fn read_control_pins(&self) -> (bool, bool, bool, bool) {
        let sync = if let Ok(pin) = self.base.get_pin("SYNC") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        let cm_rom = if let Ok(pin) = self.base.get_pin("CM_ROM") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        let cm_ram = if let Ok(pin) = self.base.get_pin("CM_RAM") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        let reset = if let Ok(pin) = self.base.get_pin("RESET") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        (sync, cm_rom, cm_ram, reset)
    }

    fn handle_reset(&mut self) {
        let (_, _, _, reset) = self.read_control_pins();

        if reset {
            // Reset clears all registers but not RAM content
            self.output_ports = [0u8; 4];
            self.input_latch = 0;
            self.status_character = 0;
            self.bank_select = 0;
            self.tri_state_data_bus();
            self.update_output_ports();
        }
    }

    fn decode_address(&self, address_low: u8, instruction: u8) -> (u8, u8) {
        // Intel 4002 uses a complex addressing scheme:
        // - 4 register banks (selected by bank_select)
        // - 4 status characters per bank
        // - 16 main memory locations per bank

        let bank = self.bank_select & 0x03; // 2-bit bank select
        let ram_address = match instruction {
            // Main memory addressing (locations 0-15)
            0x0..=0xF => (bank * 16) + (address_low & 0x0F),

            // Status character addressing (locations 16-19)
            0x10..=0x13 => 16 + (address_low & 0x03),

            // Output port addressing
            0x14..=0x17 => {
                // Output ports are not in RAM, handled separately
                return (0, instruction - 0x14); // Return (0, port_number)
            }

            // Invalid address
            _ => 0,
        };

        (bank, ram_address)
    }

    fn handle_ram_operation(&mut self) -> bool {
        let (sync, cm_rom, cm_ram, _) = self.read_control_pins();

        // RAM operations occur when SYNC is high and CM-RAM is high
        if sync && cm_ram {
            if cm_rom {
                // Second cycle - data transfer
                let data = self.read_data_bus();

                // Write to previously addressed location
                if self.last_address < 40 {
                    self.memory[self.last_address as usize] = data;
                }

                // For read operations, data would be output in the first cycle
                return true;
            } else {
                // First cycle - address/instruction phase
                let address_instruction = self.read_data_bus();

                // Decode the address based on instruction type
                let (_bank, ram_address) = self.decode_address(self.last_address, address_instruction);

                if ram_address < 40 {
                    // Read from RAM and output data
                    let data = self.memory[ram_address as usize];
                    self.write_data_bus(data);
                    self.last_address = ram_address;
                } else if ram_address >= 0x14 && ram_address <= 0x17 {
                    // Output port operation
                    let port = (ram_address - 0x14) as usize;
                    let data = self.read_data_bus();
                    self.output_ports[port] = data;
                    self.update_output_ports();
                }
            }
        }

        false
    }

    fn handle_instruction(&mut self, instruction: u8) {
        match instruction {
            // Bank select instructions
            0xE0..=0xE3 => {
                self.bank_select = instruction & 0x03;
            }

            // Status character load
            0xF0..=0xF3 => {
                let sc_index = (instruction & 0x03) as usize;
                self.status_character = self.input_latch;
                // Status characters are stored in RAM locations 16-19
                if sc_index < 4 {
                    self.memory[16 + sc_index] = self.status_character;
                }
            }

            // Input latch operations
            0x60..=0x63 => {
                // Read from input (in real hardware, this would read from external pins)
                // For emulation, we use the input latch
                self.write_data_bus(self.input_latch);
            }

            _ => {
                // Other instructions are handled as RAM operations
            }
        }
    }
}

impl Component for Intel4002 {
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
        // Respect access timing
        if self.last_access.elapsed() < self.access_time {
            return;
        }

        self.handle_reset();

        let (sync, cm_rom, cm_ram, _) = self.read_control_pins();

        if sync {
            if cm_ram {
                // RAM operation
                self.handle_ram_operation();
            } else if !cm_rom {
                // Instruction phase (both CM-ROM and CM-RAM low)
                let instruction = self.read_data_bus();
                self.handle_instruction(instruction);
            }
        } else {
            // Not in sync phase - tri-state data bus
            self.tri_state_data_bus();
        }

        self.last_access = Instant::now();
    }

    fn run(&mut self) {
        self.base.set_running(true);

        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10));
        }
    }

    fn stop(&mut self) {
        self.base.set_running(false);

        // Tri-state all outputs when stopped
        self.tri_state_data_bus();
        // Note: Output ports remain driven even when stopped (like real hardware)
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

// Intel 4002 specific methods
impl Intel4002 {
    pub fn get_ram_size(&self) -> usize {
        self.memory.len()
    }

    pub fn read_ram(&self, address: u8) -> Option<u8> {
        if address < 40 {
            Some(self.memory[address as usize])
        } else {
            None
        }
    }

    pub fn write_ram(&mut self, address: u8, data: u8) -> Result<(), String> {
        if address < 40 {
            self.memory[address as usize] = data & 0x0F;
            Ok(())
        } else {
            Err("Address out of range (0-39)".to_string())
        }
    }

    pub fn get_output_port(&self, port: usize) -> Option<u8> {
        if port < 4 {
            Some(self.output_ports[port])
        } else {
            None
        }
    }

    pub fn set_output_port(&mut self, port: usize, data: u8) -> Result<(), String> {
        if port < 4 {
            self.output_ports[port] = data & 0x0F;
            self.update_output_ports();
            Ok(())
        } else {
            Err("Port number out of range (0-3)".to_string())
        }
    }

    pub fn set_input_latch(&mut self, data: u8) {
        self.input_latch = data & 0x0F;
    }

    pub fn get_input_latch(&self) -> u8 {
        self.input_latch
    }

    pub fn get_status_character(&self) -> u8 {
        self.status_character
    }

    pub fn get_bank_select(&self) -> u8 {
        self.bank_select
    }

    pub fn clear_ram(&mut self) {
        self.memory = [0u8; 40];
    }

    pub fn get_ram_bank(&self, bank: u8) -> Vec<u8> {
        let bank = bank & 0x03;
        let start = (bank * 16) as usize;
        let end = start + 16;

        if end <= 40 {
            self.memory[start..end].to_vec()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel4002_creation() {
        let ram = Intel4002::new("RAM_4002".to_string());
        assert_eq!(ram.name(), "RAM_4002");
        assert_eq!(ram.get_ram_size(), 40);
        assert!(!ram.is_running());
    }

    #[test]
    fn test_intel4002_ram_operations() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Test RAM write/read
        assert!(ram.write_ram(0, 0x0A).is_ok());
        assert_eq!(ram.read_ram(0).unwrap(), 0x0A);

        // Test out of bounds
        assert!(ram.write_ram(40, 0x0F).is_err());
        assert!(ram.read_ram(40).is_none());
    }

    #[test]
    fn test_intel4002_output_ports() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Test output port operations
        assert!(ram.set_output_port(0, 0x05).is_ok());
        assert_eq!(ram.get_output_port(0).unwrap(), 0x05);

        // Test invalid port
        assert!(ram.set_output_port(4, 0x0F).is_err());
        assert!(ram.get_output_port(4).is_none());
    }

    #[test]
    fn test_intel4002_input_latch() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        ram.set_input_latch(0x07);
        assert_eq!(ram.get_input_latch(), 0x07);

        // Test that only lower 4 bits are stored
        ram.set_input_latch(0x1F);
        assert_eq!(ram.get_input_latch(), 0x0F);
    }

    #[test]
    fn test_intel4002_ram_banks() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Write test data to different banks
        ram.write_ram(0, 0x01).unwrap();  // Bank 0
        ram.write_ram(16, 0x02).unwrap(); // Bank 1
        ram.write_ram(32, 0x03).unwrap(); // Bank 2

        let bank0 = ram.get_ram_bank(0);
        assert_eq!(bank0[0], 0x01);

        let bank1 = ram.get_ram_bank(1);
        assert_eq!(bank1[0], 0x02);

        let bank2 = ram.get_ram_bank(2);
        assert_eq!(bank2[0], 0x03);
    }

    #[test]
    fn test_intel4002_clear_ram() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        ram.write_ram(0, 0x0A).unwrap();
        ram.write_ram(10, 0x0B).unwrap();

        assert_eq!(ram.read_ram(0).unwrap(), 0x0A);
        assert_eq!(ram.read_ram(10).unwrap(), 0x0B);

        ram.clear_ram();

        assert_eq!(ram.read_ram(0).unwrap(), 0);
        assert_eq!(ram.read_ram(10).unwrap(), 0);
    }
}