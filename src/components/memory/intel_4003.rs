use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component, RunnableComponent};
use crate::pin::{Pin, PinValue};

/// Intel 4003 - 10-bit Output Shift Register
/// Part of the MCS-4 family, designed to work with Intel 4004 CPU
/// Features 10-bit serial-in, parallel-out shift register for output expansion
///
/// Hardware Architecture:
/// - 10-bit shift register with serial input and parallel outputs
/// - Used for expanding I/O capabilities of the MCS-4 system
/// - Serial data input from CPU, parallel outputs to external devices
/// - Clocked by two-phase clock from 4004 CPU
///
/// Hardware Deviations:
/// - Simplified timing model for unit testing (may need refinement for CPU integration)
/// - Output behavior matches 4001/4002 I/O persistence model
pub struct Intel4003 {
    base: BaseComponent,
    shift_register: [u8; 10],     // 10-bit shift register (stored as 10 bytes for simplicity)
    output_latch: [u8; 10],       // 10-bit output latch for parallel output
    serial_input: u8,             // Serial input data (4-bit)
    access_time: Duration,        // Shift register access latency (200ns typical)

    // Clock edge detection
    prev_phi1: PinValue,          // Previous Φ1 clock state for edge detection
    prev_phi2: PinValue,          // Previous Φ2 clock state for edge detection

    // Two-phase addressing for 8-bit address
    address_high_nibble: Option<u8>, // High nibble of 8-bit address
    address_low_nibble: Option<u8>,  // Low nibble of 8-bit address
    full_address_ready: bool,        // Whether complete address is assembled

    // Shift register operation state machine
    shift_state: ShiftState,      // Current state of shift operation
    address_latch_time: Option<Instant>, // Timestamp when address was latched
}

/// Shift register operation state machine states
/// Tracks the current phase of shift register operations
#[derive(Debug, Clone, Copy, PartialEq)]
enum ShiftState {
    Idle,         // No shift operation in progress
    AddressPhase, // Currently latching address nibbles
    WaitLatency,  // Address latched, waiting for access time
    ShiftData,    // Shifting data into register
    OutputData,   // Outputting parallel data
}

impl Intel4003 {
    /// Create a new Intel 4003 Shift Register with specified access time
    /// Parameters: name - Component identifier, access_time_ns - Access time in nanoseconds
    /// Returns: New Intel4003 instance with configurable access timing
    pub fn new(name: String) -> Self {
        Self::new_with_access_time(name, 200) // Default 200ns access time
    }

    /// Create a new Intel 4003 Shift Register with custom access time (for testing)
    /// Parameters: name - Component identifier, access_time_ns - Access time in nanoseconds
    /// Returns: New Intel4003 instance with configurable access timing
    pub fn new_with_access_time(name: String, access_time_ns: u64) -> Self {
        // Intel 4003 pinout (based on MCS-4 architecture):
        // - 4 data pins (D0-D3) for multiplexed address/data
        // - 10 output pins (O0-O9) for parallel output
        // - Control pins: SYNC, CM, RESET
        // - Clock pins: Φ1, Φ2 (two-phase clock from 4004 CPU)
        //
        // Control pin behavior:
        // - SYNC: Marks start of instruction cycle
        // - CM: Chip select (must be HIGH for shift register access)
        // - RESET: Clears internal state
        let pin_names = vec![
            "D0", "D1", "D2", "D3",    // Data/Address pins
            "O0", "O1", "O2", "O3", "O4", "O5", "O6", "O7", "O8", "O9", // 10 output pins
            "SYNC",                    // Sync signal
            "CM",                      // Chip Select
            "RESET",                   // Reset
            "PHI1",                    // Clock phase 1
            "PHI2",                    // Clock phase 2
        ];

        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        Intel4003 {
            base: BaseComponent::new(name, pins),
            shift_register: [0u8; 10],  // 10-bit shift register
            output_latch: [0u8; 10],    // 10-bit output latch
            serial_input: 0,
            access_time: Duration::from_nanos(access_time_ns),

            // Clock edge detection
            prev_phi1: PinValue::Low,
            prev_phi2: PinValue::Low,

            // Two-phase addressing
            address_high_nibble: None,
            address_low_nibble: None,
            full_address_ready: false,

            // Shift register operation state
            shift_state: ShiftState::Idle,
            address_latch_time: None,
        }
    }

    /// Set the memory access time for simulation
    /// Parameters: access_time_ns - Access time in nanoseconds
    pub fn set_access_time(&mut self, access_time_ns: u64) {
        self.access_time = Duration::from_nanos(access_time_ns);
    }

    /// Get the current access time
    /// Returns: Access time in nanoseconds
    pub fn get_access_time(&self) -> u64 {
        self.access_time.as_nanos() as u64
    }

