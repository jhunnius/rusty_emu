use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component, RunnableComponent};
use crate::pin::{Pin, PinValue};

/// Intel 4002 - 320-bit RAM (80 nibbles × 4 bits) with integrated output ports
/// Part of the MCS-4 family, designed to work with Intel 4004 CPU
/// Features 80 nibbles of RAM organized in 4 banks of 20 nibbles each,
/// plus 4 status characters and 4 output ports
///
/// Hardware Architecture:
/// - 4 banks × 20 registers × 4 bits = 80 nibbles total
/// - Status characters are separate 4-bit latches (not part of RAM)
/// - 4 output ports, each 4 bits wide
/// - Complex addressing via SRC/WRM/RDM instructions from CPU
///
/// Hardware Deviations:
/// - Simplified timing model for unit testing (may need refinement for CPU integration)
/// - Output port behavior matches 4001 I/O latch persistence
/// - Bank selection and status character handling follows Intel MCS-4 architecture
pub struct Intel4002 {
    base: BaseComponent,
    memory: [u8; 80],              // 80 nibbles of RAM (320 bits total) - 4 banks × 20 nibbles
    last_address: u8,              // Last accessed memory address
    access_time: Duration,         // RAM access latency (500ns typical)
    address_latch_time: Option<Instant>, // Timestamp when address was latched
    output_ports: [u8; 4],         // 4 output ports (4 bits each) - TODO: Make [[u8; 4]; 4] for 4-bit ports
    input_latch: u8,               // Input data latch for I/O operations
    status_characters: [u8; 4],    // 4 separate status character latches (4 bits each)
    bank_select: u8,               // RAM bank selection (2 bits)
    // Clock edge detection (same as 4001)
    prev_phi1: PinValue,           // Previous Φ1 clock state for edge detection
    prev_phi2: PinValue,           // Previous Φ2 clock state for edge detection
    // Two-phase addressing for 8-bit address (same as 4001)
    address_high_nibble: Option<u8>, // High nibble of 8-bit address
    address_low_nibble: Option<u8>,  // Low nibble of 8-bit address
    full_address_ready: bool,        // Whether complete address is assembled
    // RAM operation state machine
    ram_state: RamState,           // Current state of RAM operation
    // Data latching for RAM operations
    data_latch: Option<u8>,        // Latched data for write operations
    // Instruction cycle tracking
    instruction_phase: bool,       // Whether we're in instruction phase
    current_instruction: u8,       // Current instruction being processed
}

/// RAM operation state machine states
/// Tracks the current phase of RAM access operations
#[derive(Debug, Clone, Copy, PartialEq)]
enum RamState {
    Idle,         // No RAM operation in progress
    AddressPhase, // Currently latching address nibbles
    WaitLatency,  // Address latched, waiting for access time
    ReadData,     // Reading data from RAM
    WriteData,    // Writing data to RAM
    OutputPort,   // Output port operation
}

/// Intel 4002 timing constants (based on datasheet specifications)
/// These represent the actual hardware timing requirements
struct TimingConstants;

impl TimingConstants {
    const ADDRESS_SETUP: Duration = Duration::from_nanos(100);  // T_ADDRESS_SETUP
    const DATA_VALID: Duration = Duration::from_nanos(200);     // T_DATA_VALID
    const OUTPUT_DISABLE: Duration = Duration::from_nanos(150); // T_OUTPUT_DISABLE
    const RAM_ACCESS: Duration = Duration::from_nanos(500);     // RAM access time
}

impl Intel4002 {
    /// Create a new Intel 4002 RAM with specified access time
    /// Parameters: name - Component identifier, access_time_ns - Memory access time in nanoseconds
    /// Returns: New Intel4002 instance with configurable access timing
    pub fn new(name: String) -> Self {
        Self::new_with_access_time(name, 500) // Default 500ns access time
    }

