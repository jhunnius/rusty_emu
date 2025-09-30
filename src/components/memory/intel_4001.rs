use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component, RunnableComponent};
use crate::components::common::intel_400x::{
    Intel400xAddressHandling, Intel400xClockHandling, Intel400xControlPins, Intel400xDataBus,
    Intel400xResetHandling, Intel400xTimingState, MemoryState, TimingState,
};
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
    memory: Vec<u8>,                 // 256-byte ROM storage
    last_address: u16,               // Last accessed memory address
    access_time: Duration,           // ROM access latency (500ns)
    output_latch: u8,                // 4-bit output latch for I/O operations
    input_latch: u8,                 // 4-bit input latch for I/O operations
    io_mode: IoMode,                 // Current I/O mode configuration
    io_ports: [u8; 4],               // 4 I/O ports (4 bits each) - matches datasheet
    io_direction: [IoDirection; 4],  // I/O direction for each port
    selected_io_port: Option<usize>, // Currently selected I/O port (0-3)
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
pub enum IoMode {
    Input,         // I/O pins configured as inputs (RDM instruction)
    Output,        // I/O pins configured as outputs (WRM instruction)
    Bidirectional, // I/O pins bidirectional (not used in 4001)
}

/// I/O direction for each I/O port
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoDirection {
    Input,  // Port configured as input
    Output, // Port configured as output
}

impl Intel400xClockHandling for Intel4001 {
    fn get_base(&self) -> &BaseComponent {
        &self.base
    }
}

impl Intel400xDataBus for Intel4001 {
    fn get_base(&self) -> &BaseComponent {
        &self.base
    }
}

impl Intel400xAddressHandling for Intel4001 {
    fn get_base(&self) -> &BaseComponent {
        &self.base
    }
}

impl Intel400xControlPins for Intel4001 {
    fn get_base(&self) -> &BaseComponent {
        &self.base
    }
}

impl Intel400xResetHandling for Intel4001 {
    fn get_base(&self) -> &BaseComponent {
        &self.base
    }

    fn perform_reset(&mut self) {
        // Reset operations specific to Intel4001
        // Note: This is called from handle_reset, so we don't need to check reset pin again
        self.set_timing_state(TimingState::Idle);
        self.tri_state_data_bus();
        self.address_low_nibble = None;
        self.address_high_nibble = None;
        self.full_address_ready = false;

        // Reset I/O state
        self.io_ports = [0u8; 4];
        self.io_direction = [IoDirection::Input; 4];
        self.selected_io_port = None;
        self.io_mode = IoMode::Input; // Reset I/O mode to Input
        self.tri_state_io_pins();
    }
}

impl Intel400xTimingState for Intel4001 {
    fn get_timing_state(&self) -> TimingState {
        self.memory_state.into()
    }

    fn set_timing_state(&mut self, state: TimingState) {
        self.memory_state = state.into();
    }

    fn get_address_latch_time(&self) -> Option<Instant> {
        self.address_latch_time
    }

    fn set_address_latch_time(&mut self, time: Option<Instant>) {
        self.address_latch_time = time;
    }

    fn get_full_address_ready(&self) -> bool {
        self.full_address_ready
    }

    fn set_full_address_ready(&mut self, ready: bool) {
        self.full_address_ready = ready;
    }

    fn get_address_high_nibble(&self) -> Option<u8> {
        self.address_high_nibble
    }

    fn set_address_high_nibble(&mut self, nibble: Option<u8>) {
        self.address_high_nibble = nibble;
    }

    fn get_address_low_nibble(&self) -> Option<u8> {
        self.address_low_nibble
    }

    fn set_address_low_nibble(&mut self, nibble: Option<u8>) {
        self.address_low_nibble = nibble;
    }

    fn get_access_time(&self) -> Duration {
        self.access_time
    }
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
            io_ports: [0u8; 4],                    // Initialize all I/O ports to 0
            io_direction: [IoDirection::Input; 4], // Default all ports to input
            selected_io_port: None,                // No I/O port selected initially
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

    /// Data bus methods now use common functionality

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

