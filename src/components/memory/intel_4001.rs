use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component, RunnableComponent};
use crate::pin::{Pin, PinValue};

/// Intel 4001 - 256-byte ROM with integrated I/O
/// Part of the MCS-4 family, designed to work with Intel 4004 CPU
pub struct Intel4001 {
    base: BaseComponent,
    memory: Vec<u8>,
    last_address: u16,
    access_time: Duration,
    last_access: Instant,
    output_latch: u8,
    input_latch: u8,
    io_mode: IoMode,
    // Clock edge detection
    prev_phi1: PinValue,
    prev_phi2: PinValue,
    // Access latency modeling
    address_latched: bool,
    address_latch_time: Option<Instant>,
    data_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum IoMode {
    Input,         // I/O pins as inputs
    Output,        // I/O pins as outputs
    Bidirectional, // I/O pins bidirectional
}

impl Intel4001 {
    pub fn new(name: String) -> Self {
        // Intel 4001 has 256 bytes of ROM
        let rom_size = 256;

        // Intel 4001 pinout (per datasheet):
        // - 4 data pins (D0-D3) for multiplexed address/data
        // - 4 I/O pins (IO0-IO3)
        // - Control pins: SYNC, RESET, CM, CI
        // - Clock pins: Φ1, Φ2 (two-phase clock from 4004 CPU)
        //
        // Control pin behavior:
        // - SYNC: Marks start of instruction cycle
        // - RESET: Clears internal state
        // - CM: Chip select for ROM vs RAM
        // - CI: Distinguishes I/O (LOW) vs ROM access (HIGH) when CM-ROM is HIGH
        let pin_names = vec![
            "D0", "D1", "D2", "D3", // Data/Address pins
            "IO0", "IO1", "IO2", "IO3",    // I/O pins
            "SYNC",   // Sync signal
            "CM",     // CM-ROM: ROM/RAM Chip Select
            "CI",     // CM-RAM: RAM Chip Select (I/O vs ROM access)
            "RESET",  // Reset
            "PHI1",     // Clock phase 1
            "PHI2",     // Clock phase 2
        ];

        let pins = BaseComponent::create_pin_map(&pin_names, &name);
        let memory = vec![0u8; rom_size];

        Intel4001 {
            base: BaseComponent::new(name, pins),
            memory,
            last_address: 0,
            access_time: Duration::from_nanos(500), // 500ns access time
            last_access: Instant::now(),
            output_latch: 0,
            input_latch: 0,
            io_mode: IoMode::Input,
            prev_phi1: PinValue::Low,
            prev_phi2: PinValue::Low,
            address_latched: false,
            address_latch_time: None,
            data_available: false,
        }
    }

    pub fn load_rom_data(&mut self, data: Vec<u8>, offset: usize) -> Result<(), String> {
        if offset + data.len() > self.memory.len() {
            return Err(format!(
                "Data exceeds ROM capacity: offset {} + data length {} > ROM size {}",
                offset,
                data.len(),
                self.memory.len()
            ));
        }

        self.memory[offset..offset + data.len()].copy_from_slice(&data);
        Ok(())
    }

    pub fn load_from_hex(&mut self, hex_data: &str, offset: usize) -> Result<(), String> {
        let bytes: Result<Vec<u8>, _> = hex_data
            .split_whitespace()
            .map(|s| u8::from_str_radix(s.trim(), 16))
            .collect();

        match bytes {
            Ok(data) => self.load_rom_data(data, offset),
            Err(e) => Err(format!("Invalid hex data: {}", e)),
        }
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

        data
    }

    fn write_data_bus(&self, data: u8) {
        for i in 0..4 {
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

    fn read_io_pins(&self) -> u8 {
        let mut data = 0;

        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("IO{}", i)) {
                if let Ok(pin_guard) = pin.lock() {
                    if pin_guard.read() == PinValue::High {
                        data |= 1 << i;
                    }
                }
            }
        }

        data
    }

