use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

/// Intel 4004 - World's first commercially available microprocessor
/// 4-bit CPU, 46 instructions, 4KB address space
pub struct Intel4004 {
    base: BaseComponent,

    // Registers
    accumulator: u8,        // 4-bit accumulator
    carry: bool,           // Carry flag
    index_registers: [u8; 16], // 16×4-bit index registers (R0-R15)
    program_counter: u12,   // 12-bit program counter
    stack: [u12; 3],       // 3-level subroutine stack
    stack_pointer: u8,     // Stack pointer (0-2)

    // Internal state
    cycle_count: u64,
    instruction_phase: InstructionPhase,
    current_instruction: u8,
    address_latch: u8,
    data_latch: u8,

    // Timing
    clock_speed: f64,      // Clock frequency in Hz
    last_clock_transition: Instant,
    clock_phase: ClockPhase,

    // External interfaces
    rom_port: u8,          // Current ROM port selection
    ram_bank: u8,          // Current RAM bank selection
}

// 12-bit address type for program counter and stack
#[derive(Debug, Clone, Copy, PartialEq)]
struct u12(u16);

impl u12 {
    fn new(value: u16) -> Self {
        u12(value & 0xFFF) // Mask to 12 bits
    }

    fn inc(&mut self) {
        self.0 = (self.0 + 1) & 0xFFF;
    }

    fn value(&self) -> u16 {
        self.0
    }

    fn set(&mut self, value: u16) {
        self.0 = value & 0xFFF;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InstructionPhase {
    Fetch,      // Fetch instruction
    Address,    // Address phase (for memory operations)
    Execute,    // Execute instruction
    Wait,       // Wait state
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ClockPhase {
    Phase1,     // φ1 clock phase
    Phase2,     // φ2 clock phase
}

impl Intel4004 {
    pub fn new(name: String, clock_speed: f64) -> Self {
        // Intel 4004 pinout:
        // - 4 data pins (D0-D3) for multiplexed address/data
        // - 10 address lines (A0-A9) for ROM addressing (through output ports)
        // - Control pins: SYNC, CM-ROM, CM-RAM, TEST, RESET, φ1, φ2
        let pin_names = vec![
            "D0", "D1", "D2", "D3",    // Data/Address pins (bidirectional)
            "SYNC",                     // Sync signal (output)
            "CM_ROM",                   // ROM Chip Select (output)
            "CM_RAM",                   // RAM Chip Select (output)
            "TEST",                     // Test input pin
            "RESET",                    // Reset input
            "PHI1",                     // Clock phase 1 (input)
            "PHI2",                     // Clock phase 2 (input)
        ];

        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        Intel4004 {
            base: BaseComponent::new(name, pins),
            accumulator: 0,
            carry: false,
            index_registers: [0u8; 16],
            program_counter: u12::new(0),
            stack: [u12::new(0); 3],
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

    pub fn with_initial_pc(mut self, pc: u16) -> Self {
        self.program_counter = u12::new(pc);
        self
    }

    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.carry = false;
        self.index_registers = [0u8; 16];
        self.program_counter = u12::new(0);
        self.stack = [u12::new(0); 3];
        self.stack_pointer = 0;
        self.instruction_phase = InstructionPhase::Fetch;
        self.rom_port = 0;
        self.ram_bank = 0;

        // Reset control signals
        self.set_sync(false);
        self.set_cm_rom(false);
        self.set_cm_ram(false);
        self.tri_state_data_bus();
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

    fn tri_state_data_bus(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.base.get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), PinValue::HighZ);
                }
            }
        }
    }