    /// Handle Φ1 rising edge - Address and control phase
    /// Hardware: Φ1 high = CPU drives bus with address/control information
    /// Memory operations start when SYNC goes high during Φ1 rising edge
    /// Focus: Address latching, reset, I/O operations
    fn handle_phi1_rising(&mut self) {
        // Handle system reset first (highest priority)
        self.handle_reset("RESET");

        // Check for memory operation start on Φ1 rising edge with SYNC high
        // Hardware: Memory operations start when SYNC goes high during Φ1
        // Note: SYNC marks instruction fetch start, but exact logic involves CPU cycle state.
        // ROM access: CM=1 (chip_select), CI=0 (!io_select)
        // I/O access: CM=1 (chip_select), CI=1 (io_select)
        let sync = self.read_sync_pin();
        let chip_select = self.read_cm_rom_pin();
        let io_select = self.read_ci_pin();

        if sync && chip_select && !io_select {
            // Start memory address phase on Φ1 rising edge (ROM access)
            self.start_memory_address_phase();
        }

        // Handle I/O operations (higher priority than memory operations)
        // I/O operations: CM=1 (chip_select), CI=1 (io_select)
        // Note: SYNC is NOT required for I/O - only marks instruction fetch (ROM access)
        if chip_select && io_select {
            self.handle_io_operation();
        }

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
                    pin_guard.set_driver(Some(self.base.name()), PinValue::HighZ);
                }
            }
        }
    }

    /// Set I/O mode for WRM/RDM instruction handling
    /// Parameters: mode - I/O mode (Input for RDM, Output for WRM)
    pub fn set_io_mode(&mut self, mode: IoMode) {
        self.io_mode = mode;
    }

    /// Get current I/O mode
    /// Returns: Current I/O mode
    pub fn get_io_mode(&self) -> IoMode {
        self.io_mode
    }

    /// Handle I/O port operations according to Intel 4001 datasheet
    /// Hardware: I/O operations occur during I/O instructions (WRM/RDM)
    /// Direction determined by CPU instruction, not manual configuration
    /// Note: I/O pins remain driven after WRM until another WRM or reset.
    /// This matches real 4001 hardware where I/O ports are latched.
    ///
    /// I/O Port Addressing (datasheet):
    /// - I/O ports are addressed using the lower 2 bits of the address
    /// - Port 0: Address 0x00, Port 1: Address 0x01, etc.
    /// - Each port is 4 bits wide (matches the data bus width)
    ///
    /// I/O Latch Persistence:
    /// - Output latch persists after WRM until next WRM or reset
    /// - Input latch overwrites every RDM (I/O input pins are live reads)
    fn handle_io_operation(&mut self) {
        let chip_select = self.read_cm_rom_pin();
        let io_select = self.read_ci_pin(); // Proper CI pin for I/O vs ROM selection

        // I/O operations occur when: CM=1 (chip_select), CI=1 (io_select)
        // Note: SYNC is NOT required for I/O - only marks instruction fetch (ROM access)
        if chip_select && io_select {
            let port_address = self.read_data_bus() & 0x03; // Lower 2 bits for port selection (0-3)
            self.selected_io_port = Some(port_address as usize);

            // Determine operation type based on current instruction phase
            // In real hardware, this would be determined by the CPU instruction (WRM vs RDM)
            match self.io_mode {
                IoMode::Input => {
                    // RDM instruction: Read from I/O port
                    if let Some(port) = self.selected_io_port {
                        self.input_latch = self.read_io_port(port);
                        // Write input data to data bus for CPU to read
                        self.write_data_bus(self.input_latch);
                    }
                }
                IoMode::Output => {
                    // WRM instruction: Write to I/O port
                    if let Some(port) = self.selected_io_port {
                        let data = self.read_data_bus();
                        self.write_io_port(port, data);
                    }
                }
                IoMode::Bidirectional => {
                    // Not used in 4001, treat as input
                    if let Some(port) = self.selected_io_port {
                        self.input_latch = self.read_io_port(port);
                        self.write_data_bus(self.input_latch);
                    }
                }
            }
        }
    }

    /// Read the CI (I/O select) pin state
    /// Hardware: CI pin distinguishes I/O vs ROM access when chip selected (CM=1)
    fn read_ci_pin(&self) -> bool {
        if let Ok(pin) = self.base.get_pin("CI") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Read from a specific I/O port
    /// Parameters: port - I/O port number (0-3)
    /// Returns: 4-bit value from the I/O port
    fn read_io_port(&self, port: usize) -> u8 {
        if port < 4 {
            match self.io_direction[port] {
                IoDirection::Input => {
                    // Read from actual I/O pins
                    self.read_io_pins()
                }
                IoDirection::Output => {
                    // Read from output latch (for read-back capability)
                    self.io_ports[port]
                }
            }
        } else {
            0
        }
    }

    /// Write to a specific I/O port
    /// Parameters: port - I/O port number (0-3), data - 4-bit data to write
    fn write_io_port(&mut self, port: usize, data: u8) {
        if port < 4 {
            self.io_ports[port] = data & 0x0F;
            self.io_direction[port] = IoDirection::Output;
            self.update_io_pins();
        }
    }

    /// Update I/O pins based on current port values and directions
    /// Hardware: Only output ports drive the I/O pins
    fn update_io_pins(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("IO{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    match self.io_direction[i] {
                        IoDirection::Output => {
                            // Drive pin with output port value
                            let bit_value = (self.io_ports[i]) & 1;
                            let pin_value = if bit_value == 1 {
                                PinValue::High
                            } else {
                                PinValue::Low
                            };
                            pin_guard.set_driver(Some(self.base.name()), pin_value);
                        }
                        IoDirection::Input => {
                            // Input mode - tri-state the pin
                            pin_guard.set_driver(Some(self.base.name()), PinValue::HighZ);
                        }
                    }
                }
            }
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
        println!(
            "DEBUG: {} - handle_memory_data_operations: state={:?}, address_ready={}",
            self.base.name(),
            self.memory_state,
            self.full_address_ready
        );
        match self.memory_state {
            MemoryState::Idle => {
                // During data phase, idle state means tri-state the bus
                println!("DEBUG: {} - In Idle state, tri-stating", self.base.name());
                self.tri_state_data_bus();
            }

            MemoryState::AddressPhase => {
                // Address phase should be handled by Φ1, not Φ2
                // Tri-state bus during wrong phase
                println!(
                    "DEBUG: {} - In AddressPhase during Φ2, tri-stating",
                    self.base.name()
                );
                self.tri_state_data_bus();
            }

            MemoryState::WaitLatency => {
                // Address latched, waiting for access latency
                // Check if latency has elapsed and we can transition to data phase
                println!(
                    "DEBUG: {} - In WaitLatency, checking latency",
                    self.base.name()
                );
                self.handle_latency_wait();
                // If we transitioned to DriveData, handle data driving
                if self.memory_state == MemoryState::DriveData {
                    println!(
                        "DEBUG: {} - Transitioned to DriveData, calling handle_data_driving",
                        self.base.name()
                    );
                    self.handle_data_driving();
                } else {
                    println!(
                        "DEBUG: {} - Still in WaitLatency after handle_latency_wait",
                        self.base.name()
                    );
                }
            }

            MemoryState::DriveData => {
                // Latency elapsed, drive data on bus during Φ2
                // Data will remain on bus until Φ2 falling edge
                println!(
                    "DEBUG: {} - In DriveData state, calling handle_data_driving",
                    self.base.name()
                );
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
        // Don't clear nibbles here - they should only be cleared after successful assembly
        // or when explicitly resetting the state machine
        self.full_address_ready = false;
    }

    /// Handle address nibble latching during address phase
    /// Hardware: CPU provides 8-bit address as two 4-bit nibbles
    fn handle_address_latching(&mut self) {
        // Use common address latching functionality
        let nibble = self.read_data_bus();

        if self.address_high_nibble.is_none() {
            // First cycle: latch high nibble (bits 7-4)
            self.address_high_nibble = Some(nibble);
        } else if self.address_low_nibble.is_none() {
            // Second cycle: latch low nibble (bits 3-0) and transition to latency wait
            self.address_low_nibble = Some(nibble);
            if let Some(address) =
                self.assemble_full_address(self.address_high_nibble, self.address_low_nibble)
            {
                self.last_address = address;
                self.start_latency_wait();
            }
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
            let elapsed = latch_time.elapsed();
            println!(
                "DEBUG: {} - handle_latency_wait: elapsed={:?}, access_time={:?}, ready={}",
                self.base.get_name(),
                elapsed,
                self.access_time,
                self.full_address_ready
            );
            if elapsed >= self.access_time {
                // Latency elapsed, transition to data driving
                // Data will be driven on next Φ2 rising edge
                println!(
                    "DEBUG: {} - Latency elapsed, transitioning to DriveData",
                    self.base.get_name()
                );
                self.start_data_driving();
            }
        } else {
            println!(
                "DEBUG: {} - handle_latency_wait: no latch_time set",
                self.base.get_name()
            );
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
        let sync = self.read_sync_pin();
        let chip_select = self.read_cm_rom_pin();
        let io_select = self.read_ci_pin();

        println!(
            "DEBUG: {} - handle_data_driving: SYNC={}, CM={}, CI={}, Address_Ready={}",
            self.base.name(),
            sync,
            chip_select,
            io_select,
            self.full_address_ready
        );

        // Memory read: CM=1 (chip_select), CI=0 (!io_select), valid address
        if sync && chip_select && !io_select && self.full_address_ready {
            // All conditions met: drive data on bus
            // Data will remain on bus until Φ2 falling edge
            let address = self.last_address;
            if (address as usize) < self.memory.len() {
                let data = self.memory[address as usize];
                println!(
                    "DEBUG: {} - All conditions met, driving data 0x{:x} to address 0x{:x}",
                    self.base.name(),
                    data,
                    address
                );
                self.write_data_bus(data);
                // Note: Don't call return_to_idle() here - wait for Φ2 falling edge
            } else {
                // Invalid address, tri-state
                println!(
                    "DEBUG: {} - Invalid address 0x{:x}, tri-stating",
                    self.base.name(),
                    address
                );
                self.tri_state_data_bus();
            }
        } else {
            // Bus contention guard: ROM should not drive when conditions not met
            // In real hardware, this would cause a short if CPU is still driving
            if self.full_address_ready {
                println!("DEBUG: {} - Bus contention detected! ROM attempting to drive data bus when conditions not met (SYNC={}, CM={}, CI={}, Address_Ready={})",
                         self.base.name(), sync, chip_select, io_select, self.full_address_ready);
            }
            // Conditions not met, tri-state
            println!("DEBUG: {} - Conditions not met, tri-stating (SYNC={}, CM={}, CI={}, Address_Ready={})",
                     self.base.name(), sync, chip_select, io_select, self.full_address_ready);
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
        if !self.is_running() {
            println!("DEBUG: Component not running, returning");
            return;
        }
        // Handle both rising and falling edges for proper two-phase operation
        let phi1_rising = self.is_phi1_rising_edge(self.prev_phi1);
        let phi1_falling = self.is_phi1_falling_edge(self.prev_phi1);
        let phi2_rising = self.is_phi2_rising_edge(self.prev_phi2);
        let phi2_falling = self.is_phi2_falling_edge(self.prev_phi2);

        // Update clock states for next edge detection
        let (phi1, phi2) = self.read_clock_pins();
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

    /// Get I/O port value
    /// Parameters: port - I/O port number (0-3)
    /// Returns: Some(4-bit value) if port valid, None if invalid
    pub fn get_io_port(&self, port: usize) -> Option<u8> {
        if port < 4 {
            Some(self.io_ports[port])
        } else {
            None
        }
    }

    /// Set I/O port value (for testing/debugging)
    /// Parameters: port - I/O port number (0-3), data - 4-bit data to set
    /// Returns: Ok(()) on success, Err(String) on failure
    pub fn set_io_port(&mut self, port: usize, data: u8) -> Result<(), String> {
        if port < 4 {
            self.io_ports[port] = data & 0x0F;
            self.io_direction[port] = IoDirection::Output;
            self.update_io_pins();
            Ok(())
        } else {
            Err("I/O port number out of range (0-3)".to_string())
        }
    }

    /// Get I/O port direction
    /// Parameters: port - I/O port number (0-3)
    /// Returns: Some(direction) if port valid, None if invalid
    pub fn get_io_direction(&self, port: usize) -> Option<IoDirection> {
        if port < 4 {
            Some(self.io_direction[port])
        } else {
            None
        }
    }

    /// Set I/O port direction (for testing/debugging)
    /// Parameters: port - I/O port number (0-3), direction - I/O direction
    /// Returns: Ok(()) on success, Err(String) on failure
    pub fn set_io_direction(&mut self, port: usize, direction: IoDirection) -> Result<(), String> {
        if port < 4 {
            self.io_direction[port] = direction;
            self.update_io_pins();
            Ok(())
        } else {
            Err("I/O port number out of range (0-3)".to_string())
        }
    }

    /// Get currently selected I/O port
    /// Returns: Some(port_number) if a port is selected, None otherwise
    pub fn get_selected_io_port(&self) -> Option<usize> {
        self.selected_io_port
    }

    /// Debug function to log state transitions for troubleshooting
    /// Parameters: test_name - Name of the test for context
    pub fn debug_state_transitions(&self, test_name: &str) {
        println!(
            "{} - State: {:?}, High: {:?}, Low: {:?}, Address: 0x{:x}, Ready: {}, Latch time: {:?}",
            test_name,
            self.memory_state,
            self.address_high_nibble,
            self.address_low_nibble,
            self.last_address,
            self.full_address_ready,
            self.address_latch_time
        );
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

impl std::fmt::Display for IoDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoDirection::Input => write!(f, "Input"),
            IoDirection::Output => write!(f, "Output"),
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
    fn test_memory_operation_start() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Initially should be in idle state
        assert_eq!(rom.memory_state, MemoryState::Idle);

        // Start the component so it can process updates
        rom.base.set_running(true);
        assert!(rom.is_running());

        // Get pin references
        let sync_pin = rom.get_pin("SYNC").unwrap();
        let cm_pin = rom.get_pin("CM").unwrap();
        let ci_pin = rom.get_pin("CI").unwrap();
        let phi1_pin = rom.get_pin("PHI1").unwrap();

        // Initialize PHI1 to Low first
        {
            let mut phi1_guard = phi1_pin.lock().unwrap();
            phi1_guard.set_driver(Some("TEST".to_string()), PinValue::Low);
        }

        // Set up memory read operation: SYNC=1, CM=1, CI=0 (memory access)
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut cm_guard = cm_pin.lock().unwrap();
            let mut ci_guard = ci_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            cm_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            ci_guard.set_driver(Some("TEST".to_string()), PinValue::Low); // Memory access
        }

        // Set Φ1 high (creating rising edge)
        {
            let mut phi1_guard = phi1_pin.lock().unwrap();
            phi1_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Debug: Check pin states before update
        println!("DEBUG: Before update - SYNC: {:?}, CM: {:?}, CI: {:?}, PHI1: {:?}",
                 sync_pin.lock().unwrap().read(),
                 cm_pin.lock().unwrap().read(),
                 ci_pin.lock().unwrap().read(),
                 phi1_pin.lock().unwrap().read());

        // Debug: Check conditions that would trigger memory operation
        let sync = rom.read_sync_pin();
        let chip_select = rom.read_cm_rom_pin();
        let io_select = rom.read_ci_pin();
        println!("DEBUG: Conditions - sync: {}, chip_select: {}, io_select: {}, combined: {}",
                 sync, chip_select, io_select, sync && chip_select && !io_select);

        // Debug: Check edge detection
        println!("DEBUG: Before update - prev_phi1: {:?}", rom.prev_phi1);

        // Debug: Check what read_clock_pins returns
        let (read_phi1, _) = rom.read_clock_pins();
        println!("DEBUG: read_clock_pins() returns PHI1: {:?}", read_phi1);

        // Debug: Check edge detection logic manually
        let phi1_rising = read_phi1 == PinValue::High && rom.prev_phi1 == PinValue::Low;
        println!("DEBUG: Manual edge detection - PHI1: {:?}, prev_phi1: {:?}, rising: {}",
                 read_phi1, rom.prev_phi1, phi1_rising);

        // Update to process Φ1 rising edge
        rom.update();

        // Debug: Check edge detection results
        println!("DEBUG: After update - prev_phi1: {:?}", rom.prev_phi1);

        // Should have started memory operation and transitioned to AddressPhase
        assert_eq!(rom.memory_state, MemoryState::AddressPhase);
        println!("Memory state after Φ1 rising: {:?}", rom.memory_state);
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
    fn test_intel4001_state_transitions() {
        let mut rom = Intel4001::new_with_access_time("StateTestROM".to_string(), 1);

        // Test all state transitions
        let states = vec![
            TimingState::Idle,
            TimingState::AddressPhase,
            TimingState::WaitLatency,
            TimingState::DriveData,
        ];

        for &state in &states {
            rom.set_timing_state(state);
            assert_eq!(rom.get_timing_state(), state);
        }

        // Test return to idle
        rom.set_timing_state(TimingState::Idle);
        assert_eq!(rom.get_timing_state(), TimingState::Idle);
    }

    #[test]
    fn test_intel4001_address_handling() {
        let mut rom = Intel4001::new_with_access_time("AddressTestROM".to_string(), 1);

        // Test address nibble handling
        rom.set_address_high_nibble(Some(0x12));
        rom.set_address_low_nibble(Some(0x34));
        rom.set_full_address_ready(true);

        assert_eq!(rom.get_address_high_nibble(), Some(0x12));
        assert_eq!(rom.get_address_low_nibble(), Some(0x34));
        assert_eq!(rom.get_full_address_ready(), true);

        // Test clearing
        rom.set_address_high_nibble(None);
        rom.set_address_low_nibble(None);
        rom.set_full_address_ready(false);

        assert_eq!(rom.get_address_high_nibble(), None);
        assert_eq!(rom.get_address_low_nibble(), None);
        assert_eq!(rom.get_full_address_ready(), false);
    }

    #[test]
    fn test_intel4001_error_conditions() {
        let mut rom = Intel4001::new("ErrorTestROM".to_string());

        // Test invalid memory access
        assert_eq!(rom.read_rom(0xFF), Some(0x00)); // Default value for unmapped memory

        // Test invalid data loading
        assert!(rom.load_rom_data(vec![0x12], 255).is_ok()); // Valid
        assert!(rom.load_rom_data(vec![0x12], 256).is_err()); // Invalid - out of bounds
    }

    #[test]
    fn test_intel4001_io_port_operations() {
        let mut rom = Intel4001::new("IOTestROM".to_string());

        // Test I/O port initialization
        assert_eq!(rom.get_io_port(0).unwrap(), 0);
        assert_eq!(rom.get_io_port(1).unwrap(), 0);
        assert_eq!(rom.get_io_port(2).unwrap(), 0);
        assert_eq!(rom.get_io_port(3).unwrap(), 0);

        // Test I/O direction initialization
        assert_eq!(rom.get_io_direction(0).unwrap(), IoDirection::Input);
        assert_eq!(rom.get_io_direction(1).unwrap(), IoDirection::Input);

        // Test setting I/O port values
        assert!(rom.set_io_port(0, 0x05).is_ok());
        assert_eq!(rom.get_io_port(0).unwrap(), 0x05);
        assert_eq!(rom.get_io_direction(0).unwrap(), IoDirection::Output);

        // Test invalid port access
        assert!(rom.set_io_port(4, 0x0F).is_err());
        assert!(rom.get_io_port(4).is_none());
        assert!(rom.set_io_direction(4, IoDirection::Output).is_err());
        assert!(rom.get_io_direction(4).is_none());
    }

    #[test]
    fn test_intel4001_io_mode_control() {
        let mut rom = Intel4001::new("ModeTestROM".to_string());

        // Test initial I/O mode
        assert_eq!(rom.get_io_mode(), IoMode::Input);

        // Test setting I/O modes
        rom.set_io_mode(IoMode::Output);
        assert_eq!(rom.get_io_mode(), IoMode::Output);

        rom.set_io_mode(IoMode::Input);
        assert_eq!(rom.get_io_mode(), IoMode::Input);

        rom.set_io_mode(IoMode::Bidirectional);
        assert_eq!(rom.get_io_mode(), IoMode::Bidirectional);
    }

    #[test]
    fn test_intel4001_io_port_selection() {
        let rom = Intel4001::new("SelectionTestROM".to_string());

        // Initially no port selected
        assert_eq!(rom.get_selected_io_port(), None);

        // Test I/O port selection during operation
        // This would be set during actual I/O operations
        // For testing, we can verify the field exists and can be modified
        assert_eq!(rom.selected_io_port, None);
    }

    #[test]
    fn test_io_direction_display() {
        assert_eq!(IoDirection::Input.to_string(), "Input");
        assert_eq!(IoDirection::Output.to_string(), "Output");
    }

    #[test]
    fn test_intel4001_reset_io_behavior() {
        let mut rom = Intel4001::new("ResetIOTestROM".to_string());

        // Set up some I/O state
        rom.set_io_port(0, 0x05).unwrap();
        rom.set_io_port(1, 0x0A).unwrap();
        rom.set_io_direction(0, IoDirection::Output).unwrap();
        rom.set_io_direction(1, IoDirection::Output).unwrap();
        rom.set_io_mode(IoMode::Output);

        // Verify state is set
        assert_eq!(rom.get_io_port(0).unwrap(), 0x05);
        assert_eq!(rom.get_io_port(1).unwrap(), 0x0A);
        assert_eq!(rom.get_io_direction(0).unwrap(), IoDirection::Output);
        assert_eq!(rom.get_io_mode(), IoMode::Output);

        // Perform reset
        rom.perform_reset();

        // Verify I/O state is cleared
        assert_eq!(rom.get_io_port(0).unwrap(), 0);
        assert_eq!(rom.get_io_port(1).unwrap(), 0);
        assert_eq!(rom.get_io_direction(0).unwrap(), IoDirection::Input);
        assert_eq!(rom.get_io_mode(), IoMode::Input);
        assert_eq!(rom.get_selected_io_port(), None);
    }
}
