use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

pub struct GenericRam {
    base: BaseComponent,
    memory: Vec<u8>,
    address_width: usize,
    data_width: usize,
    last_address: u32,
    last_operation: RamOperation,
    access_time: Duration,
    last_access: Instant,
    write_enable: bool,
    output_enable: bool,
    chip_select: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RamOperation {
    Read,
    Write,
    Idle,
}

impl GenericRam {
    pub fn new(name: String, size: usize, address_width: usize, data_width: usize) -> Self {
        let mut pin_names = Vec::new();

        // Address pins (A0, A1, A2, ...)
        for i in 0..address_width {
            pin_names.push(format!("A{}", i));
        }

        // Data pins (D0, D1, D2, ...) - bidirectional
        for i in 0..data_width {
            let name = format!("D{}", i);
            pin_names.push(name.clone());
        }

        // Control pins
        pin_names.push("CS".parse().unwrap());    // Chip Select
        pin_names.push("WE".parse().unwrap());    // Write Enable
        pin_names.push("OE".parse().unwrap());    // Output Enable

        let pin_refs: Vec<&str> = pin_names.iter().map(|s| s.as_str()).collect();
        let pins = BaseComponent::create_pin_map(&pin_refs, &name);
        let memory = vec![0u8; size];

        GenericRam {
            base: BaseComponent::new(name, pins),
            memory,
            address_width,
            data_width,
            last_address: 0,
            last_operation: RamOperation::Idle,
            access_time: Duration::from_nanos(100), // 100ns access time
            last_access: Instant::now(),
            write_enable: false,
            output_enable: false,
            chip_select: false,
        }
    }

    pub fn load_data(&mut self, data: Vec<u8>, offset: usize) -> Result<(), String> {
        if offset + data.len() > self.memory.len() {
            return Err(format!(
                "Data exceeds RAM capacity: offset {} + data length {} > RAM size {}",
                offset, data.len(), self.memory.len()
            ));
        }

        self.memory[offset..offset + data.len()].copy_from_slice(&data);
        Ok(())
    }

