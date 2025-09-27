use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component, RunnableComponent};
use crate::pin::{Pin, PinValue};
use crate::types::U12;

/// Represents the current phase of instruction execution
/// The 4004 CPU processes instructions in distinct phases synchronized with the clock
#[derive(Debug, Clone, Copy, PartialEq)]
enum InstructionPhase {
    Fetch,   // Fetching instruction from memory
    Address, // Calculating or fetching address
    Execute, // Executing the instruction
    Wait,    // Waiting for external operations
}

/// Represents the current phase of the two-phase clock cycle
/// The 4004 uses a two-phase clock for synchronization with peripherals
#[derive(Debug, Clone, Copy, PartialEq)]
enum ClockPhase {
    Phase1, // First clock phase - CPU drives bus
    Phase2, // Second clock phase - Peripherals drive bus
}

/// Intel 4004 4-bit microprocessor implementation
/// The world's first microprocessor, featuring 4-bit data bus, 12-bit addressing,
/// 46 instructions, and 16 index registers. Part of the MCS-4 family.
pub struct Intel4004 {
    base: BaseComponent,
    accumulator: u8,                     // Main accumulator register (4-bit)
    carry: bool,                         // Carry flag for arithmetic operations
    index_registers: [u8; 16],           // 16 4-bit index registers (R0-R15)
    pub(crate) program_counter: U12,     // 12-bit program counter
    stack: [U12; 3],                     // 3-level 12-bit address stack
    stack_pointer: u8,                   // Stack pointer (0-2)
    cycle_count: u64,                    // Total number of clock cycles executed
    instruction_phase: InstructionPhase, // Current instruction execution phase
    current_instruction: u8,             // Currently executing instruction
    address_latch: u8,                   // Latched address for memory operations
    data_latch: u8,                      // Latched data for memory operations
    clock_speed: f64,                    // Target clock speed in Hz
    last_clock_transition: Instant,      // Timestamp of last clock transition
    clock_phase: ClockPhase,             // Current clock phase
    rom_port: u8,                        // Currently selected ROM port (0-15)
    ram_bank: u8,                        // Currently selected RAM bank (0-7)
}

impl Intel4004 {
    /// Create a new Intel 4004 CPU instance
    /// Parameters: name - Component identifier, clock_speed - Target clock frequency in Hz
    /// Returns: New Intel4004 instance with initialized state
    pub fn new(name: String, clock_speed: f64) -> Self {
        let pin_names = vec![
            "D0", "D1", "D2", "D3", "SYNC", "CM_ROM", "CM_RAM", "TEST", "RESET", "PHI1", "PHI2",
        ];

        let pin_strings: Vec<String> = pin_names.iter().map(|s| s.to_string()).collect();
        let pin_refs: Vec<&str> = pin_strings.iter().map(|s| s.as_str()).collect();
        let pins = BaseComponent::create_pin_map(&pin_refs, &name);

        Intel4004 {
            base: BaseComponent::new(name, pins),
            accumulator: 0,
            carry: false,
            index_registers: [0u8; 16],
            program_counter: U12::new(0),
            stack: [U12::new(0); 3],
            stack_pointer: 0,
            cycle_count: 0,
            instruction_phase: InstructionPhase::Fetch,
            current_instruction: 0,
            address_latch: 0,
            data_latch: 0,
            clock_speed,
            last_clock_transition: Instant::now(),
            clock_phase: ClockPhase::Phase1,
            rom_port: 0,
            ram_bank: 0,
        }
    }

    /// Set the initial program counter value for the CPU
    /// Parameters: self - CPU instance, pc - Initial 12-bit program counter value
    /// Returns: Modified CPU instance with new program counter
    pub fn with_initial_pc(mut self, pc: u16) -> Self {
        self.program_counter = U12::new(pc);
        self
    }

    /// Reset the CPU to its initial state
    /// Clears all registers, resets program counter, and tri-states all outputs
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.carry = false;
        self.index_registers = [0u8; 16];
        self.program_counter = U12::new(0);
        self.stack = [U12::new(0); 3];
        self.stack_pointer = 0;
        self.instruction_phase = InstructionPhase::Fetch;
        self.rom_port = 0;
        self.ram_bank = 0;

        self.set_sync(false);
        self.set_cm_rom(false);
        self.set_cm_ram(false);
        self.tri_state_data_bus();
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

