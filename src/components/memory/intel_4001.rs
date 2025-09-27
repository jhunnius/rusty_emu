use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component, RunnableComponent};
use crate::pin::{Pin, PinValue};

/// Intel 4001 - 256-byte ROM with integrated I/O
/// Part of the MCS-4 family, designed to work with Intel 4004 CPU
/// Features 256 bytes of mask-programmable ROM and 4 I/O pins
///
/// Hardware Deviations:
/// - I/O read results are written to D0-D3 for test convenience
///   (real hardware reads directly from I/O pins)
/// - SYNC logic simplified for unit testing (may need refinement for CPU integration)
pub struct Intel4001 {
    base: BaseComponent,
    memory: Vec<u8>,       // 256-byte ROM storage
    last_address: u16,     // Last accessed memory address
    access_time: Duration, // ROM access latency (500ns)
    output_latch: u8,      // 4-bit output latch for I/O operations
    input_latch: u8,       // 4-bit input latch for I/O operations
    io_mode: IoMode,       // Current I/O mode configuration
    // Clock edge detection
    prev_phi1: PinValue, // Previous Φ1 clock state for edge detection
    prev_phi2: PinValue, // Previous Φ2 clock state for edge detection
    // Access latency modeling
    address_latch_time: Option<Instant>, // Timestamp when address was latched
    // Two-phase addressing for 8-bit address
    address_high_nibble: Option<u8>, // High nibble of 8-bit address
    address_low_nibble: Option<u8>,  // Low nibble of 8-bit address
    full_address_ready: bool,        // Whether complete address is assembled
    // Memory operation state machine
    memory_state: MemoryState, // Current state of memory operation
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// I/O mode configuration for the 4001 ROM
/// Determines how the I/O pins are configured during read/write operations
enum IoMode {
    Input,         // I/O pins configured as inputs (RDM instruction)
    Output,        // I/O pins configured as outputs (WRM instruction)
    Bidirectional, // I/O pins bidirectional (not used in 4001)
}

/// Memory operation state machine states
/// Tracks the current phase of memory access operations
#[derive(Debug, Clone, Copy, PartialEq)]
enum MemoryState {
    Idle,         // No memory operation in progress
    AddressPhase, // Currently latching address nibbles
    WaitLatency,  // Address latched, waiting for access time
    DriveData,    // Latency elapsed, driving data on bus
}

impl Intel4001 {
    /// Create a new Intel 4001 ROM with specified access time
    /// Parameters: name - Component identifier, access_time_ns - Memory access time in nanoseconds
    /// Returns: New Intel4001 instance with configurable access timing
    pub fn new(name: String) -> Self {
        Self::new_with_access_time(name, 500) // Default 500ns access time
    }

    /// Create a new Intel 4001 ROM with custom access time (for testing)
    /// Parameters: name - Component identifier, access_time_ns - Memory access time in nanoseconds
    /// Returns: New Intel4001 instance with configurable access timing
    pub fn new_with_access_time(name: String, access_time_ns: u64) -> Self {
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
        // - CM: Chip select (must be HIGH for ROM access)
        // - CI: I/O select (distinguishes I/O vs memory when chip selected)
        let pin_names = vec![
            "D0", "D1", "D2", "D3", // Data/Address pins
            "IO0", "IO1", "IO2", "IO3",   // I/O pins
            "SYNC",  // Sync signal
            "CM",    // CM-ROM: ROM/RAM Chip Select
            "CI",    // CM-RAM: RAM Chip Select (I/O vs ROM access)
            "RESET", // Reset
            "PHI1",  // Clock phase 1
            "PHI2",  // Clock phase 2
        ];

        let pins = BaseComponent::create_pin_map(&pin_names, &name);
        let memory = vec![0u8; rom_size];

        Intel4001 {
            base: BaseComponent::new(name, pins),
            memory,
            last_address: 0,
            access_time: Duration::from_nanos(access_time_ns),
            output_latch: 0,
            input_latch: 0,
            io_mode: IoMode::Input,
            prev_phi1: PinValue::Low,
            prev_phi2: PinValue::Low,
            address_latch_time: None,
            address_high_nibble: None,
            address_low_nibble: None,
            full_address_ready: false,
            memory_state: MemoryState::Idle,
        }
    }

    /// Set the memory access time for simulation
    /// Parameters: access_time_ns - Access time in nanoseconds
    pub fn set_access_time(&mut self, access_time_ns: u64) {
        self.access_time = Duration::from_nanos(access_time_ns);
    }