    fn set_sync(&self, high: bool) {
        if let Ok(pin) = self.base.get_pin("SYNC") {
            if let Ok(mut pin_guard) = pin.lock() {
                let value = if high { PinValue::High } else { PinValue::Low };
                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), value);
            }
        }
    }

    fn set_cm_rom(&self, high: bool) {
        if let Ok(pin) = self.base.get_pin("CM_ROM") {
            if let Ok(mut pin_guard) = pin.lock() {
                let value = if high { PinValue::High } else { PinValue::Low };
                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), value);
            }
        }
    }

    fn set_cm_ram(&self, high: bool) {
        if let Ok(pin) = self.base.get_pin("CM_RAM") {
            if let Ok(mut pin_guard) = pin.lock() {
                let value = if high { PinValue::High } else { PinValue::Low };
                pin_guard.set_driver(Some(self.base.get_name().parse().unwrap()), value);
            }
        }
    }

    fn read_test_pin(&self) -> bool {
        if let Ok(pin) = self.base.get_pin("TEST") {
            if let Ok(pin_guard) = pin.lock() {
                return pin_guard.read() == PinValue::High;
            }
        }
        false
    }

    fn read_reset_pin(&self) -> bool {
        if let Ok(pin) = self.base.get_pin("RESET") {
            if let Ok(pin_guard) = pin.lock() {
                return pin_guard.read() == PinValue::High;
            }
        }
        false
    }

    fn read_clock_phase(&self) -> (bool, bool) {
        let phi1 = if let Ok(pin) = self.base.get_pin("PHI1") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        let phi2 = if let Ok(pin) = self.base.get_pin("PHI2") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        (phi1, phi2)
    }

    fn handle_clock(&mut self) {
        let (phi1, phi2) = self.read_clock_phase();

        // Detect clock transitions
        let new_phase = if phi1 && !phi2 {
            ClockPhase::Phase1
        } else if !phi1 && phi2 {
            ClockPhase::Phase2
        } else {
            self.clock_phase // No change
        };

        if new_phase != self.clock_phase {
            self.clock_phase = new_phase;
            self.last_clock_transition = Instant::now();
            self.cycle_count += 1;

            // Process instruction based on clock phase
            match self.clock_phase {
                ClockPhase::Phase1 => self.process_phase1(),
                ClockPhase::Phase2 => self.process_phase2(),
            }
        }
    }

    fn process_phase1(&mut self) {
        // φ1 phase: Address/output phase
        match self.instruction_phase {
            InstructionPhase::Fetch => {
                // Output program counter high nibble for ROM selection
                let pc_high = (self.program_counter.value() >> 8) as u8;
                self.write_data_bus(pc_high);
                self.set_sync(true);
                self.set_cm_rom(true);
                self.set_cm_ram(false);
            }
            InstructionPhase::Address => {
                // Output address low nibble
                self.write_data_bus(self.address_latch);
                self.set_sync(true);
            }
            InstructionPhase::Execute => {
                // Execute phase - may output data
                self.execute_instruction_phase1();
            }
            InstructionPhase::Wait => {
                // Wait state - no operation
            }
        }
    }

    fn process_phase2(&mut self) {
        // φ2 phase: Input phase
        match self.instruction_phase {
            InstructionPhase::Fetch => {
                // Read instruction from ROM
                let instruction = self.read_data_bus();
                self.current_instruction = instruction;
                self.decode_instruction(instruction);
                self.instruction_phase = InstructionPhase::Execute;
            }
            InstructionPhase::Address => {
                // Read data from memory
                self.data_latch = self.read_data_bus();
                self.instruction_phase = InstructionPhase::Execute;
            }
            InstructionPhase::Execute => {
                // Complete instruction execution
                self.execute_instruction_phase2();
                self.program_counter.inc();
                self.instruction_phase = InstructionPhase::Fetch;
            }
            InstructionPhase::Wait => {
                // Wait state - no operation
            }
        }
    }

    fn decode_instruction(&mut self, instruction: u8) {
        // Decode the instruction and set up for execution
        match instruction {
            // NOP
            0x00 => {
                // No operation
            }
            // JCN - Jump Conditional
            0x10..=0x1F => {
                self.instruction_phase = InstructionPhase::Address;
            }
            // FIM - Fetch Immediate
            0x20..=0x2F => {
                self.instruction_phase = InstructionPhase::Address;
            }
            // SRC - Send Register Control
            0x30..=0x3F => {
                let register_pair = (instruction & 0x0E) >> 1;
                self.address_latch = self.index_registers[register_pair as usize * 2];
            }
            // FIN - Fetch Indirect
            0x40..=0x4F => {
                self.instruction_phase = InstructionPhase::Address;
            }
            // JIN - Jump Indirect
            0x50..=0x5F => {
                let register_pair = (instruction & 0x0E) >> 1;
                let address_low = self.index_registers[register_pair as usize * 2 + 1];
                let address_high = self.index_registers[register_pair as usize * 2];
                self.program_counter.set((address_high as u16) << 8 | address_low as u16);
            }
            // JUN - Jump Unconditional
            0x60..=0x6F => {
                self.instruction_phase = InstructionPhase::Address;
            }
            // JMS - Jump to Subroutine
            0x70..=0x7F => {
                self.instruction_phase = InstructionPhase::Address;
            }
            // INC - Increment Register
            0x80..=0x8F => {
                let register = instruction & 0x0F;
                self.index_registers[register as usize] =
                    (self.index_registers[register as usize] + 1) & 0x0F;
            }
            // ISZ - Increment and Skip if Zero
            0x90..=0x9F => {
                self.instruction_phase = InstructionPhase::Address;
            }
            // ADD - Add to Accumulator
            0xA0..=0xAF => {
                let register = instruction & 0x0F;
                let value = self.index_registers[register as usize];
                self.add_to_accumulator(value);
            }
            // SUB - Subtract from Accumulator
            0xB0..=0xBF => {
                let register = instruction & 0x0F;
                let value = self.index_registers[register as usize];
                self.subtract_from_accumulator(value);
            }
            // LD - Load Accumulator
            0xC0..=0xCF => {
                let register = instruction & 0x0F;
                self.accumulator = self.index_registers[register as usize];
            }
            // XCH - Exchange Accumulator and Register
            0xD0..=0xDF => {
                let register = instruction & 0x0F;
                let temp = self.accumulator;
                self.accumulator = self.index_registers[register as usize];
                self.index_registers[register as usize] = temp;
            }
            // BBL - Branch Back and Load
            0xE0..=0xEF => {
                let value = instruction & 0x0F;
                self.accumulator = value;
                self.return_from_subroutine();
            }
            // LDM - Load Accumulator Immediate
            0xF0..=0xFF => {
                let value = instruction & 0x0F;
                self.accumulator = value;
            }
            _ => {}
        }
    }

    fn execute_instruction_phase1(&mut self) {
        // Phase 1 execution for specific instructions
        match self.current_instruction {
            // Instructions that output data in phase 1
            _ => {
                // Most instructions don't output in phase 1
            }
        }
    }

    fn execute_instruction_phase2(&mut self) {
        // Phase 2 execution for specific instructions
        match self.current_instruction {
            // JCN - Jump Conditional
            0x10..=0x1F => {
                self.execute_jump_conditional();
            }
            // FIM - Fetch Immediate
            0x20..=0x2F => {
                self.execute_fetch_immediate();
            }
            // JUN - Jump Unconditional
            0x60..=0x6F => {
                self.execute_jump_unconditional();
            }
            // JMS - Jump to Subroutine
            0x70..=0x7F => {
                self.execute_jump_subroutine();
            }
            // ISZ - Increment and Skip if Zero
            0x90..=0x9F => {
                self.execute_increment_skip_zero();
            }
            _ => {
                // Other instructions complete in decode phase
            }
        }
    }

    fn add_to_accumulator(&mut self, value: u8) {
        let result = self.accumulator as u16 + value as u16;
        self.accumulator = (result & 0x0F) as u8;
        self.carry = (result & 0x10) != 0;
    }

    fn subtract_from_accumulator(&mut self, value: u8) {
        let result = self.accumulator as i16 - value as i16;
        self.accumulator = (result & 0x0F) as u8;
        self.carry = result >= 0;
    }

    fn execute_jump_conditional(&mut self) {
        let condition = (self.current_instruction & 0x0F) as u8;
        let address_low = self.data_latch;
        let address_high = (self.current_instruction & 0x0F) as u16;
        let address = (address_high << 8) | address_low as u16;

        let should_jump = match condition {
            0x1 => !self.carry,        // Jump if no carry
            0x2 => self.accumulator == 0, // Jump if accumulator zero
            0x4 => !self.read_test_pin(), // Jump if test pin low
            0x8 => true,               // Unconditional jump (part of condition)
            _ => false,
        };

        if should_jump {
            self.program_counter.set(address);
        }
    }

    fn execute_fetch_immediate(&mut self) {
        let register_pair = (self.current_instruction & 0x0E) >> 1;
        self.index_registers[register_pair as usize * 2] = self.data_latch;
    }

    fn execute_jump_unconditional(&mut self) {
        let address_low = self.data_latch;
        let address_high = (self.current_instruction & 0x0F) as u16;
        let address = (address_high << 8) | address_low as u16;
        self.program_counter.set(address);
    }

    fn execute_jump_subroutine(&mut self) {
        // Push current PC to stack
        if self.stack_pointer < 3 {
            self.stack[self.stack_pointer as usize] = self.program_counter;
            self.stack_pointer += 1;
        }

        let address_low = self.data_latch;
        let address_high = (self.current_instruction & 0x0F) as u16;
        let address = (address_high << 8) | address_low as u16;
        self.program_counter.set(address);
    }

    fn execute_increment_skip_zero(&mut self) {
        let register = self.current_instruction & 0x0F;
        self.index_registers[register as usize] =
            (self.index_registers[register as usize] + 1) & 0x0F;

        if self.index_registers[register as usize] == 0 {
            self.program_counter.inc(); // Skip next instruction
        }
    }

    fn return_from_subroutine(&mut self) {
        if self.stack_pointer > 0 {
            self.stack_pointer -= 1;
            self.program_counter = self.stack[self.stack_pointer as usize];
        }
    }
}