    fn write_io_pins(&self, data: u8) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("IO{}", i)) {
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

    fn tri_state_data_bus(&self) {
        // CRITICAL: Tri-state data bus to avoid contention with other chips
        // The 4001 must be high-Z whenever not actively driving valid data
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard
                        .set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                }
            }
        }
    }

    fn should_drive_data_bus(&self) -> bool {
        let (sync, cm_rom, cm_ram, _) = self.read_control_pins();
        // ROM drives data bus ONLY when all conditions are met:
        sync && cm_rom && cm_ram && self.data_available
    }

    fn tri_state_io_pins(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("IO{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard
                        .set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                }
            }
        }
    }

    fn read_clock_pins(&self) -> (PinValue, PinValue) {
        let phi1 = if let Ok(pin) = self.base.get_pin("PHI1") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read()
            } else {
                PinValue::Low
            }
        } else {
            PinValue::Low
        };

        let phi2 = if let Ok(pin) = self.base.get_pin("PHI2") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read()
            } else {
                PinValue::Low
            }
        } else {
            PinValue::Low
        };

        (phi1, phi2)
    }

    fn is_phi1_rising_edge(&self) -> bool {
        let (phi1, _) = self.read_clock_pins();
        phi1 == PinValue::High && self.prev_phi1 == PinValue::Low
    }

    fn is_phi1_falling_edge(&self) -> bool {
        let (phi1, _) = self.read_clock_pins();
        phi1 == PinValue::Low && self.prev_phi1 == PinValue::High
    }

    fn is_phi2_rising_edge(&self) -> bool {
        let (_, phi2) = self.read_clock_pins();
        phi2 == PinValue::High && self.prev_phi2 == PinValue::Low
    }

    fn is_phi2_falling_edge(&self) -> bool {
        let (_, phi2) = self.read_clock_pins();
        phi2 == PinValue::Low && self.prev_phi2 == PinValue::High
    }

    fn update_clock_states(&mut self) {
        let (phi1, phi2) = self.read_clock_pins();
        self.prev_phi1 = phi1;
        self.prev_phi2 = phi2;
    }

    fn read_control_pins(&self) -> (bool, bool, bool, bool) {
        // SYNC: Marks the start of an instruction cycle
        let sync = if let Ok(pin) = self.base.get_pin("SYNC") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        // CM-ROM: Chip select for ROM vs RAM (HIGH = ROM access)
        let cm_rom = if let Ok(pin) = self.base.get_pin("CM") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        // CM-RAM: Distinguishes I/O vs ROM access when CM-ROM is HIGH
        // HIGH = ROM access, LOW = I/O access
        let cm_ram = if let Ok(pin) = self.base.get_pin("CI") {
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
            // RESET is high - clear internal state
            self.output_latch = 0;
            self.input_latch = 0;
            self.io_mode = IoMode::Input;
            self.tri_state_data_bus();
            self.tri_state_io_pins();
            // Reset access latency state
            self.address_latched = false;
            self.address_latch_time = None;
            self.data_available = false;
        }
    }

    fn handle_io_operation(&mut self) {
        let (sync, cm_rom, cm_ram, _) = self.read_control_pins();

        // I/O operations occur when SYNC is high and CM-ROM is low
        if sync && !cm_rom {
            let _address_low = self.read_data_bus(); // Lower 4 bits of address from data bus

            // For I/O operations, the upper address bits determine the operation
            if cm_ram {
                // I/O read operation
                self.input_latch = self.read_io_pins();
                self.write_data_bus(self.input_latch);
            } else {
                // I/O write operation
                let data = self.read_data_bus();
                self.output_latch = data;
                self.write_io_pins(data);
                self.io_mode = IoMode::Output;
            }
        }
    }

    fn handle_memory_operation(&mut self) -> bool {
        let (sync, cm_rom, cm_ram, _) = self.read_control_pins();

        // TRI-STATE RULE: ROM only drives data bus when ALL of these are true:
        // - SYNC=1 (instruction cycle active)
        // - CM-ROM=1 (ROM chip selected)
        // - CM-RAM=1 (data phase, not address phase)
        // - data_available=true (access latency has elapsed)
        //
        // ALL other cases MUST be high-Z to avoid bus contention with CPU/other chips

        if self.should_drive_data_bus() {
            // EXACT CONDITIONS: SYNC=1, CM-ROM=1, CM-RAM=1, and data available
            // This is the ONLY case where ROM drives the data bus
            let address = self.last_address;
            if (address as usize) < self.memory.len() {
                let data = self.memory[address as usize];
                self.write_data_bus(data);
                return true; // Successfully drove the bus
            }
        }

        // ALL other cases: tri-state the data bus
        self.tri_state_data_bus();

        // Handle address latching when in address phase (but still tri-state)
        if sync && cm_rom && !cm_ram {
            // First cycle: CPU outputs address, ROM latches it
            let address_low = self.read_data_bus();
            self.last_address = address_low as u16;
            self.address_latched = true;
            self.address_latch_time = Some(Instant::now());
            self.data_available = false;
        } else if !self.address_latched {
            // Reset data available flag when not in active operation
            self.data_available = false;
        }

        false
    }
}

impl Component for Intel4001 {
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
        // Only update on PHI1 rising edge (synchronous with CPU clock)
        if !self.is_phi1_rising_edge() {
            return;
        }

        // Update clock states for next edge detection
        let (phi1, phi2) = self.read_clock_pins();
        self.prev_phi1 = phi1;
        self.prev_phi2 = phi2;