    /// Create a new Intel 4002 RAM with custom access time (for testing)
    /// Parameters: name - Component identifier, access_time_ns - Memory access time in nanoseconds
    /// Returns: New Intel4002 instance with configurable access timing
    pub fn new_with_access_time(name: String, access_time_ns: u64) -> Self {
        // Intel 4002 pinout (based on MCS-4 architecture):
        // - 4 data pins (D0-D3) for multiplexed address/data
        // - 4 output port pins (O0-O3)
        // - Control pins: SYNC, CM, P0, RESET
        // - Clock pins: Φ1, Φ2 (two-phase clock from 4004 CPU)
        //
        // Control pin behavior:
        // - SYNC: Marks start of instruction cycle
        // - CM: ROM chip select (must be HIGH for ROM access)
        // - P0: RAM chip select (must be HIGH for RAM access)
        // - RESET: Clears internal state
        let pin_names = vec![
            "D0", "D1", "D2", "D3",    // Data/Address pins
            "O0", "O1", "O2", "O3",    // Output port pins
            "SYNC",                    // Sync signal
            "CM",                      // ROM Chip Select
            "P0",                      // RAM Chip Select
            "RESET",                   // Reset
            "PHI1",                    // Clock phase 1
            "PHI2",                    // Clock phase 2
        ];

        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        Intel4002 {
            base: BaseComponent::new(name, pins),
            memory: [0u8; 80],  // 80 nibbles = 4 banks × 20 nibbles each
            last_address: 0,
            access_time: Duration::from_nanos(access_time_ns),
            address_latch_time: None,
            output_ports: [0u8; 4],
            input_latch: 0,
            status_characters: [0u8; 4],  // 4 separate status character latches
            bank_select: 0,
            prev_phi1: PinValue::Low,
            prev_phi2: PinValue::Low,
            address_high_nibble: None,
            address_low_nibble: None,
            full_address_ready: false,
            ram_state: RamState::Idle,
            data_latch: None,
            instruction_phase: false,
            current_instruction: 0,
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

    /// Initialize RAM with data
    /// Parameters: data - Binary data to load (max 80 nibbles)
    /// Returns: Ok(()) on success, Err(String) on failure
    pub fn initialize_ram(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() > 80 {
            return Err("Data exceeds RAM capacity (80 nibbles)".to_string());
        }

        for (i, &byte) in data.iter().enumerate() {
            self.memory[i] = byte & 0x0F; // Store only lower 4 bits
        }
        Ok(())
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

        data & 0x0F // Return only lower 4 bits
    }

    /// Drive the 4-bit data bus with the specified value
    /// Parameters: data - 4-bit value to drive on D0-D3 pins
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
                    // Use unique driver name to avoid conflicts with other components
                    pin_guard.set_driver(Some(format!("{}_DATA", self.base.name())), pin_value);
                }
            }
        }
    }

    /// Update output port pins based on current output port values
    /// Hardware: Output ports are driven continuously until changed or reset
    fn update_output_ports(&self) {
        for port in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("O{}", port)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    // Each output port drives its corresponding pin
                    let bit_value = (self.output_ports[port] >> 0) & 1;
                    let pin_value = if bit_value == 1 {
                        PinValue::High
                    } else {
                        PinValue::Low
                    };
                    // Use unique driver name for output ports
                    pin_guard.set_driver(Some(format!("{}_OUTPUT", self.base.name())), pin_value);
                }
            }
        }
    }

    /// Set data bus to high-impedance state to avoid bus contention
    /// CRITICAL: Must be called whenever RAM is not actively driving valid data
    fn tri_state_data_bus(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(format!("{}_DATA", self.base.name())), PinValue::HighZ);
                }
            }
        }
    }

    /// Tri-state output port pins
    /// Hardware: Output ports remain driven until explicitly changed
    fn tri_state_output_ports(&self) {
        for port in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("O{}", port)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(format!("{}_OUTPUT", self.base.name())), PinValue::HighZ);
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
    /// Returns: (sync, cm, p0, reset)
    /// Hardware: Control pins determine operation type and chip state
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

        let cm = if let Ok(pin) = self.base.get_pin("CM") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        let p0 = if let Ok(pin) = self.base.get_pin("P0") {
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

        (sync, cm, p0, reset)
    }

    /// Handle system reset signal
    /// Hardware: RESET pin clears all internal state and tri-states outputs
    fn handle_reset(&mut self) {
        let (_, _, _, reset) = self.read_control_pins();
        if reset {
            // Hardware reset - clear all registers
            self.memory = [0u8; 80];  // Clear 80 nibbles
            self.output_ports = [0u8; 4];
            self.input_latch = 0;
            self.status_characters = [0u8; 4];  // Clear 4 status character latches
            self.bank_select = 0;

            // Reset all state machines
            self.ram_state = RamState::Idle;
            self.address_latch_time = None;
            self.address_high_nibble = None;
            self.address_low_nibble = None;
            self.full_address_ready = false;
            self.data_latch = None;
            self.instruction_phase = false;
            self.current_instruction = 0;

            // Tri-state all outputs
            self.tri_state_data_bus();
            self.tri_state_output_ports();
        }
    }

    /// Assemble complete 8-bit address from high and low nibbles
    /// Hardware: Intel 4004 provides address in two 4-bit phases
    /// Format: (high_nibble << 4) | low_nibble
    fn assemble_full_address(&mut self) {
        if let (Some(high), Some(low)) = (self.address_high_nibble, self.address_low_nibble) {
            // Assemble 8-bit address: (high << 4) | low
            self.last_address = ((high as u8) << 4) | (low as u8);
            self.full_address_ready = true;
            self.address_latch_time = Some(Instant::now());

            // Clear nibble storage for next address
            self.address_high_nibble = None;
            self.address_low_nibble = None;
        }
    }

    /// Decode RAM address based on Intel 4002 addressing scheme
    /// Hardware: Complex addressing with 4 register banks, status characters, and output ports
    /// Parameters: address_low - Low nibble from CPU, instruction - Instruction/opcode
    /// Returns: (bank_number, effective_address)
    fn decode_address(&self, address_low: u8, instruction: u8) -> (u8, u8) {
        // The 4002 uses the high nibble of the address for bank selection
        // and the low nibble for within-bank addressing
        // Each bank has 20 nibbles (0-19), not 16
        let bank = (address_low >> 4) & 0x03;  // 2-bit bank from high nibble
        let ram_address = (bank * 20) + (address_low & 0x0F);  // 20 nibbles per bank

        // Status characters are separate latches, not part of RAM
        if self.is_status_character_instruction(instruction) {
            // Status characters are separate from RAM - return special address
            return (bank, 0xFF);  // Special marker for status character access
        }

        // Output ports are also separate from RAM (addresses 0x14-0x17)
        if address_low >= 0x14 && address_low <= 0x17 {
            return (bank, 0xFF);  // Special marker for output port access
        }

        (bank, ram_address)
    }

    /// Check if instruction is related to status character operations
    fn is_status_character_instruction(&self, instruction: u8) -> bool {
        matches!(instruction, 0x10..=0x13)
    }

    /// Check if instruction is a RAM-related instruction
    fn is_ram_instruction(&self, instruction: u8) -> bool {
        matches!(instruction, 0x00..=0x0F | 0x10..=0x17)
    }

    /// Handle RAM bank selection instructions
    /// Hardware: DCL (Designate Command Line) instructions select RAM banks
    fn handle_bank_selection(&mut self, instruction: u8) {
        match instruction {
            // Bank select instructions (DCL)
            0xE0..=0xE3 => {
                self.bank_select = instruction & 0x03;
            }
            _ => {}
        }
    }

    /// Handle status character operations
    /// Hardware: Status characters are separate 4-bit latches, not part of RAM
    fn handle_status_character(&mut self, instruction: u8) {
        match instruction {
            // Status character load instructions
            0xF0..=0xF3 => {
                let sc_index = (instruction & 0x03) as usize;
                if sc_index < 4 {
                    // Load status character from input latch into separate latch
                    self.status_characters[sc_index] = self.input_latch;
                }
            }
            _ => {}
        }
    }

    /// Handle output port operations
    /// Hardware: Output ports are separate from RAM, continuously driven
    fn handle_output_port_operation(&mut self, port: usize, data: u8) {
        if port < 4 {
            self.output_ports[port] = data & 0x0F;
            self.update_output_ports();
        }
    }

    /// Handle input latch operations
    /// Hardware: RDM (Read Data Memory) instructions read from input pins
    fn handle_input_latch_operation(&mut self) {
        // In real hardware, this would read from external input pins
        // For emulation, we use the input latch
        self.write_data_bus(self.input_latch);
    }

    /// Handle Φ1 rising edge - Address and control phase
    /// Hardware: Φ1 high = CPU drives bus with address/control information
    /// RAM operations start when SYNC goes high during Φ1 rising edge
    fn handle_phi1_rising(&mut self) {
        // Handle system reset first (highest priority)
        self.handle_reset();

        // Check for RAM operation start on Φ1 rising edge with SYNC high
        let (sync, cm, p0, _) = self.read_control_pins();
        if sync && p0 {
            // Start RAM address phase on Φ1 rising edge
            self.start_ram_address_phase();
        }

        // Handle RAM address phase operations during Φ1
        self.handle_ram_address_operations();
    }

    /// Handle Φ1 falling edge - End of address phase
    fn handle_phi1_falling(&mut self) {
        // Currently no specific operations needed on Φ1 falling
    }

    /// Handle Φ2 rising edge - Data phase
    fn handle_phi2_rising(&mut self) {
        self.handle_ram_data_operations();
    }

    /// Handle Φ2 falling edge - End of data phase
    fn handle_phi2_falling(&mut self) {
        self.handle_ram_cleanup_operations();
    }

    /// Handle RAM address-related operations during Φ1
    fn handle_ram_address_operations(&mut self) {
        match self.ram_state {
            RamState::Idle => {
                self.tri_state_data_bus();
            }

            RamState::AddressPhase => {
                self.handle_address_latching();
            }

            RamState::WaitLatency => {
                self.handle_latency_wait();
            }

            RamState::ReadData | RamState::WriteData | RamState::OutputPort => {
                self.tri_state_data_bus();
            }
        }
    }

    /// Handle RAM data-related operations during Φ2
    fn handle_ram_data_operations(&mut self) {
        match self.ram_state {
            RamState::Idle => {
                self.tri_state_data_bus();
            }

            RamState::AddressPhase => {
                self.tri_state_data_bus();
            }

            RamState::WaitLatency => {
                self.handle_latency_wait();
                if self.ram_state == RamState::ReadData || self.ram_state == RamState::WriteData {
                    self.handle_data_operations();
                }
            }

            RamState::ReadData => {
                self.handle_data_operations();
            }

            RamState::WriteData => {
                self.handle_data_operations();
            }

            RamState::OutputPort => {
                self.handle_output_port_state();
            }
        }
    }

    /// Handle RAM cleanup operations on Φ2 falling edge
    fn handle_ram_cleanup_operations(&mut self) {
        match self.ram_state {
            RamState::ReadData | RamState::WriteData | RamState::OutputPort => {
                self.tri_state_data_bus();
                self.return_to_idle();
            }

            RamState::Idle | RamState::AddressPhase | RamState::WaitLatency => {
                self.tri_state_data_bus();
            }
        }
    }

    /// Transition to address phase state
    fn start_ram_address_phase(&mut self) {
        self.ram_state = RamState::AddressPhase;
        self.full_address_ready = false;
    }

    /// Handle address nibble latching during address phase
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
        self.ram_state = RamState::WaitLatency;
        self.address_latch_time = Some(Instant::now());
    }

    /// Handle latency timing during wait state
    fn handle_latency_wait(&mut self) {
        if let Some(latch_time) = self.address_latch_time {
            if latch_time.elapsed() >= self.access_time {
                self.start_data_operation();
            }
        }
    }

    /// Update timing state with more precise hardware timing
    fn update_timing_state(&mut self) {
        match self.ram_state {
            RamState::AddressPhase => {
                if let Some(latch_time) = self.address_latch_time {
                    if latch_time.elapsed() >= TimingConstants::ADDRESS_SETUP {
                        self.start_data_operation();
                    }
                }
            }
            RamState::ReadData => {
                if let Some(latch_time) = self.address_latch_time {
                    if latch_time.elapsed() >= TimingConstants::DATA_VALID {
                        // Data should be valid now
                    }
                }
            }
            _ => {}
        }
    }

    /// Transition to data operation state
    fn start_data_operation(&mut self) {
        // Determine operation type based on control signals and address
        let (sync, cm, p0, _) = self.read_control_pins();

        if sync && p0 && self.full_address_ready {
            let address = self.last_address;

            if address >= 0x14 && address <= 0x17 {
                // Output port operation
                self.ram_state = RamState::OutputPort;
            } else {
                // RAM read/write operation
                // For now, assume read - write detection happens in data phase
                self.ram_state = RamState::ReadData;
            }
        }
    }

    /// Handle data operations during ReadData or WriteData state
    fn handle_data_operations(&mut self) {
        let (sync, cm, p0, _) = self.read_control_pins();

        if sync && p0 && self.full_address_ready {
            let address = self.last_address;

            if address == 0xFF {
                // Status character operation
                let data = self.read_data_bus();
                if !cm {
                    // First cycle - read from status character
                    // For now, return status character 0 - this should be determined by instruction
                    self.write_data_bus(self.status_characters[0]);
                } else {
                    // Second cycle - write to status character
                    // For now, write to status character 0 - this should be determined by instruction
                    self.status_characters[0] = data & 0x0F;
                    self.ram_state = RamState::WriteData;
                }
            } else if address < 80 {
                // RAM operation
                let data = self.read_data_bus();
                if !cm {
                    // First cycle - read from RAM
                    let ram_data = self.memory[address as usize];
                    self.write_data_bus(ram_data);
                } else {
                    // Second cycle - write to RAM
                    self.memory[address as usize] = data & 0x0F;
                    self.ram_state = RamState::WriteData;
                }
            }
        } else {
            self.tri_state_data_bus();
        }
    }

    /// Handle output port state operations
    fn handle_output_port_state(&mut self) {
        let (sync, cm, p0, _) = self.read_control_pins();

        if sync && p0 && self.full_address_ready {
            let address = self.last_address;

            if address >= 0x14 && address <= 0x17 {
                let port = (address - 0x14) as usize;
                if !cm {
                    // First cycle - read from output port (not typical for 4002)
                    let port_data = self.output_ports[port];
                    self.write_data_bus(port_data);
                } else {
                    // Second cycle - write to output port
                    let data = self.read_data_bus();
                    self.handle_output_port_operation(port, data);
                }
            }
        } else {
            self.tri_state_data_bus();
        }
    }

    /// Reset RAM state machine to idle
    fn return_to_idle(&mut self) {
        self.ram_state = RamState::Idle;
        self.address_latch_time = None;
        self.address_high_nibble = None;
        self.address_low_nibble = None;
        self.full_address_ready = false;
        self.data_latch = None;
        self.instruction_phase = false;
        self.current_instruction = 0;
    }

    /// Check if RAM should drive the bus
    fn should_drive_bus(&self) -> bool {
        let (sync, cm, p0, _) = self.read_control_pins();

        // Only drive bus during data phase when selected
        sync && p0 && self.ram_state == RamState::ReadData
    }

    /// Update data bus drivers with proper contention prevention
    fn update_data_bus_drivers(&self) {
        if self.should_drive_bus() {
            // Drive bus with RAM data
            let data = self.read_ram_data();
            self.write_data_bus(data);
        } else {
            // High impedance when not selected
            self.tri_state_data_bus();
        }
    }

    /// Read RAM data at current address
    fn read_ram_data(&self) -> u8 {
        if self.full_address_ready && (self.last_address as usize) < self.memory.len() {
            self.memory[self.last_address as usize]
        } else {
            0
        }
    }

    /// Handle instruction cycle synchronization
    fn handle_instruction_cycle(&mut self) {
        let (sync, cm, p0, _) = self.read_control_pins();

        if sync && self.is_phi1_rising_edge() {
            // Start of new instruction cycle
            self.instruction_phase = true;
            self.current_instruction = self.read_data_bus();

            // Decode instruction to determine if it's for RAM
            if self.is_ram_instruction(self.current_instruction) {
                self.prepare_ram_operation();
            }
        }
    }

    /// Prepare for RAM operation based on instruction
    fn prepare_ram_operation(&mut self) {
        // This would set up the RAM for the upcoming operation
        // based on the decoded instruction
    }

    /// Handle special RAM instructions (bank select, status characters, etc.)
    /// Hardware: These are handled during instruction cycles, not memory cycles
    fn handle_special_instructions(&mut self, instruction: u8) {
        match instruction {
            // DCL (Designate Command Line) - selects RAM bank
            0xF0..=0xF3 => {
                self.bank_select = instruction & 0x03;
            }

            // SRC (Send Register Control) - sets output port address
            0x20..=0x2F => {
                // SRC instruction sets up output port addressing
                // For now, this is handled by the output port operation logic
            }

            // Input latch operations (RDM - Read Data Memory)
            0x60..=0x63 => {
                self.handle_input_latch_operation();
            }

            _ => {
                // Other instructions may be handled elsewhere
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
            // Φ2 Rising Edge: Data phase (RAM drives bus if ready) - handle data operations
            self.handle_phi2_rising();
        }

        if phi2_falling {
            // Φ2 Falling Edge: End of data phase - tri-state bus and return to idle
            self.handle_phi2_falling();
        }

        // Handle special instructions when not in memory operation
        let (sync, cm, p0, _) = self.read_control_pins();
        if sync && !p0 && !cm {
            // Instruction phase - handle special RAM instructions
            let instruction = self.read_data_bus();
            self.handle_special_instructions(instruction);
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
        self.tri_state_output_ports();

        // Reset RAM operation state
        self.address_latch_time = None;
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

impl RunnableComponent for Intel4002 {}

// Intel 4002 specific methods
impl Intel4002 {
    /// Get the RAM size in nibbles
    /// Returns: Total number of nibbles in RAM (80 for 4002)
    pub fn get_ram_size(&self) -> usize {
        self.memory.len()
    }

    /// Read a nibble from RAM at specified address
    /// Parameters: address - RAM address (0-79)
    /// Returns: Some(data) if address valid, None if out of bounds
    pub fn read_ram(&self, address: u8) -> Option<u8> {
        if (address as usize) < self.memory.len() {
            Some(self.memory[address as usize])
        } else {
            None
        }
    }

    /// Write a nibble to RAM at specified address
    /// Parameters: address - RAM address (0-79), data - 4-bit data to write
    /// Returns: Ok(()) on success, Err(String) on failure
    pub fn write_ram(&mut self, address: u8, data: u8) -> Result<(), String> {
        if (address as usize) < self.memory.len() {
            self.memory[address as usize] = data & 0x0F;
            Ok(())
        } else {
            Err("Address out of range (0-79)".to_string())
        }
    }

    /// Get the current output port value
    /// Parameters: port - Port number (0-3)
    /// Returns: Some(data) if port valid, None if invalid
    pub fn get_output_port(&self, port: usize) -> Option<u8> {
        if port < 4 {
            Some(self.output_ports[port])
        } else {
            None
        }
    }

    /// Set an output port value
    /// Parameters: port - Port number (0-3), data - 4-bit data to set
    /// Returns: Ok(()) on success, Err(String) on failure
    pub fn set_output_port(&mut self, port: usize, data: u8) -> Result<(), String> {
        if port < 4 {
            self.output_ports[port] = data & 0x0F;
            self.update_output_ports();
            Ok(())
        } else {
            Err("Port number out of range (0-3)".to_string())
        }
    }

    /// Set the input latch value
    /// Parameters: data - 4-bit input data
    pub fn set_input_latch(&mut self, data: u8) {
        self.input_latch = data & 0x0F;
    }

    /// Get the current input latch value
    /// Returns: 4-bit input latch value
    pub fn get_input_latch(&self) -> u8 {
        self.input_latch
    }

    /// Get the current status character value
    /// Parameters: index - Status character index (0-3)
    /// Returns: 4-bit status character value
    pub fn get_status_character(&self, index: usize) -> Option<u8> {
        if index < 4 {
            Some(self.status_characters[index])
        } else {
            None
        }
    }

    /// Get all status characters
    /// Returns: Array of 4 status character values
    pub fn get_all_status_characters(&self) -> [u8; 4] {
        self.status_characters
    }

    /// Get the current bank select value
    /// Returns: 2-bit bank select value (0-3)
    pub fn get_bank_select(&self) -> u8 {
        self.bank_select
    }

    /// Clear all RAM to zero
    pub fn clear_ram(&mut self) {
        self.memory = [0u8; 80];  // Clear 80 nibbles
    }

    /// Get all RAM data for a specific bank
    /// Parameters: bank - Bank number (0-3)
    /// Returns: Vector of 20 nibbles for the bank (4 banks × 20 nibbles = 80 total)
    pub fn get_ram_bank(&self, bank: u8) -> Vec<u8> {
        let bank = (bank & 0x03) as usize;
        let start = bank * 20;  // 20 nibbles per bank
        let end = start + 20;

        if end <= self.memory.len() {
            self.memory[start..end].to_vec()
        } else if start >= self.memory.len() {
            Vec::new()
        } else {
            self.memory[start..].to_vec()
        }
    }

    /// Debug function to log state transitions for troubleshooting
    /// Parameters: test_name - Name of the test for context
    pub fn debug_state_transitions(&self, test_name: &str) {
        println!("{} - State: {:?}, Bank: {}, High: {:?}, Low: {:?}, Address: 0x{:x}, Ready: {}",
                 test_name, self.ram_state, self.bank_select, self.address_high_nibble,
                 self.address_low_nibble, self.last_address, self.full_address_ready);
    }

    /// Setup helper for instruction testing
    pub fn setup_instruction_test(&mut self, instruction: u8) {
        self.current_instruction = instruction;
        self.instruction_phase = true;
    }

    /// Tri-state all outputs (comprehensive tri-state)
    fn tri_state_all_outputs(&self) {
        self.tri_state_data_bus();
        self.tri_state_output_ports();
    }
}

// Custom formatter for debugging
impl std::fmt::Display for RamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RamState::Idle => write!(f, "Idle"),
            RamState::AddressPhase => write!(f, "AddressPhase"),
            RamState::WaitLatency => write!(f, "WaitLatency"),
            RamState::ReadData => write!(f, "ReadData"),
            RamState::WriteData => write!(f, "WriteData"),
            RamState::OutputPort => write!(f, "OutputPort"),
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
        assert_eq!(ram.get_ram_size(), 80);
        assert_eq!(ram.get_access_time(), 500); // Default 500ns
        assert!(!ram.is_running());
    }

    #[test]
    fn test_intel4002_ram_operations() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Test RAM write/read
        assert!(ram.write_ram(0, 0x0A).is_ok());
        assert_eq!(ram.read_ram(0).unwrap(), 0x0A);

        // Test out of bounds
        assert!(ram.write_ram(80, 0x0F).is_err());
        assert!(ram.read_ram(80).is_none());
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
        ram.write_ram(0, 0x01).unwrap();  // Bank 0, address 0
        ram.write_ram(20, 0x02).unwrap(); // Bank 1, address 0
        ram.write_ram(40, 0x03).unwrap(); // Bank 2, address 0

        let bank0 = ram.get_ram_bank(0);
        assert_eq!(bank0.len(), 20); // 20 nibbles per bank
        assert_eq!(bank0[0], 0x01);

        let bank1 = ram.get_ram_bank(1);
        assert_eq!(bank1.len(), 20); // 20 nibbles per bank
        assert_eq!(bank1[0], 0x02);

        let bank2 = ram.get_ram_bank(2);
        assert_eq!(bank2.len(), 20); // 20 nibbles per bank
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

    #[test]
    fn test_configurable_access_time() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Test default access time
        assert_eq!(ram.get_access_time(), 500);

        // Test setting custom access time
        ram.set_access_time(100);
        assert_eq!(ram.get_access_time(), 100);

        // Test constructor with custom access time
        let fast_ram = Intel4002::new_with_access_time("FAST_RAM".to_string(), 1);
        assert_eq!(fast_ram.get_access_time(), 1);
        assert_eq!(fast_ram.name(), "FAST_RAM");
    }

    #[test]
    fn test_address_latching() {
        let mut ram = Intel4002::new_with_access_time("RAM_4002".to_string(), 1);

        // Get pin references
        let sync_pin = ram.get_pin("SYNC").unwrap();
        let p0_pin = ram.get_pin("P0").unwrap();
        let phi1_pin = ram.get_pin("PHI1").unwrap();
        let d0_pin = ram.get_pin("D0").unwrap();
        let d1_pin = ram.get_pin("D1").unwrap();
        let d2_pin = ram.get_pin("D2").unwrap();
        let d3_pin = ram.get_pin("D3").unwrap();

        // Initialize clock pins to Low
        {
            let mut phi1_guard = phi1_pin.lock().unwrap();
            phi1_guard.set_driver(Some("TEST".to_string()), PinValue::Low);
        }

        // Set up RAM operation: SYNC=1, CM-RAM=1
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut p0_guard = p0_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            p0_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Set address high nibble (0x0) on data bus
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

        // High nibble
        phi1_pin.lock().unwrap().set_driver(Some("TEST".into()), PinValue::High);
        ram.update(); // rising edge -> latch high nibble
        phi1_pin.lock().unwrap().set_driver(Some("TEST".into()), PinValue::Low);
        ram.update(); // falling edge

        // Should have transitioned to AddressPhase
        assert_eq!(ram.ram_state, RamState::AddressPhase);
        assert_eq!(ram.address_high_nibble, Some(0x0));

        // Set address low nibble (0x0) on data bus
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

        // Low nibble
        phi1_pin.lock().unwrap().set_driver(Some("TEST".into()), PinValue::High);
        ram.update(); // rising edge -> latch low nibble
        phi1_pin.lock().unwrap().set_driver(Some("TEST".into()), PinValue::Low);
        ram.update(); // falling edge

        // Should have assembled full address and transitioned to WaitLatency
        assert_eq!(ram.last_address, 0x00);
        assert_eq!(ram.full_address_ready, true);
        assert_eq!(ram.ram_state, RamState::WaitLatency);
    }

    #[test]
    fn test_reset_behavior() {
        let mut ram = Intel4002::new_with_access_time("RAM_4002".to_string(), 1);

        // Set up some state first
        let sync_pin = ram.get_pin("SYNC").unwrap();
        let p0_pin = ram.get_pin("P0").unwrap();
        let reset_pin = ram.get_pin("RESET").unwrap();

        // Set RAM operation active
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut p0_guard = p0_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            p0_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Trigger some state changes
        ram.update();

        // Now assert RESET
        {
            let mut reset_guard = reset_pin.lock().unwrap();
            reset_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Update with RESET high - should clear all state
        ram.update();

        // Verify all state is cleared
        assert_eq!(ram.get_output_port(0).unwrap(), 0);
        assert_eq!(ram.get_input_latch(), 0);
        assert_eq!(ram.get_bank_select(), 0);

        // Verify data bus is tri-stated
        let d0_pin = ram.get_pin("D0").unwrap();
        assert_eq!(d0_pin.lock().unwrap().read(), PinValue::HighZ);
    }

    #[test]
    fn test_bank_selection() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Test bank selection instructions
        ram.handle_bank_selection(0xE0);
        assert_eq!(ram.get_bank_select(), 0);

        ram.handle_bank_selection(0xE1);
        assert_eq!(ram.get_bank_select(), 1);

        ram.handle_bank_selection(0xE2);
        assert_eq!(ram.get_bank_select(), 2);

        ram.handle_bank_selection(0xE3);
        assert_eq!(ram.get_bank_select(), 3);
    }

    #[test]
    fn test_status_character_operations() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Set input latch
        ram.set_input_latch(0x0A);

        // Test status character load
        ram.handle_status_character(0xF0);
        assert_eq!(ram.get_status_character(0).unwrap(), 0x0A); // Status character 0

        ram.set_input_latch(0x0B);
        ram.handle_status_character(0xF1);
        assert_eq!(ram.get_status_character(1).unwrap(), 0x0B); // Status character 1
    }

    #[test]
    fn test_output_port_operations() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Test output port operations
        ram.handle_output_port_operation(0, 0x05);
        assert_eq!(ram.get_output_port(0).unwrap(), 0x05);

        ram.handle_output_port_operation(1, 0x0A);
        assert_eq!(ram.get_output_port(1).unwrap(), 0x0A);

        // Test invalid port (should not crash)
        ram.handle_output_port_operation(4, 0x0F);
    }

    #[test]
    fn test_input_latch_operations() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        // Set input latch
        ram.set_input_latch(0x07);

        // Mock data bus read to return input latch value
        // In real test, this would involve setting up pin states

        // For now, just verify the latch is set correctly
        assert_eq!(ram.get_input_latch(), 0x07);
    }

    #[test]
    fn test_ram_addressing_scheme() {
        let ram = Intel4002::new("RAM_4002".to_string());

        // Test address decoding for different instruction types
        // For instruction 0x05 with address_low 0x00: should be bank 0, address 0
        let (bank, address) = ram.decode_address(0x00, 0x05);
        assert_eq!(bank, 0);
        assert_eq!(address, 0); // Bank 0, address 0 (instruction 0x05 addresses location 0)

        // Test with different address_low value
        let (bank, address) = ram.decode_address(0x05, 0x00);
        assert_eq!(bank, 0);
        assert_eq!(address, 5); // Bank 0, address 5

        // Test status character addressing (returns 0xFF marker)
        let (_bank, address) = ram.decode_address(0x00, 0x10);
        assert_eq!(address, 0xFF); // Status character marker

        // Test output port addressing (returns 0xFF marker)
        // For output port 0x14, we need to pass 0x14 as address_low
        let (_bank, address) = ram.decode_address(0x14, 0x00);
        assert_eq!(address, 0xFF); // Output port marker
    }

    #[test]
    fn test_ram_initialization() {
        let mut ram = Intel4002::new("RAM_4002".to_string());

        let test_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        assert!(ram.initialize_ram(&test_data).is_ok());

        assert_eq!(ram.read_ram(0).unwrap(), 0x01);
        assert_eq!(ram.read_ram(1).unwrap(), 0x02);
        assert_eq!(ram.read_ram(2).unwrap(), 0x03);
        assert_eq!(ram.read_ram(3).unwrap(), 0x04);
        assert_eq!(ram.read_ram(4).unwrap(), 0x05);

        // Test overflow
        let large_data = vec![0u8; 81];  // More than 80 nibbles
        assert!(ram.initialize_ram(&large_data).is_err());
    }

    #[test]
    fn test_ram_state_transitions() {
        let mut ram = Intel4002::new_with_access_time("RAM_4002".to_string(), 1);

        // Initially should be in idle state
        assert_eq!(ram.ram_state, RamState::Idle);

        // Test state transitions through debug function
        ram.debug_state_transitions("INITIAL");
        assert_eq!(ram.ram_state, RamState::Idle);
    }

    #[test]
    fn test_pin_configuration() {
        let ram = Intel4002::new("RAM_4002".to_string());

        // Test that all expected pins are present
        assert!(ram.get_pin("D0").is_ok());
        assert!(ram.get_pin("D1").is_ok());
        assert!(ram.get_pin("D2").is_ok());
        assert!(ram.get_pin("D3").is_ok());
        assert!(ram.get_pin("O0").is_ok());
        assert!(ram.get_pin("O1").is_ok());
        assert!(ram.get_pin("O2").is_ok());
        assert!(ram.get_pin("O3").is_ok());
        assert!(ram.get_pin("SYNC").is_ok());
        assert!(ram.get_pin("CM").is_ok());
        assert!(ram.get_pin("P0").is_ok());
        assert!(ram.get_pin("RESET").is_ok());
        assert!(ram.get_pin("PHI1").is_ok());
        assert!(ram.get_pin("PHI2").is_ok());
    }

    #[test]
    fn test_dcl_instruction() {
        let mut ram = Intel4002::new("TEST".to_string());

        // Simulate DCL instruction on data bus during instruction phase
        ram.setup_instruction_test(0xF1); // DCL to select bank 1
        ram.handle_special_instructions(0xF1);

        assert_eq!(ram.get_bank_select(), 1);
    }

    #[test]
    fn test_src_instruction() {
        let mut ram = Intel4002::new("TEST".to_string());

        // SRC instruction sets up output port addressing
        ram.setup_instruction_test(0x25); // SRC with specific address
        // Verify output port addressing is set up correctly
        // This would require more complex setup in a real implementation
    }

    #[test]
    fn test_comprehensive_reset() {
        let mut ram = Intel4002::new_with_access_time("RAM_4002".to_string(), 1);

        // Set up some non-zero state
        ram.write_ram(0, 0x0A).unwrap();
        ram.set_output_port(0, 0x05).unwrap();
        ram.set_input_latch(0x07);
        ram.bank_select = 2;

        // Verify state is set
        assert_eq!(ram.read_ram(0).unwrap(), 0x0A);
        assert_eq!(ram.get_output_port(0).unwrap(), 0x05);
        assert_eq!(ram.get_input_latch(), 0x07);
        assert_eq!(ram.get_bank_select(), 2);

        // Set RESET pin to HIGH first
        let reset_pin = ram.get_pin("RESET").unwrap();
        {
            let mut reset_guard = reset_pin.lock().unwrap();
            reset_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Apply reset by calling handle_reset directly
        ram.handle_reset();

        // Verify comprehensive reset
        assert_eq!(ram.read_ram(0).unwrap(), 0); // RAM cleared
        assert_eq!(ram.get_output_port(0).unwrap(), 0); // Output ports cleared
        assert_eq!(ram.get_input_latch(), 0); // Input latch cleared
        assert_eq!(ram.get_status_character(0).unwrap(), 0); // Status characters cleared
        assert_eq!(ram.get_bank_select(), 0); // Bank select cleared
        assert_eq!(ram.ram_state, RamState::Idle); // State machine reset
    }

    #[test]
    fn test_bus_contention_prevention() {
        let mut ram = Intel4002::new_with_access_time("RAM_4002".to_string(), 1);

        // Initially should not drive bus
        assert!(!ram.should_drive_bus());

        // Set up RAM operation
        let sync_pin = ram.get_pin("SYNC").unwrap();
        let p0_pin = ram.get_pin("P0").unwrap();
        {
            let mut sync_guard = sync_pin.lock().unwrap();
            let mut p0_guard = p0_pin.lock().unwrap();
            sync_guard.set_driver(Some("TEST".to_string()), PinValue::High);
            p0_guard.set_driver(Some("TEST".to_string()), PinValue::High);
        }

        // Start memory operation
        ram.update();

        // Should still not drive bus until data phase
        assert!(!ram.should_drive_bus());

        // Test that bus drivers are updated correctly
        ram.update_data_bus_drivers();
        // In a real test, we would verify the pin states
    }
}