    pub fn load_from_binary(&mut self, data: &[u8], offset: usize) -> Result<(), String> {
        if offset + data.len() > self.memory.len() {
            return Err(format!(
                "Data exceeds RAM capacity: offset {} + data length {} > RAM size {}",
                offset, data.len(), self.memory.len()
            ));
        }

        self.memory[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn get_memory_size(&self) -> usize {
        self.memory.len()
    }

    pub fn read_memory(&self, address: usize, length: usize) -> Result<Vec<u8>, String> {
        if address + length > self.memory.len() {
            return Err("Address range out of bounds".to_string());
        }

        Ok(self.memory[address..address + length].to_vec())
    }

    pub fn write_memory(&mut self, address: usize, data: &[u8]) -> Result<(), String> {
        if address + data.len() > self.memory.len() {
            return Err("Address range out of bounds".to_string());
        }

        self.memory[address..address + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn get_access_time(&self) -> Duration {
        self.access_time
    }

    pub fn set_access_time(&mut self, access_time: Duration) {
        self.access_time = access_time;
    }

    fn read_address(&self) -> u32 {
        let mut address = 0;

        for i in 0..self.address_width {
            if let Ok(pin) = self.base.get_pin(&format!("A{}", i)) {
                if let Ok(pin_guard) = pin.lock() {
                    if pin_guard.read() == PinValue::High {
                        address |= 1 << i;
                    }
                }
            }
        }

        address
    }

    fn read_data_bus(&self) -> u8 {
        let mut data = 0;

        for i in 0..self.data_width {
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

    fn write_data_bus(&self, data: u8) {
        for i in 0..self.data_width {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    let bit_value = (data >> i) & 1;
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

    fn read_control_pins(&mut self) {
        // Read Chip Select (active low)
        self.chip_select = if let Ok(cs_pin) = self.base.get_pin("CS") {
            if let Ok(cs_guard) = cs_pin.lock() {
                cs_guard.read() == PinValue::Low
            } else {
                false
            }
        } else {
            false
        };

        // Read Write Enable (active low)
        self.write_enable = if let Ok(we_pin) = self.base.get_pin("WE") {
            if let Ok(we_guard) = we_pin.lock() {
                we_guard.read() == PinValue::Low
            } else {
                false
            }
        } else {
            false
        };

        // Read Output Enable (active low)
        self.output_enable = if let Ok(oe_pin) = self.base.get_pin("OE") {
            if let Ok(oe_guard) = oe_pin.lock() {
                oe_guard.read() == PinValue::Low
            } else {
                false
            }
        } else {
            false
        };
    }

    fn tri_state_data_bus(&self) {
        for i in 0..self.data_width {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                }
            }
        }
    }

    fn perform_read_operation(&mut self, address: u32) {
        if (address as usize) < self.memory.len() {
            let data = self.memory[address as usize];
            self.write_data_bus(data);
            self.last_operation = RamOperation::Read;
        } else {
            // Address out of bounds - tri-state
            self.tri_state_data_bus();
        }
    }

    fn perform_write_operation(&mut self, address: u32) {
        if (address as usize) < self.memory.len() {
            let data = self.read_data_bus();
            self.memory[address as usize] = data;
            self.last_operation = RamOperation::Write;
        }
        // During write, RAM doesn't drive the data bus
        self.tri_state_data_bus();
    }
}

impl Component for GenericRam {
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

        self.read_control_pins();

        if !self.chip_select {
            // Chip not selected - tri-state all outputs
            self.tri_state_data_bus();
            self.last_operation = RamOperation::Idle;
            return;
        }

        let current_address = self.read_address();
        let address_changed = current_address != self.last_address;

        // Determine operation based on control pins
        if self.write_enable {
            // Write operation (WE low)
            self.perform_write_operation(current_address);
        } else if self.output_enable {
            // Read operation (OE low, WE high)
            if address_changed || self.last_operation != RamOperation::Read {
                self.perform_read_operation(current_address);
            }
        } else {
            // Output disabled - tri-state data bus
            self.tri_state_data_bus();
            self.last_operation = RamOperation::Idle;
        }

        self.last_address = current_address;
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

        // Tri-state all data pins when stopped
        self.tri_state_data_bus();
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

// Additional utility methods
impl GenericRam {
    pub fn clear_memory(&mut self) {
        for byte in &mut self.memory {
            *byte = 0;
        }
    }

    pub fn fill_memory(&mut self, value: u8) {
        for byte in &mut self.memory {
            *byte = value;
        }
    }

    pub fn get_memory_snapshot(&self) -> Vec<u8> {
        self.memory.clone()
    }

    pub fn restore_memory_snapshot(&mut self, snapshot: Vec<u8>) -> Result<(), String> {
        if snapshot.len() != self.memory.len() {
            return Err("Snapshot size doesn't match RAM size".to_string());
        }

        self.memory.copy_from_slice(&snapshot);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_creation() {
        let ram = GenericRam::new("TEST_RAM".to_string(), 1024, 10, 8);
        assert_eq!(ram.name(), "TEST_RAM");
        assert_eq!(ram.memory.len(), 1024);
        assert_eq!(ram.address_width, 10);
        assert_eq!(ram.data_width, 8);
    }

    #[test]
    fn test_ram_data_loading() {
        let mut ram = GenericRam::new("TEST_RAM".to_string(), 256, 8, 8);

        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        assert!(ram.load_data(test_data.clone(), 0).is_ok());

        assert_eq!(ram.memory[0], 0x12);
        assert_eq!(ram.memory[1], 0x34);
        assert_eq!(ram.memory[2], 0x56);
        assert_eq!(ram.memory[3], 0x78);
    }

    #[test]
    fn test_ram_read_write() {
        let mut ram = GenericRam::new("TEST_RAM".to_string(), 256, 8, 8);

        // Test direct memory access
        assert!(ram.write_memory(0x10, &[0xAA, 0xBB, 0xCC]).is_ok());

        let read_data = ram.read_memory(0x10, 3).unwrap();
        assert_eq!(read_data, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_ram_clear_and_fill() {
        let mut ram = GenericRam::new("TEST_RAM".to_string(), 256, 8, 8);

        // Fill with pattern
        ram.fill_memory(0x55);
        assert_eq!(ram.memory[0], 0x55);
        assert_eq!(ram.memory[100], 0x55);

        // Clear memory
        ram.clear_memory();
        assert_eq!(ram.memory[0], 0);
        assert_eq!(ram.memory[100], 0);
    }

    #[test]
    fn test_ram_snapshot() {
        let mut ram = GenericRam::new("TEST_RAM".to_string(), 256, 8, 8);

        // Write some data
        ram.write_memory(0, &[0x11, 0x22, 0x33]).unwrap();

        // Take snapshot
        let snapshot = ram.get_memory_snapshot();

        // Modify memory
        ram.write_memory(0, &[0x44, 0x55, 0x66]).unwrap();

        // Restore snapshot
        ram.restore_memory_snapshot(snapshot).unwrap();

        // Verify restoration
        let read_data = ram.read_memory(0, 3).unwrap();
        assert_eq!(read_data, vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn test_ram_is_running() {
        let ram = GenericRam::new("TEST_RAM".to_string(), 256, 8, 8);
        assert!(!ram.is_running());
    }
}