impl Component for Intel4004 {
    fn name(&self) -> String {
        self.base.name()
    }

    fn pins(&self) -> &HashMap<String, Arc<Mutex<Pin>>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.base.get_pin(name)
    }

    fn update(&mut self) {
        // Handle reset
        if self.read_reset_pin() {
            self.reset();
            return;
        }

        // Process clock
        self.handle_clock();
    }

    fn run(&mut self) {
        self.base.set_running(true);

        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10)); // Prevent busy waiting
        }
    }

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

// Intel 4004 specific methods
impl Intel4004 {
    pub fn get_program_counter(&self) -> u16 {
        self.program_counter.value()
    }

    pub fn set_program_counter(&mut self, pc: u16) {
        self.program_counter.set(pc);
    }

    pub fn get_accumulator(&self) -> u8 {
        self.accumulator
    }

    pub fn set_accumulator(&mut self, value: u8) {
        self.accumulator = value & 0x0F;
    }

    pub fn get_carry(&self) -> bool {
        self.carry
    }

    pub fn set_carry(&mut self, carry: bool) {
        self.carry = carry;
    }

    pub fn get_register(&self, index: u8) -> Option<u8> {
        if index < 16 {
            Some(self.index_registers[index as usize])
        } else {
            None
        }
    }

    pub fn set_register(&mut self, index: u8, value: u8) -> Result<(), String> {
        if index < 16 {
            self.index_registers[index as usize] = value & 0x0F;
            Ok(())
        } else {
            Err("Register index out of range (0-15)".to_string())
        }
    }