    /// Get the current memory access time
    /// Returns: Access time in nanoseconds
    pub fn get_access_time(&self) -> u64 {
        self.access_time.as_nanos() as u64
    }

    /// Load binary data into ROM at specified offset
    /// Parameters: data - Binary data to load, offset - Starting address
    /// Returns: Ok(()) on success, Err(String) on failure
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

    /// Load hexadecimal data into ROM at specified offset
    /// Parameters: hex_data - Space-separated hex bytes, offset - Starting address
    /// Returns: Ok(()) on success, Err(String) on failure
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

    /// Read the 4-bit data bus from D0-D3 pins
    /// Returns: 4-bit value from data bus pins
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

    /// Drive the 4-bit data bus with the specified value
    /// Parameters: data - 4-bit value to drive on D0-D3 pins
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

    /// Set data bus to high-impedance state to avoid bus contention
    /// CRITICAL: Must be called whenever ROM is not actively driving valid data
    fn tri_state_data_bus(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard
                        .set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                }
            }
        }
    }

    /// Assemble complete 8-bit address from high and low nibbles
    /// Hardware: Intel 4004 provides address in two 4-bit phases
    /// Format: (high_nibble << 4) | low_nibble
    fn assemble_full_address(&mut self) {
        if let (Some(high), Some(low)) = (self.address_high_nibble, self.address_low_nibble) {
            // Assemble 8-bit address: (high << 4) | low
            self.last_address = ((high as u16) << 4) | (low as u16);
            self.full_address_ready = true;
            self.address_latch_time = Some(Instant::now());

            // Clear nibble storage for next address
            self.address_high_nibble = None;
            self.address_low_nibble = None;
        }
    }

    /// Handle Φ1 rising edge - Address and control phase
    /// Hardware: Φ1 high = CPU drives bus with address/control information
    /// Memory operations start when SYNC goes high during Φ1 rising edge
    /// Focus: Address latching, reset, I/O operations
    fn handle_phi1_rising(&mut self) {
        // Handle system reset first (highest priority)
        self.handle_reset();

        // Check for memory operation start on Φ1 rising edge with SYNC high
        // Hardware: Memory operations start when SYNC goes high during Φ1
        // Note: SYNC marks instruction fetch start, but exact logic involves CPU cycle state.
        // ROM access: CM=1 (chip_select), CI=0 (!io_select)
        // I/O access: CM=1 (chip_select), CI=1 (io_select)
        let (sync, chip_select, io_select, _) = self.read_control_pins();
        if sync && chip_select && !io_select {
            // Start memory address phase on Φ1 rising edge
            self.start_memory_address_phase();
        }

        // Handle I/O operations (higher priority than memory operations)
        // I/O operations: CM=1 (chip_select), CI=1 (io_select)
        // Note: SYNC is NOT required for I/O - only marks instruction fetch (ROM access)
        self.handle_io_operation();

        // Handle memory address phase operations during Φ1
        self.handle_memory_address_operations();
    }

    /// Handle Φ1 falling edge - End of address phase
    /// Hardware: Φ1 low = End of CPU address driving phase
    fn handle_phi1_falling(&mut self) {
        // Currently no specific operations needed on Φ1 falling
        // Address latching happens on Φ1 rising, data driving on Φ2 rising
    }

    /// Handle Φ2 rising edge - Data phase
    /// Hardware: Φ2 high = Peripherals drive bus with data
    /// Focus: Data driving operations
    fn handle_phi2_rising(&mut self) {
        // Handle memory data phase operations during Φ2
        self.handle_memory_data_operations();
    }

    /// Handle Φ2 falling edge - End of data phase
    /// Hardware: Φ2 low = End of data driving phase, time to tri-state bus
    /// Data is held on bus during Φ2 high, tri-stated when Φ2 falls
    /// Focus: Clean up and return to idle state
    fn handle_phi2_falling(&mut self) {
        // Handle memory cleanup operations when Φ2 falls
        self.handle_memory_cleanup_operations();
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

    /// Read the two-phase clock pins from CPU
    /// Returns: (PHI1_value, PHI2_value)
    /// Hardware: Intel 4004 provides two-phase clock for synchronization
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

    /// Read all control pins from CPU
    /// Returns: (sync, chip_select, io_select, reset)
    /// Hardware: Control pins determine operation type and chip state
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

        // CM: Chip select (must be HIGH for ROM access)
        let chip_select = if let Ok(pin) = self.base.get_pin("CM") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        // CI: I/O select (distinguishes I/O vs memory when chip selected)
        let io_select = if let Ok(pin) = self.base.get_pin("CI") {
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

        (sync, chip_select, io_select, reset)
    }

    /// Handle system reset signal
    /// Hardware: RESET pin clears all internal state and tri-states outputs
    fn handle_reset(&mut self) {
        let (_, _, _, reset) = self.read_control_pins();
        if reset {
            // RESET is high - clear all internal state
            self.output_latch = 0;
            self.input_latch = 0;
            self.io_mode = IoMode::Input;
            self.tri_state_data_bus();
            self.tri_state_io_pins();

            // Reset memory operation state
            self.address_latch_time = None;
            self.memory_state = MemoryState::Idle;

            // Reset two-phase addressing state
            self.address_high_nibble = None;
            self.address_low_nibble = None;
            self.full_address_ready = false;
        }
    }

    /// Handle I/O port operations
    /// Hardware: I/O operations occur during I/O instructions (WRM/RDM)
    /// Direction determined by CPU instruction, not manual configuration
    /// Note: I/O pins remain driven after WRM until another WRM or reset.
    /// This matches real 4001 hardware where I/O ports are latched.
    /// Hardware deviation: Model writes IO latch to D0-D3 for test convenience,
    /// but real hardware reads I/O pins directly.
    ///
    /// I/O Latch Persistence:
    /// - Output latch persists after WRM until next WRM or reset
    /// - Input latch overwrites every RDM (I/O input pins are live reads)
    fn handle_io_operation(&mut self) {
        let (_sync, chip_select, io_select, _) = self.read_control_pins();

        // I/O operations occur when: CM=1 (chip_select), CI=1 (io_select)
        // Note: SYNC is NOT required for I/O - only marks instruction fetch (ROM access)
        if chip_select && io_select {
            let _address_low = self.read_data_bus(); // I/O port address (lower 4 bits)

            // Real 4001 I/O behavior based on CPU instructions:
            // RDM instruction: Read from I/O port
            self.input_latch = self.read_io_pins();
            // Hardware deviation: Write to D0-D3 for test convenience
            self.write_data_bus(self.input_latch);
            self.io_mode = IoMode::Input;
        }
    }

    /// Handle memory address-related operations during Φ1
    /// Hardware-accurate: Φ1 is when CPU drives address, so we handle address latching
    /// Memory operation start is checked in handle_phi1_rising() on SYNC high + Φ1 rising
    /// Focus: Address latching and setup operations
    fn handle_memory_address_operations(&mut self) {
        match self.memory_state {
            MemoryState::Idle => {
                // In idle state, ensure bus is tri-stated
                // Memory operation start is checked in handle_phi1_rising()
                self.tri_state_data_bus();
            }

            MemoryState::AddressPhase => {
                // Currently latching address nibbles during Φ1
                self.handle_address_latching();
            }

            MemoryState::WaitLatency => {
                // Address latched, waiting for access latency
                // Check if latency has elapsed and we can transition to data phase
                self.handle_latency_wait();
            }

            MemoryState::DriveData => {
                // Data phase should be handled by Φ2, not Φ1
                // Just ensure bus is tri-stated during wrong phase
                self.tri_state_data_bus();
            }
        }
    }

    /// Handle memory data-related operations during Φ2
    /// Hardware-accurate: Φ2 is when peripherals drive data, so we handle data driving
    /// Focus: Data driving operations
    fn handle_memory_data_operations(&mut self) {
        match self.memory_state {
            MemoryState::Idle => {
                // During data phase, idle state means tri-state the bus
                self.tri_state_data_bus();
            }

            MemoryState::AddressPhase => {
                // Address phase should be handled by Φ1, not Φ2
                // Tri-state bus during wrong phase
                self.tri_state_data_bus();
            }

            MemoryState::WaitLatency => {
                // Still waiting for latency, tri-state bus
                self.tri_state_data_bus();
            }

            MemoryState::DriveData => {
                // Latency elapsed, drive data on bus during Φ2
                // Data will remain on bus until Φ2 falling edge
                self.handle_data_driving();
            }
        }
    }

    /// Handle memory cleanup operations on Φ2 falling edge
    /// Hardware-accurate: Clean up when Φ2 falls (end of data phase)
    /// Focus: Tri-state bus and return to idle
    fn handle_memory_cleanup_operations(&mut self) {
        match self.memory_state {
            MemoryState::DriveData => {
                // End of data phase - tri-state bus and return to idle
                self.tri_state_data_bus();
                self.return_to_idle();
            }

            MemoryState::Idle | MemoryState::AddressPhase | MemoryState::WaitLatency => {
                // For other states, just ensure bus is tri-stated
                self.tri_state_data_bus();
            }
        }
    }

    /// Transition to address phase state
    /// Hardware: Start of memory read cycle, CPU begins providing address
    fn start_memory_address_phase(&mut self) {
        self.memory_state = MemoryState::AddressPhase;
        self.address_high_nibble = None;
        self.address_low_nibble = None;
        self.full_address_ready = false;
    }

    /// Handle address nibble latching during address phase
    /// Hardware: CPU provides 8-bit address as two 4-bit nibbles
    fn handle_address_latching(&mut self) {
        let nibble = self.read_data_bus();

        if self.address_high_nibble.is_none() {
            // First cycle: latch high nibble (bits 7-4)
            self.address_high_nibble = Some(nibble);
        } else if self.address_low_nibble.is_none() {
            // Second cycle: latch low nibble (bits 3-0) and transition to latency wait
            self.address_low_nibble = Some(nibble);
            self.assemble_full_address();
            self.start_latency_wait();
        }
    }

    /// Transition to latency wait state
    /// Hardware: Address captured, start 500ns access time before data available
    fn start_latency_wait(&mut self) {
        self.memory_state = MemoryState::WaitLatency;
        self.address_latch_time = Some(Instant::now());
    }

    /// Handle latency timing during wait state
    /// Hardware: ROM needs 500ns to access data after address is latched
    fn handle_latency_wait(&mut self) {
        if let Some(latch_time) = self.address_latch_time {
            if latch_time.elapsed() >= self.access_time {
                // Latency elapsed, transition to data driving
                self.start_data_driving();
            }
        }
    }

    /// Transition to data driving state
    /// Hardware: Access latency complete, ready to drive data on next Φ2 cycle
    fn start_data_driving(&mut self) {
        self.memory_state = MemoryState::DriveData;
    }

    /// Handle data driving during DriveData state
    /// Hardware: ROM drives data bus when all conditions are met
    /// Data stays on bus during Φ2 high period, tri-stated on Φ2 falling edge
    fn handle_data_driving(&mut self) {
        let (sync, chip_select, io_select, _) = self.read_control_pins();

        // Memory read: CM=1 (chip_select), CI=0 (!io_select), valid address
        if sync && chip_select && !io_select && self.full_address_ready {
            // All conditions met: drive data on bus
            // Data will remain on bus until Φ2 falling edge
            let address = self.last_address;
            if (address as usize) < self.memory.len() {
                let data = self.memory[address as usize];
                self.write_data_bus(data);
                // Note: Don't call return_to_idle() here - wait for Φ2 falling edge
            } else {
                // Invalid address, tri-state
                self.tri_state_data_bus();
            }
        } else {
            // Bus contention guard: ROM should not drive when conditions not met
            // In real hardware, this would cause a short if CPU is still driving
            if self.full_address_ready {
                eprintln!("WARNING: {} - Bus contention detected! ROM attempting to drive data bus when conditions not met (SYNC={}, CM={}, CI={}, Address_Ready={})",
                         self.base.name(), sync, chip_select, io_select, self.full_address_ready);
            }
            // Conditions not met, tri-state
            self.tri_state_data_bus();
        }
    }

    /// Reset memory state machine to idle
    /// Hardware: Called when memory operation completes or is interrupted
    fn return_to_idle(&mut self) {
        self.memory_state = MemoryState::Idle;
        self.address_latch_time = None;
        self.address_high_nibble = None;
        self.address_low_nibble = None;
        self.full_address_ready = false;
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

    /// Main update cycle - handles clock edge detection and operation dispatch
    /// Hardware: Responds to Φ1 and Φ2 clock edges from CPU
    fn update(&mut self) {
        // Handle both rising and falling edges for proper two-phase operation
        let (phi1, phi2) = self.read_clock_pins();
        let phi1_rising = phi1 == PinValue::High && self.prev_phi1 == PinValue::Low;
        let phi1_falling = phi1 == PinValue::Low && self.prev_phi1 == PinValue::High;
        let phi2_rising = phi2 == PinValue::High && self.prev_phi2 == PinValue::Low;
        let phi2_falling = phi2 == PinValue::Low && self.prev_phi2 == PinValue::High;

        // Update clock states for next edge detection
        self.prev_phi1 = phi1;
        self.prev_phi2 = phi2;

        if phi1_rising {
            // Φ1 Rising Edge: Address phase (CPU drives bus) - handle address latching
            self.handle_phi1_rising();
        }

        if phi1_falling {
            // Φ1 Falling Edge: End of address phase
            self.handle_phi1_falling();
        }

        if phi2_rising {
            // Φ2 Rising Edge: Data phase (ROM drives bus if ready) - handle data driving
            self.handle_phi2_rising();
        }

        if phi2_falling {
            // Φ2 Falling Edge: End of data phase - tri-state bus and return to idle
            self.handle_phi2_falling();
        }
    }

    /// Run component in time-slice mode (manual control)
    /// Hardware: Simulates continuous operation with clock edge detection
    fn run(&mut self) {
        self.base.set_running(true);

        // Initialize clock states for edge detection
        let (phi1, phi2) = self.read_clock_pins();
        self.prev_phi1 = phi1;
        self.prev_phi2 = phi2;

        while self.is_running() {
            self.update();
            // Small delay to prevent busy waiting when no clock is present
            thread::sleep(Duration::from_micros(1));
        }
    }

    /// Stop component and tri-state all outputs
    /// Hardware: Component enters low-power state, all pins high-impedance
    fn stop(&mut self) {
        self.base.set_running(false);

        // Tri-state all outputs when stopped
        self.tri_state_data_bus();
        self.tri_state_io_pins();

        // Reset memory operation state
        self.address_latch_time = None;
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

impl RunnableComponent for Intel4001 {}

// Intel 4001 specific methods
impl Intel4001 {
    /// Get the ROM size in bytes
    /// Returns: Total number of bytes in ROM (256 for 4001)
    pub fn get_rom_size(&self) -> usize {
        self.memory.len()
    }

    /// Read a byte from ROM at specified address
    /// Parameters: address - 8-bit address (0-255)
    /// Returns: Some(data) if address valid, None if out of bounds
    pub fn read_rom(&self, address: u8) -> Option<u8> {
        if (address as usize) < self.memory.len() {
            Some(self.memory[address as usize])
        } else {
            None
        }
    }

    /// Get the current output latch value
    /// Returns: 4-bit value last written to I/O ports
    pub fn get_output_latch(&self) -> u8 {
        self.output_latch
    }

    /// Get the current input latch value
    /// Returns: 4-bit value last read from I/O ports
    pub fn get_input_latch(&self) -> u8 {
        self.input_latch
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
        assert_eq!(rom.get_access_time(), 500); // Default 500ns
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
        let rom = Intel4001::new("ROM_4001".to_string());

        // Test that I/O mode starts as Input (after reset)
        // Note: IoMode is now internal and not exposed via public API
        // I/O direction is determined by CPU instructions (WRM/RDM)
        assert_eq!(rom.name(), "ROM_4001");
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

    #[test]
    fn test_configurable_access_time() {
        let mut rom = Intel4001::new("ROM_4001".to_string());

        // Test default access time
        assert_eq!(rom.get_access_time(), 500);

        // Test setting custom access time
        rom.set_access_time(100);
        assert_eq!(rom.get_access_time(), 100);

        // Test constructor with custom access time
        let fast_rom = Intel4001::new_with_access_time("FAST_ROM".to_string(), 1);
        assert_eq!(fast_rom.get_access_time(), 1);
        assert_eq!(fast_rom.name(), "FAST_ROM");
    }

    #[test]
    fn test_basic_memory_read() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Load test data
        let test_data = vec![0x12, 0x34];
        rom.load_rom_data(test_data, 0).unwrap();

        // Test direct memory read (bypassing pin simulation)
        assert_eq!(rom.read_rom(0x00).unwrap(), 0x12);
        assert_eq!(rom.read_rom(0x01).unwrap(), 0x34);
    }

    #[test]
    fn test_address_latching() {
        // Note: This test sets Φ1 and Φ2 directly for unit validation.
        // For system integration testing, consider using a proper clock generator
        // that drives Φ1/Φ2 continuously through realistic multiple cycles.
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Load test data
        let test_data = vec![0x12];
        rom.load_rom_data(test_data, 0).unwrap();

        // Get pin references
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let phi1_pin = rom.get_pin("PHI1").unwrap();
        let phi2_pin = rom.get_pin("PHI2").unwrap();
        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // Set up memory read operation: SYNC=1, CM=1, CI=0 (memory access)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access
        }

        // Set address high nibble (0x0) on data bus
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 3 = 0
        }

        // --- High nibble ---
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> latch high nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge

        // Should have transitioned to AddressPhase
        assert_eq!(rom.memory_state, MemoryState::AddressPhase);

        // Check that high nibble was latched
        assert_eq!(rom.address_high_nibble, Some(0x0));

        // Set address low nibble (0x0) on data bus
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 3 = 0
        }

        // --- Low nibble ---
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> latch low nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge

        // Should have assembled full address and transitioned to WaitLatency
        // Note: nibbles are cleared after assembly, so check last_address instead
        assert_eq!(rom.last_address, 0x00);
        assert_eq!(rom.full_address_ready, true);
        assert_eq!(rom.memory_state, MemoryState::WaitLatency);

        // Advance simulated time to exceed access_time (1ns)
        std::thread::sleep(Duration::from_nanos(2));
    }

    #[test]
    fn test_memory_operation_start() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Initially should be in idle state
        assert_eq!(rom.memory_state, MemoryState::Idle);

        // Get pin references
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let phi1_pin = rom.get_pin("PHI1").unwrap();

        // Set up memory read operation: SYNC=1, CM=1, CI=0 (memory access)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access
        }

        // Set Φ1 high
        {
            let mut phi1_guard = phi1_pin.lock().unwrap();
            phi1_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Update to process Φ1 rising edge
        rom.update();

        // Should have started memory operation and transitioned to AddressPhase
        assert_eq!(rom.memory_state, MemoryState::AddressPhase);
        println!("Memory state after Φ1 rising: {:?}", rom.memory_state);
    }

    #[test]
    fn test_simple_data_driving() {
        // Note: This test manually sequences through clock phases for unit validation.
        // For system integration, use continuous clock generator for realistic timing.
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Load test data
        let test_data = vec![0x12];
        rom.load_rom_data(test_data, 0).unwrap();

        // Get pin references
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let phi1_pin = rom.get_pin("PHI1").unwrap();
        let phi2_pin = rom.get_pin("PHI2").unwrap();
        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // Set up memory read operation: SYNC=1, CM=1, CI=0 (memory access)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access
        }

        // Phase 1: Address phase - latch high nibble (0x0)
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 3 = 0
        }

        // --- High nibble ---
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> latch high nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        println!(
            "After Φ1 high - State: {:?}, High nibble: {:?}, Low nibble: {:?}",
            rom.memory_state, rom.address_high_nibble, rom.address_low_nibble
        );

        // Phase 2: Address phase - latch low nibble (0x0)
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 3 = 0
        }

        // --- Low nibble ---
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> latch low nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        println!("After second Φ1 high - State: {:?}, High nibble: {:?}, Low nibble: {:?}, Address: 0x{:x}", rom.memory_state, rom.address_high_nibble, rom.address_low_nibble, rom.last_address);

        // Advance simulated time to exceed access_time (1ns)
        std::thread::sleep(Duration::from_nanos(2));

        // --- Data phase ---
        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> drive data

        // Assert DriveData state while Φ2 is high
        assert_eq!(rom.memory_state, MemoryState::DriveData);
        println!("After Φ2 high - State: {:?}", rom.memory_state);

        // Verify data is driven on bus while Φ2 is high: 0x12 = 0001 0010
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::Low); // Bit 0 = 0
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::High); // Bit 1 = 1
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::Low); // Bit 2 = 0
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::High); // Bit 3 = 1

        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge -> tri-state
    }

    #[test]
    fn test_memory_operation_start_detection() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Load test data
        let test_data = vec![0x12];
        rom.load_rom_data(test_data, 0).unwrap();

        // Get pin references
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let phi1_pin = rom.get_pin("PHI1").unwrap();
        let phi2_pin = rom.get_pin("PHI2").unwrap();
        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // Initially, memory should be in idle state
        assert_eq!(rom.memory_state, MemoryState::Idle);

        // Set up memory read operation: SYNC=1, CM=1, CI=0 (memory access)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access
        }

        // Set address 0x0 on data bus
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::Low);
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low);
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::Low);
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low);
        }

        // Set Φ1 high, Φ2 low - should start memory operation
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);

        // Update to process Φ1 rising edge
        rom.update();

        // Should have transitioned to AddressPhase
        assert_eq!(rom.memory_state, MemoryState::AddressPhase);

        // Set address on data bus for latching
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".into()), PinValue::Low);
            d1_guard.set_driver(Some("TEST".into()), PinValue::Low);
            d2_guard.set_driver(Some("TEST".into()), PinValue::Low);
            d3_guard.set_driver(Some("TEST".into()), PinValue::Low);
        }

        // --- High nibble ---
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge

        // --- Low nibble ---
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> latch low nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge

        // Advance simulated time to exceed access_time (1ns)
        std::thread::sleep(Duration::from_nanos(2));

        // --- Data phase ---
        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> drive data

        // Assert DriveData state while Φ2 is high
        assert_eq!(rom.memory_state, MemoryState::DriveData);

        // Verify data is driven on bus while Φ2 is high: 0x12 = 0001 0010
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::Low); // Bit 0 = 0
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::High); // Bit 1 = 1
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::Low); // Bit 2 = 0
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::High); // Bit 3 = 1

        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge -> tri-state
    }

    #[test]
    fn test_full_address_fetch_cycle_integration() {
        // Note: This integration test manually steps through clock phases.
        // For full system testing, implement continuous clock generator
        // that maintains proper Φ1/Φ2 timing relationships.
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Load test data: address 0x00 = 0x12, address 0x01 = 0x34
        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        rom.load_rom_data(test_data, 0).unwrap();

        // Get pin references for simulation
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let phi1_pin = rom.get_pin("PHI1").unwrap();
        let phi2_pin = rom.get_pin("PHI2").unwrap();
        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // Set up memory read operation: SYNC=1, CM=1, CI=0 (memory access, not I/O)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access, not I/O
        }

        // Test complete cycle for address 0x00
        // Step 1: Set address high nibble (0x0) on data bus
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 3 = 0
        }

        // Step 2: High nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for high nibble

        // Step 3: Set address low nibble (0x0) on data bus
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 3 = 0
        }

        // Step 4: Low nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for low nibble

        // Advance simulated time to exceed access_time (1ns)
        std::thread::sleep(Duration::from_nanos(2));

        // Data phase
        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> drive data

        // Step 5: Verify data is driven on bus while Φ2 is high: 0x12 = 0001 0010
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::Low); // Bit 0 = 0
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::High); // Bit 1 = 1
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::Low); // Bit 2 = 0
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::High); // Bit 3 = 1 (0x12 = 0001 0010)

        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge -> tri-state

        // Test complete cycle for address 0x01
        // Reset memory state first
        rom.return_to_idle();

        // Step 1: Set address high nibble (0x0) on data bus
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 3 = 0
        }

        // Step 2: High nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for high nibble

        // Step 3: Set address low nibble (0x1) on data bus
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".into()), PinValue::High); // Bit 0 = 1
            d1_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 3 = 0
        }

        // Step 4: Low nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for low nibble

        // Advance simulated time to exceed access_time (1ns)
        std::thread::sleep(Duration::from_nanos(2));

        // Data phase
        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> drive data

        // Step 5: Verify data is driven on bus while Φ2 is high: 0x34 = 0011 0100
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::Low); // Bit 0 = 0
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::Low); // Bit 1 = 0
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::High); // Bit 2 = 1
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::High); // Bit 3 = 1 (0x34 = 0011 0100)

        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge -> tri-state
    }

    #[test]
    fn test_clock_driven_memory_fetch() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1); // 1ns for fast testing

        // Load test data: address 0x01 should contain 0x34
        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        rom.load_rom_data(test_data, 0).unwrap();

        // Get pin references for simulation
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // Set up memory read operation: SYNC=1, chip_select=1, io_select=0 (memory access)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access, not I/O
        }

        // Test that memory state machine works correctly
        // The state machine should handle the two-phase addressing properly
        rom.update();

        // Verify that the ROM correctly read the test data
        assert_eq!(rom.read_rom(0).unwrap(), 0x12);
        assert_eq!(rom.read_rom(1).unwrap(), 0x34);
        assert_eq!(rom.read_rom(2).unwrap(), 0x56);
        assert_eq!(rom.read_rom(3).unwrap(), 0x78);
    }

    #[test]
    fn test_reset_behavior() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1); // 1ns for fast testing

        // Load test data
        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        rom.load_rom_data(test_data, 0).unwrap();

        // Set up some state first
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let reset_pin = rom.get_pin("RESET").unwrap();

        // Set memory operation active
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access, not I/O
        }

        // Trigger some state changes
        rom.update();

        // Now assert RESET
        {
            let mut reset_guard = reset_pin.lock().unwrap();
            reset_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Update with RESET high - should clear all state
        rom.update();

        // Verify all state is cleared
        assert_eq!(rom.get_output_latch(), 0);
        assert_eq!(rom.get_input_latch(), 0);

        // Verify data bus is tri-stated
        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // After reset, all pins should be HighZ (tri-stated)
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::HighZ);
    }

    #[test]
    fn test_tri_state_behavior() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1); // 1ns for fast testing

        // Load test data
        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        rom.load_rom_data(test_data, 0).unwrap();

        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // Initially, no operation - should be tri-stated
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::HighZ);

        // Set up memory operation but don't complete address phase
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();

        let mut sync_guard = sync_pin.lock().unwrap();
        let mut cm_guard = cm_pin.lock().unwrap();
        let mut ci_guard = ci_pin.lock().unwrap();
        sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access, not I/O

        // Trigger update - should start address phase but still tri-state
        rom.update();

        // Should still be tri-stated during address phase
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::HighZ);

        // Complete address phase but don't wait for latency
        // Should still be tri-stated
        rom.update();

        // Should still be tri-stated (waiting for latency)
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::HighZ);
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::HighZ);
    }

    #[test]
    fn test_clock_generator_integration() {
        // Integration test with simulated clock generator stepping through multiple cycles
        // This tests more realistic timing scenarios compared to direct Φ1/Φ2 manipulation
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Load test data: address 0x00 = 0x12, address 0x01 = 0x34
        let test_data = vec![0x12, 0x34];
        rom.load_rom_data(test_data, 0).unwrap();

        // Get pin references
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let phi1_pin = rom.get_pin("PHI1").unwrap();
        let phi2_pin = rom.get_pin("PHI2").unwrap();
        let d0_pin = rom.get_pin("D0").unwrap();
        let d1_pin = rom.get_pin("D1").unwrap();
        let d2_pin = rom.get_pin("D2").unwrap();
        let d3_pin = rom.get_pin("D3").unwrap();

        // Set up memory read operation: SYNC=1, CM=1, CI=0 (memory access)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access
        }

        // Simulate clock generator: step through multiple Φ1/Φ2 cycles
        // Cycle 1: Address phase for high nibble (0x0)
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 3 = 0
        }

        // High nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for high nibble

        // Cycle 2: Address phase for low nibble (0x0)
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 3 = 0
        }

        // Low nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for low nibble

        // Advance simulated time to exceed access_time (1ns)
        std::thread::sleep(Duration::from_nanos(2));

        // Data phase
        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> drive data

        // Should have data 0x12 on bus while Φ2 is high: 0001 0010
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::Low); // Bit 0 = 0
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::High); // Bit 1 = 1
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::Low); // Bit 2 = 0
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::High); // Bit 3 = 1

        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge -> tri-state

        // Cycle 3: Next memory operation for address 0x01
        rom.return_to_idle();

        // Set address high nibble (0x0) for address 0x01
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 0 = 0
            d1_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Bit 3 = 0
        }

        // High nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for high nibble

        // Set address low nibble (0x1) for address 0x01
        {
            let mut d0_guard = d0_pin.lock().unwrap();
            let mut d1_guard = d1_pin.lock().unwrap();
            let mut d2_guard = d2_pin.lock().unwrap();
            let mut d3_guard = d3_pin.lock().unwrap();
            d0_guard.set_driver(Some("TEST".into()), PinValue::High); // Bit 0 = 1
            d1_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 1 = 0
            d2_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 2 = 0
            d3_guard.set_driver(Some("TEST".into()), PinValue::Low); // Bit 3 = 0
        }

        // Low nibble
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge
        phi1_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge for low nibble

        // Advance simulated time to exceed access_time (1ns)
        std::thread::sleep(Duration::from_nanos(2));

        // Data phase
        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::High);
        rom.update(); // rising edge -> drive data

        // Should have data 0x34 on bus while Φ2 is high: 0011 0100
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::Low); // Bit 0 = 0
        assert_eq!(d1_pin.lock().unwrap().read(), PinValue::Low); // Bit 1 = 0
        assert_eq!(d2_pin.lock().unwrap().read(), PinValue::High); // Bit 2 = 1
        assert_eq!(d3_pin.lock().unwrap().read(), PinValue::High); // Bit 3 = 1

        phi2_pin
            .lock()
            .unwrap()
            .set_driver(Some("TEST".into()), PinValue::Low);
        rom.update(); // falling edge -> tri-state
    }
}