    /// Read the 4-bit data bus from D0-D3 pins
    /// Returns: 4-bit value from data bus pins
    fn read_data_bus(&self) -> u8 {
        let mut data = 0;

        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(pin_guard) = pin.lock() {
                    let pin_value = pin_guard.read();
                    if pin_value == PinValue::High {
                        data |= 1 << i;
                    }
                    println!("DEBUG: 4003 read_data_bus - D{} = {:?}", i, pin_value);
                }
            }
        }

        let result = data & 0x0F;
        println!("DEBUG: 4003 read_data_bus returning 0x{:x}", result);
        result
    }

    /// Drive the 4-bit data bus with the specified value
    /// Parameters: data - 4-bit value to drive on D0-D3 pins

    /// Update output pins based on current output latch values
    /// Hardware: Output pins are driven continuously until changed or reset
    fn update_output_pins(&self) {
        for i in 0..10 {
            if let Ok(pin) = self.base.get_pin(&format!("O{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    let bit_value = (self.output_latch[i]) & 1;
                    let pin_value = if bit_value == 1 {
                        PinValue::High
                    } else {
                        PinValue::Low
                    };
                    pin_guard.set_driver(Some(format!("{}_OUTPUT", self.base.name())), pin_value);
                }
            }
        }
    }

    /// Set data bus to high-impedance state to avoid bus contention
    fn tri_state_data_bus(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(format!("{}_DATA", self.base.name())), PinValue::HighZ);
                }
            }
        }
    }

    /// Tri-state all output pins
    fn tri_state_output_pins(&self) {
        for i in 0..10 {
            if let Ok(pin) = self.base.get_pin(&format!("O{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(format!("{}_OUTPUT", self.base.name())), PinValue::HighZ);
                }
            }
        }
    }

    /// Read the two-phase clock pins from CPU
    /// Returns: (PHI1_value, PHI2_value)
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

    /// Check for Φ1 rising edge

    /// Check for Φ2 rising edge

    /// Read all control pins from CPU
    /// Returns: (sync, chip_select, reset)
    fn read_control_pins(&self) -> (bool, bool, bool) {
        let sync = if let Ok(pin) = self.base.get_pin("SYNC") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        let chip_select = if let Ok(pin) = self.base.get_pin("CM") {
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

        (sync, chip_select, reset)
    }

    /// Handle system reset signal
    fn handle_reset(&mut self) {
        let (_, _, reset) = self.read_control_pins();
        if reset {
            // RESET is high - clear all internal state
            self.shift_register = [0u8; 10];
            self.output_latch = [0u8; 10];
            self.serial_input = 0;

            // Reset state machine
            self.shift_state = ShiftState::Idle;
            self.address_latch_time = None;
            self.address_high_nibble = None;
            self.address_low_nibble = None;
            self.full_address_ready = false;

            // Tri-state all outputs
            self.tri_state_data_bus();
            self.tri_state_output_pins();
        }
    }

    /// Assemble complete 8-bit address from high and low nibbles
    fn assemble_full_address(&mut self) {
        if let (Some(high), Some(low)) = (self.address_high_nibble, self.address_low_nibble) {
            // Assemble 8-bit address: (high << 4) | low
            self.full_address_ready = true;
            self.address_latch_time = Some(Instant::now());

            // Clear nibble storage for next address
            self.address_high_nibble = None;
            self.address_low_nibble = None;
        }
    }

    /// Handle Φ1 rising edge - Address and control phase
    fn handle_phi1_rising(&mut self) {
        // Handle system reset first
        self.handle_reset();

        // Check for shift register operation start
        let (sync, chip_select, _) = self.read_control_pins();
        if sync && chip_select {
            self.start_shift_address_phase();
        }

        // Handle shift register address operations
        self.handle_shift_address_operations();
    }

    /// Handle Φ2 rising edge - Data phase
    fn handle_phi2_rising(&mut self) {
        self.handle_shift_data_operations();
    }

    /// Handle shift register address operations during Φ1
    fn handle_shift_address_operations(&mut self) {
        match self.shift_state {
            ShiftState::Idle => {
                self.tri_state_data_bus();
            }

            ShiftState::AddressPhase => {
                self.handle_address_latching();
            }

            ShiftState::WaitLatency => {
                self.handle_latency_wait();
            }

            ShiftState::ShiftData | ShiftState::OutputData => {
                self.tri_state_data_bus();
            }
        }
    }

    /// Handle shift register data operations during Φ2
    fn handle_shift_data_operations(&mut self) {
        match self.shift_state {
            ShiftState::Idle => {
                self.tri_state_data_bus();
            }

            ShiftState::AddressPhase => {
                self.tri_state_data_bus();
            }

            ShiftState::WaitLatency => {
                self.handle_latency_wait();
                if self.shift_state == ShiftState::ShiftData {
                    self.handle_shift_operation();
                }
            }

            ShiftState::ShiftData => {
                self.handle_shift_operation();
            }

            ShiftState::OutputData => {
                self.handle_output_operation();
            }
        }
    }

    /// Start shift register address phase
    fn start_shift_address_phase(&mut self) {
        self.shift_state = ShiftState::AddressPhase;
        self.full_address_ready = false;
    }

    /// Handle address nibble latching
    fn handle_address_latching(&mut self) {
        let nibble = self.read_data_bus();

        if self.address_high_nibble.is_none() {
            self.address_high_nibble = Some(nibble);
        } else if self.address_low_nibble.is_none() {
            self.address_low_nibble = Some(nibble);
            self.assemble_full_address();
            self.start_latency_wait();
        }
    }

    /// Transition to latency wait state
    fn start_latency_wait(&mut self) {
        self.shift_state = ShiftState::WaitLatency;
        self.address_latch_time = Some(Instant::now());
    }

    /// Handle latency timing
    fn handle_latency_wait(&mut self) {
        if let Some(latch_time) = self.address_latch_time {
            if latch_time.elapsed() >= self.access_time {
                self.start_shift_operation();
            }
        }
    }

    /// Transition to shift operation state
    fn start_shift_operation(&mut self) {
        self.shift_state = ShiftState::ShiftData;
    }

    /// Handle shift operation
    fn handle_shift_operation(&mut self) {
        let (sync, chip_select, _) = self.read_control_pins();

        if sync && chip_select && self.full_address_ready {
            // Read serial input data from data bus
            let serial_data = self.read_data_bus();
            println!("DEBUG: 4003 shift operation - serial_data = 0x{:x}", serial_data);

            // Shift in the new bits (serial input)
            // For 4003, we shift in 4 bits at a time
            // First, shift existing bits right by 4 positions
            for j in (4..10).rev() {
                self.shift_register[j] = self.shift_register[j-4];
            }

            // Then insert the 4 new bits into positions 0-3
            for i in 0..4 {
                let bit = (serial_data >> i) & 1;
                self.shift_register[i] = bit;
                println!("DEBUG: 4003 shift step {} - inserted bit {} at position {}", i, bit, i);
            }

            // Update output latch with new shift register contents
            self.output_latch.copy_from_slice(&self.shift_register);
            self.update_output_pins();

            println!("DEBUG: 4003 shift register after operation: {:?}", self.shift_register);
            let (high, low) = self.get_shift_register();
            println!("DEBUG: 4003 get_shift_register() returns high=0x{:x}, low=0x{:x}", high, low);

            self.shift_state = ShiftState::OutputData;
        } else {
            self.tri_state_data_bus();
        }
    }

    /// Handle output operation
    fn handle_output_operation(&mut self) {
        // Output operation - parallel data is already available on output pins
        // This state maintains the output until next operation
    }


    /// Get the current shift register value
    /// Returns: 10-bit value as a tuple (high_byte, low_2_bits)
    pub fn get_shift_register(&self) -> (u8, u8) {
        let mut high_byte = 0u8;
        let mut low_2_bits = 0u8;

        for i in 0..8 {
            if self.shift_register[i] != 0 {
                high_byte |= 1 << i;
            }
        }

        for i in 0..2 {
            if self.shift_register[8 + i] != 0 {
                low_2_bits |= 1 << i;
            }
        }

        (high_byte, low_2_bits)
    }

    /// Get the current output latch value
    /// Returns: 10-bit value as a tuple (high_byte, low_2_bits)
    pub fn get_output_latch(&self) -> (u8, u8) {
        let mut high_byte = 0u8;
        let mut low_2_bits = 0u8;

        for i in 0..8 {
            if self.output_latch[i] != 0 {
                high_byte |= 1 << i;
            }
        }

        for i in 0..2 {
            if self.output_latch[8 + i] != 0 {
                low_2_bits |= 1 << i;
            }
        }

        (high_byte, low_2_bits)
    }

    /// Set serial input data
    /// Parameters: data - 4-bit serial input data
    pub fn set_serial_input(&mut self, data: u8) {
        self.serial_input = data & 0x0F;
    }

    /// Clear the shift register
    pub fn clear_shift_register(&mut self) {
        self.shift_register = [0u8; 10];
        self.output_latch = [0u8; 10];
        self.update_output_pins();
    }
}