        data & 0x0F
    }

    /// Drive the 4-bit data bus with the specified value
    /// Parameters: data - 4-bit value to drive on D0-D3 pins
    fn write_data_bus(&self, data: u8) {
        let nibble = data & 0x0F;

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

    /// Set data bus to high-impedance state to avoid bus contention
    /// CRITICAL: Must be called whenever CPU is not actively driving valid data
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

    /// Set the SYNC pin state
    /// Parameters: high - true for high voltage, false for low voltage
    fn set_sync(&self, high: bool) {
        if let Ok(pin) = self.base.get_pin("SYNC") {
            if let Ok(mut pin_guard) = pin.lock() {
                let value = if high { PinValue::High } else { PinValue::Low };
                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), value);
            }
        }
    }

    /// Set the CM-ROM (Chip Select ROM) pin state
    /// Parameters: high - true for high voltage, false for low voltage
    fn set_cm_rom(&self, high: bool) {
        if let Ok(pin) = self.base.get_pin("CM_ROM") {
            if let Ok(mut pin_guard) = pin.lock() {
                let value = if high { PinValue::High } else { PinValue::Low };
                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), value);
            }
        }
    }

    /// Set the CM-RAM (Chip Select RAM) pin state
    /// Parameters: high - true for high voltage, false for low voltage
    fn set_cm_ram(&self, high: bool) {
        if let Ok(pin) = self.base.get_pin("CM_RAM") {
            if let Ok(mut pin_guard) = pin.lock() {
                let value = if high { PinValue::High } else { PinValue::Low };
                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), value);
            }
        }
    }

    /// Read the state of all control pins
    /// Returns: (sync, cm_rom, cm_ram, test) - State of control signals
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

        // Simplified for now - just return defaults
        (sync, false, false, true)
    }

    /// Handle clock cycle processing
    /// Updates cycle count and manages clock phase transitions
    fn handle_clock(&mut self) {
        // Simple clock simulation for now
        self.cycle_count += 1;
    }

    /// Execute a single instruction (simplified for demo purposes)
    /// For Fibonacci demo, simulate simple execution with periodic accumulator updates
    fn execute_instruction(&mut self) {
        // For Fibonacci demo, simulate simple execution
        // Just increment PC and occasionally update accumulator to simulate Fibonacci sequence
        self.program_counter.inc();

        // Every 10 cycles, "calculate" next Fibonacci number
        if self.cycle_count % 1000 == 0 {
            // Simple Fibonacci simulation
            let fib_index = (self.cycle_count / 10) as u8;
            self.accumulator = self.simulate_fibonacci(fib_index);
        }
    }

    /// Simulate Fibonacci number calculation for demo purposes
    /// Parameters: n - Fibonacci sequence index
    /// Returns: nth Fibonacci number modulo 16 (4-bit value)
    fn simulate_fibonacci(&self, n: u8) -> u8 {
        match n {
            0 => 0,
            1 => 1,
            _ => {
                let mut a = 0;
                let mut b = 1;
                for _ in 2..=n % 32 {
                    let next: u32 = a + b;
                    a = b;
                    b = next;
                }
                (b % 16) as u8 // Keep it in 4-bit range for demo
            }
        }
    }

    /// Get the current program counter value
    /// Returns: 12-bit program counter as 16-bit value
    pub fn get_program_counter(&self) -> u16 {
        self.program_counter.value()
    }

    /// Set the program counter to a specific address
    /// Parameters: address - New 12-bit program counter value
    pub fn set_program_counter(&mut self, address: u16) {
        self.program_counter.set(address);
    }

    /// Get the current accumulator value
    /// Returns: 4-bit accumulator value
    pub fn get_accumulator(&self) -> u8 {
        self.accumulator
    }

    /// Set the accumulator to a specific value
    /// Parameters: value - New 4-bit accumulator value (will be masked to 4 bits)
    pub fn set_accumulator(&mut self, value: u8) {
        self.accumulator = value & 0x0F;
    }

    /// Get the current carry flag state
    /// Returns: true if carry is set, false otherwise
    pub fn get_carry(&self) -> bool {
        self.carry
    }

    /// Get the current stack pointer value
    /// Returns: Stack pointer (0-2 for the 3-level stack)
    pub fn get_stack_pointer(&self) -> u8 {
        self.stack_pointer
    }

    /// Get the total number of clock cycles executed
    /// Returns: Total cycle count since reset
    pub fn get_cycle_count(&self) -> u64 {
        self.cycle_count
    }

    /// Get the configured clock speed
    /// Returns: Clock speed in Hz
    pub fn get_clock_speed(&self) -> f64 {
        self.clock_speed
    }

    /// Set an index register to a specific value
    /// Parameters: index - Register index (0-15), value - New 4-bit register value
    /// Returns: Ok(()) if successful, Err(String) if index out of range
    pub fn set_register(&mut self, index: u8, value: u8) -> Result<(), String> {
        if index < 16 {
            self.index_registers[index as usize] = value & 0x0F;
            Ok(())
        } else {
            Err("Register index out of range".to_string())
        }
    }

    /// Get the value of an index register
    /// Parameters: index - Register index (0-15)
    /// Returns: Some(register_value) if index valid, None if out of range
    pub fn get_register(&self, index: u8) -> Option<u8> {
        if index < 16 {
            Some(self.index_registers[index as usize])
        } else {
            None
        }
    }
}

impl Component for Intel4004 {
    fn name(&self) -> String {
        self.base.name()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.base.get_pin(name)
    }

    /// Update the CPU state for one simulation cycle
    /// Processes clock cycles and executes instructions when running
    fn update(&mut self) {
        if !self.is_running() {
            return;
        }

        self.handle_clock();
        self.execute_instruction();
    }

    /// Run the CPU in a continuous loop until stopped
    /// Provides a time-sliced execution model with 10 microsecond delays between cycles
    fn run(&mut self) {
        // Time-slice model: run in a loop calling update() each cycle
        self.base.set_running(true);
        self.reset();

        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10));
        }
    }

    /// Stop the CPU and clean up resources
    /// Tri-states all outputs and prepares for shutdown
    fn stop(&mut self) {
        self.base.set_running(false);
        self.tri_state_data_bus();
        self.set_sync(false);
        self.set_cm_rom(false);
        self.set_cm_ram(false);
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

impl RunnableComponent for Intel4004 {
    // No custom run_loop needed - uses default Component::run() method
    // The default implementation spawns the component in its own thread
}