        // Check if we have a latched address and need to make data available
        if self.address_latched {
            if let Some(latch_time) = self.address_latch_time {
                if latch_time.elapsed() >= self.access_time {
                    // Access latency has elapsed, data is now available
                    self.data_available = true;
                    self.address_latched = false;
                    self.address_latch_time = None;
                }
            }
        }

        self.handle_reset();

        // Handle I/O operations (higher priority than memory operations)
        self.handle_io_operation();

        // Handle memory operations
        let memory_accessed = self.handle_memory_operation();

        // Additional safety: ensure data bus is tri-stated when ROM is not actively driving
        // This handles any edge cases not covered in handle_memory_operation
        if !memory_accessed && !self.should_drive_data_bus() {
            self.tri_state_data_bus();
        }

        self.last_access = Instant::now();
    }

    fn run(&mut self) {
        // Time-slice model: run in a loop calling update() each cycle
        self.base.set_running(true);

        // Initialize clock states
        let (phi1, phi2) = self.read_clock_pins();
        self.prev_phi1 = phi1;
        self.prev_phi2 = phi2;

        while self.is_running() {
            self.update();
            // Small delay to prevent busy waiting when no clock is present
            std::thread::sleep(std::time::Duration::from_micros(1));
        }
    }

    fn stop(&mut self) {
        self.base.set_running(false);

        // Tri-state all outputs when stopped
        self.tri_state_data_bus();
        self.tri_state_io_pins();

        // Reset access latency state
        self.address_latched = false;
        self.address_latch_time = None;
        self.data_available = false;
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

impl RunnableComponent for Intel4001 {}

// Intel 4001 specific methods
impl Intel4001 {
    pub fn get_rom_size(&self) -> usize {
        self.memory.len()
    }

    pub fn read_rom(&self, address: u8) -> Option<u8> {
        if (address as usize) < self.memory.len() {
            Some(self.memory[address as usize])
        } else {
            None
        }
    }

    pub fn get_output_latch(&self) -> u8 {
        self.output_latch
    }

    pub fn get_input_latch(&self) -> u8 {
        self.input_latch
    }

    fn get_io_mode(&self) -> IoMode {
        self.io_mode
    }

    fn set_io_mode(&mut self, mode: IoMode) {
        self.io_mode = mode;
        match mode {
            IoMode::Input => self.tri_state_io_pins(),
            IoMode::Output => self.write_io_pins(self.output_latch),
            IoMode::Bidirectional => {
                // In bidirectional mode, I/O pins follow the data bus during writes
            }
        }
    }
}

// Custom formatter for debugging
impl std::fmt::Display for IoMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoMode::Input => write!(f, "Input"),
            IoMode::Output => write!(f, "Output"),
            IoMode::Bidirectional => write!(f, "Bidirectional"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel4001_creation() {
        let rom = Intel4001::new("ROM_4001".to_string());
        assert_eq!(rom.name(), "ROM_4001");
        assert_eq!(rom.get_rom_size(), 256);
        assert!(!rom.is_running());
    }

    #[test]
    fn test_intel4001_rom_loading() {
        let mut rom = Intel4001::new("ROM_4001".to_string());

        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        assert!(rom.load_rom_data(test_data.clone(), 0).is_ok());

        assert_eq!(rom.read_rom(0).unwrap(), 0x12);
        assert_eq!(rom.read_rom(1).unwrap(), 0x34);
        assert_eq!(rom.read_rom(2).unwrap(), 0x56);
        assert_eq!(rom.read_rom(3).unwrap(), 0x78);
    }

    #[test]
    fn test_intel4001_io_modes() {
        let mut rom = Intel4001::new("ROM_4001".to_string());

        assert_eq!(rom.get_io_mode(), IoMode::Input);

        rom.set_io_mode(IoMode::Output);
        assert_eq!(rom.get_io_mode(), IoMode::Output);

        rom.set_io_mode(IoMode::Bidirectional);
        assert_eq!(rom.get_io_mode(), IoMode::Bidirectional);
    }

    #[test]
    fn test_intel4001_latches() {
        let rom = Intel4001::new("ROM_4001".to_string());

        // Initial state
        assert_eq!(rom.get_output_latch(), 0);
        assert_eq!(rom.get_input_latch(), 0);

        // These would be set during actual I/O operations
        // The test verifies the latch structures exist
    }

    #[test]
    fn test_io_mode_display() {
        assert_eq!(IoMode::Input.to_string(), "Input");
        assert_eq!(IoMode::Output.to_string(), "Output");
        assert_eq!(IoMode::Bidirectional.to_string(), "Bidirectional");
    }
}
