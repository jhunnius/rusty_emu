use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

pub struct GenericRom {
    pub(crate) base: BaseComponent,
    memory: Vec<u8>,
    address_width: usize,
    data_width: usize,
    last_address: u32,
    access_time: Duration,
    last_access: Instant,
}

impl GenericRom {
    pub fn new(name: String, size: usize, address_width: usize, data_width: usize) -> Self {
        let mut pins = HashMap::new();

        // Create address pins (A0, A1, A2, ...)
        for i in 0..address_width {
            pins.insert(format!("A{}", i), Arc::new(Mutex::new(Pin::new(format!("{}_A{}", name, i)))));
        }

        // Create data pins (D0, D1, D2, ...)
        for i in 0..data_width {
            pins.insert(format!("D{}", i), Arc::new(Mutex::new(Pin::new(format!("{}_D{}", name, i)))));
        }

        // Control pins
        pins.insert("CS".to_string(), Arc::new(Mutex::new(Pin::new(format!("{}_CS", name)))));
        pins.insert("OE".to_string(), Arc::new(Mutex::new(Pin::new(format!("{}_OE", name)))));

        let memory = vec![0u8; size];

        GenericRom {
            base: BaseComponent::new(name, pins),
            memory,
            address_width,
            data_width,
            last_address: 0,
            access_time: Duration::from_nanos(100), // 100ns access time
            last_access: Instant::now(),
        }
    }

    pub fn load_data(&mut self, data: Vec<u8>, offset: usize) -> Result<(), String> {
        if offset + data.len() > self.memory.len() {
            return Err("Data exceeds ROM capacity".to_string());
        }

        self.memory[offset..offset + data.len()].copy_from_slice(&data);
        Ok(())
    }

    pub fn load_from_hex(&mut self, hex_data: &str, offset: usize) -> Result<(), String> {
        let bytes: Result<Vec<u8>, _> = hex_data
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16))
            .collect();

        match bytes {
            Ok(data) => self.load_data(data, offset),
            Err(_) => Err("Invalid hex data".to_string()),
        }
    }
    pub fn load_from_binary_file<P: AsRef<Path>>(&mut self, path: P, offset: usize) -> Result<(), String> {
        let path_ref = path.as_ref();

        // Check if file exists
        if !path_ref.exists() {
            return Err(format!("File not found: {}", path_ref.display()));
        }

        // Check if offset is within bounds
        if offset >= self.memory.len() {
            return Err(format!("Offset {} exceeds ROM size {}", offset, self.memory.len()));
        }

        let mut file = File::open(path_ref)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        // Get file size
        let metadata = file.metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        let file_size = metadata.len() as usize;

        // Check if file fits in ROM
        if offset + file_size > self.memory.len() {
            return Err(format!(
                "File too large: offset {} + file size {} > ROM size {}",
                offset, file_size, self.memory.len()
            ));
        }

        // Read file into buffer
        let mut buffer = vec![0u8; file_size];
        file.read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Copy into ROM memory
        self.memory[offset..offset + file_size].copy_from_slice(&buffer);

        Ok(())
    }

    pub fn load_from_binary(&mut self, data: &[u8], offset: usize) -> Result<(), String> {
        if offset + data.len() > self.memory.len() {
            return Err(format!(
                "Data exceeds ROM capacity: offset {} + data length {} > ROM size {}",
                offset, data.len(), self.memory.len()
            ));
        }

        self.memory[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
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

    pub(crate) fn is_selected(&self) -> bool {
        // Check Chip Select (active low by convention)
        if let Ok(cs_pin) = self.base.get_pin("CS") {
            if let Ok(cs_guard) = cs_pin.lock() {
                if cs_guard.read() == PinValue::High {
                    return false; // Not selected
                }
            }
        }
        true // Selected when CS is low or not connected
    }

    fn output_enabled(&self) -> bool {
        // Check Output Enable (active low by convention)
        if let Ok(oe_pin) = self.base.get_pin("OE") {
            if let Ok(oe_guard) = oe_pin.lock() {
                if oe_guard.read() == PinValue::High {
                    return false; // Output disabled
                }
            }
        }
        true // Enabled when OE is low or not connected
    }

    pub(crate) fn output_data(&self, data: u8) {
        // Only drive data pins if selected and output enabled
        if !self.is_selected() || !self.output_enabled() {
            // Tri-state the outputs
            for i in 0..self.data_width {
                if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                    if let Ok(mut pin_guard) = pin.lock() {
                        pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                    }
                }
            }
            return;
        }

        // Drive data pins according to the data value
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
}

impl Component for GenericRom {
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

        let current_address = self.read_address();

        // Only process if address changed or we need to update outputs
        if current_address != self.last_address || !self.is_selected() || !self.output_enabled() {
            if self.is_selected() && self.output_enabled() {
                // Read from memory (handle address bounds)
                if (current_address as usize) < self.memory.len() {
                    let data = self.memory[current_address as usize];
                    self.output_data(data);
                } else {
                    // Address out of bounds - tri-state outputs
                    for i in 0..self.data_width {
                        if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                            if let Ok(mut pin_guard) = pin.lock() {
                                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                            }
                        }
                    }
                }
            } else {
                // Not selected or output disabled - tri-state outputs
                for i in 0..self.data_width {
                    if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                        if let Ok(mut pin_guard) = pin.lock() {
                            pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                        }
                    }
                }
            }

            self.last_address = current_address;
            self.last_access = Instant::now();
        }
    }

    fn run(&mut self) {
        self.base.set_running(true);

        while self.base.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(1)); // Small delay to prevent busy waiting
        }
    }

    fn stop(&mut self) {
        self.base.set_running(false);

        // Tri-state all outputs when stopped
        for i in 0..self.data_width {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                }
            }
        }
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::NamedTempFile;
    use super::*;

    #[test]
    fn test_rom_creation() {
        let rom = GenericRom::new("TEST_ROM".to_string(), 1024, 10, 8);
        assert_eq!(rom.name(), "TEST_ROM");
        assert_eq!(rom.memory.len(), 1024);
    }

    #[test]
    fn test_rom_file_loading() {
        let mut rom = GenericRom::new("TEST_ROM".to_string(), 256, 8, 8);

        // Create a temporary file with test data
        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), &test_data).unwrap();

        assert!(rom.load_from_binary_file(temp_file.path(), 0).is_ok());

        assert_eq!(rom.memory[0], 0x12);
        assert_eq!(rom.memory[1], 0x34);
        assert_eq!(rom.memory[2], 0x56);
        assert_eq!(rom.memory[3], 0x78);
    }

    #[test]
    fn test_rom_data_loading() {
        let mut rom = GenericRom::new("TEST_ROM".to_string(), 256, 8, 8);

        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        assert!(rom.load_data(test_data.clone(), 0).is_ok());

        assert_eq!(rom.memory[0], 0x12);
        assert_eq!(rom.memory[1], 0x34);
        assert_eq!(rom.memory[2], 0x56);
        assert_eq!(rom.memory[3], 0x78);
    }

    #[test]
    fn test_rom_hex_loading() {
        let mut rom = GenericRom::new("TEST_ROM".to_string(), 256, 8, 8);

        assert!(rom.load_from_hex("12 34 56 78", 0).is_ok());

        assert_eq!(rom.memory[0], 0x12);
        assert_eq!(rom.memory[1], 0x34);
        assert_eq!(rom.memory[2], 0x56);
        assert_eq!(rom.memory[3], 0x78);
    }
}