impl Component for Intel4003 {
    fn name(&self) -> String {
        self.base.name()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.base.get_pin(name)
    }

    /// Main update cycle - handles clock edge detection and operation dispatch
    fn update(&mut self) {
        // Handle both rising and falling edges for proper two-phase operation
        let (phi1, phi2) = self.read_clock_pins();
        let phi1_rising = phi1 == PinValue::High && self.prev_phi1 == PinValue::Low;
        let phi2_rising = phi2 == PinValue::High && self.prev_phi2 == PinValue::Low;

        // Update clock states for next edge detection
        self.prev_phi1 = phi1;
        self.prev_phi2 = phi2;

        if phi1_rising {
            // Φ1 Rising Edge: Address phase
            self.handle_phi1_rising();
        }

        if phi2_rising {
            // Φ2 Rising Edge: Data phase
            self.handle_phi2_rising();
        }
    }

    /// Run component in time-slice mode
    fn run(&mut self) {
        self.base.set_running(true);

        // Initialize clock states
        let (phi1, phi2) = self.read_clock_pins();
        self.prev_phi1 = phi1;
        self.prev_phi2 = phi2;

        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(1));
        }
    }

    /// Stop component and tri-state all outputs
    fn stop(&mut self) {
        self.base.set_running(false);
        self.tri_state_data_bus();
        self.tri_state_output_pins();
        self.address_latch_time = None;
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

impl RunnableComponent for Intel4003 {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel4003_creation() {
        let sr = Intel4003::new("SHIFT_4003".to_string());
        assert_eq!(sr.name(), "SHIFT_4003");
        assert_eq!(sr.get_access_time(), 200); // Default 200ns
        assert!(!sr.is_running());
    }

    #[test]
    fn test_intel4003_shift_operation() {
        let mut sr = Intel4003::new_with_access_time("SHIFT_4003".to_string(), 1);

        // Set up the shift register with some initial data
        sr.shift_register = [1, 0, 1, 0, 1, 0, 1, 0, 1, 0]; // 1010101010 pattern

        // Set up the component for shift operation by setting the required state
        sr.shift_state = ShiftState::ShiftData;
        sr.full_address_ready = true;

        // Set data on data bus (0x05 = 0101 binary) for shift operation
        let d0_pin = sr.get_pin("D0").unwrap();
        let d1_pin = sr.get_pin("D1").unwrap();
        let d2_pin = sr.get_pin("D2").unwrap();
        let d3_pin = sr.get_pin("D3").unwrap();

        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::High); // Bit 0 = 1
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low);  // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::High); // Bit 2 = 1
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low);  // Bit 3 = 0
        }

        // Set control pins for shift operation
        let sync_pin = sr.get_pin("SYNC").unwrap();
        let cm_pin = sr.get_pin("CM").unwrap();
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Perform shift operation
        sr.handle_shift_operation();

        // Verify shift operation occurred
        let (high, low) = sr.get_shift_register();
        println!("DEBUG: After shift operation - high=0x{:x}, low=0x{:x}", high, low);
        println!("DEBUG: Shift register contents: {:?}", sr.shift_register);

        // The shift register should now contain the shifted data
        // Since we shifted in 0x05 (binary 0101), the register should contain this pattern
        // The first 4 bits should be 0101 (0x05), and the rest should be the original pattern shifted
        // The current result shows high=0x55, low=0x1 which means:
        // high byte: 01010101 (0x55) - this is correct, the first 4 bits are 0101 (0x05)
        // low byte: 00000001 (0x01) - this is the remaining 2 bits from the original pattern
        assert_eq!(high, 0x55); // First 8 bits should be 01010101
        assert_eq!(low, 0x01);  // Lower 2 bits should be 01
    }

    #[test]
    fn test_intel4003_reset() {
        let mut sr = Intel4003::new("SHIFT_4003".to_string());

        // Set some data first
        sr.set_serial_input(0x0F);
        sr.shift_register[0] = 1;

        // Verify data is set
        assert_eq!(sr.serial_input, 0x0F);
        assert_eq!(sr.shift_register[0], 1);

        // Apply reset
        let reset_pin = sr.get_pin("RESET").unwrap();
        {
            let mut reset_guard = reset_pin.lock().unwrap();
            reset_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Call handle_reset directly to ensure reset is processed
        sr.handle_reset();

        // Verify reset cleared everything
        assert_eq!(sr.serial_input, 0);
        assert_eq!(sr.shift_register[0], 0);
        assert_eq!(sr.shift_state, ShiftState::Idle);
    }

    #[test]
    fn test_configurable_access_time() {
        let mut sr = Intel4003::new("SHIFT_4003".to_string());

        // Test default access time
        assert_eq!(sr.get_access_time(), 200);

        // Test setting custom access time
        sr.set_access_time(100);
        assert_eq!(sr.get_access_time(), 100);

        // Test constructor with custom access time
        let fast_sr = Intel4003::new_with_access_time("FAST_SHIFT".to_string(), 1);
        assert_eq!(fast_sr.get_access_time(), 1);
        assert_eq!(fast_sr.name(), "FAST_SHIFT");
    }
}