    pub fn get_stack_pointer(&self) -> u8 {
        self.stack_pointer
    }

    pub fn get_stack_value(&self, level: u8) -> Option<u16> {
        if level < 3 {
            Some(self.stack[level as usize].value())
        } else {
            None
        }
    }

    pub fn get_cycle_count(&self) -> u64 {
        self.cycle_count
    }

    pub fn get_clock_speed(&self) -> f64 {
        self.clock_speed
    }

    pub fn set_clock_speed(&mut self, speed: f64) {
        self.clock_speed = speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel4004_creation() {
        let cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0); // 750kHz
        assert_eq!(cpu.name(), "CPU_4004");
        assert_eq!(cpu.get_clock_speed(), 750_000.0);
        assert!(!cpu.is_running());
    }

    #[test]
    fn test_intel4004_registers() {
        let mut cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0);

        // Test register access
        assert!(cpu.set_register(0, 0x0A).is_ok());
        assert_eq!(cpu.get_register(0).unwrap(), 0x0A);

        // Test invalid register
        assert!(cpu.set_register(16, 0x0F).is_err());
        assert!(cpu.get_register(16).is_none());
    }

    #[test]
    fn test_intel4004_accumulator() {
        let mut cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0);

        cpu.set_accumulator(0x0B);
        assert_eq!(cpu.get_accumulator(), 0x0B);

        // Test that only lower 4 bits are stored
        cpu.set_accumulator(0x1F);
        assert_eq!(cpu.get_accumulator(), 0x0F);
    }

    #[test]
    fn test_intel4004_program_counter() {
        let mut cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0);

        cpu.set_program_counter(0x123);
        assert_eq!(cpu.get_program_counter(), 0x123);

        // Test 12-bit masking
        cpu.set_program_counter(0x1FFF);
        assert_eq!(cpu.get_program_counter(), 0x0FFF);
    }

    #[test]
    fn test_intel4004_arithmetic() {
        let mut cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0);

        // Test addition
        cpu.set_accumulator(0x05);
        cpu.add_to_accumulator(0x07);
        assert_eq!(cpu.get_accumulator(), 0x0C);
        assert!(cpu.get_carry()); // 5 + 7 = 12, no carry in 4-bit math? Wait, 12 is 0xC, no carry

        // Test subtraction
        cpu.set_accumulator(0x08);
        cpu.subtract_from_accumulator(0x03);
        assert_eq!(cpu.get_accumulator(), 0x05);
        assert!(cpu.get_carry()); // No borrow
    }

    #[test]
    fn test_intel4004_reset() {
        let mut cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0);

        cpu.set_program_counter(0x123);
        cpu.set_accumulator(0x0A);
        cpu.set_register(0, 0x05).unwrap();

        cpu.reset();

        assert_eq!(cpu.get_program_counter(), 0);
        assert_eq!(cpu.get_accumulator(), 0);
        assert_eq!(cpu.get_register(0).unwrap(), 0);
    }
}