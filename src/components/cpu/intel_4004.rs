use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};
use crate::types::U12;

#[derive(Debug, Clone, Copy, PartialEq)]
enum InstructionPhase {
    Fetch,
    Address,
    Execute,
    Wait,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ClockPhase {
    Phase1,
    Phase2,
}

pub struct Intel4004 {
    base: BaseComponent,
    accumulator: u8,
    carry: bool,
    index_registers: [u8; 16],
    pub(crate) program_counter: U12,
    stack: [U12; 3],
    stack_pointer: u8,
    cycle_count: u64,
    instruction_phase: InstructionPhase,
    current_instruction: u8,
    address_latch: u8,
    data_latch: u8,
    clock_speed: f64,
    last_clock_transition: Instant,
    clock_phase: ClockPhase,
    rom_port: u8,
    ram_bank: u8,
}

impl Intel4004 {
    pub fn new(name: String, clock_speed: f64) -> Self {
        let pin_names = vec![
            "D0", "D1", "D2", "D3",
            "SYNC", "CM_ROM", "CM_RAM", "TEST", "RESET", "PHI1", "PHI2",
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

    pub fn with_initial_pc(mut self, pc: u16) -> Self {
        self.program_counter = U12::new(pc);
        self
    }

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

    fn handle_clock(&mut self) {
        // Simple clock simulation for now
        self.cycle_count += 1;
    }

    // Simplified instruction execution for Fibonacci demo
    fn execute_instruction(&mut self) {
        // For Fibonacci demo, simulate simple execution
        // Just increment PC and occasionally update accumulator to simulate Fibonacci sequence
        self.program_counter.inc();

        // Every 10 cycles, "calculate" next Fibonacci number
        if self.cycle_count % 10 == 0 {
            // Simple Fibonacci simulation
            let fib_index = (self.cycle_count / 10) as u8;
            self.accumulator = self.simulate_fibonacci(fib_index);
        }
    }

    fn simulate_fibonacci(&self, n: u8) -> u8 {
        match n {
            0 => 0,
            1 => 1,
            _ => {
                let mut a = 0;
                let mut b = 1;
                for _ in 2..=n {
                    let next = a + b;
                    a = b;
                    b = next;
                }
                b % 16 // Keep it in 4-bit range for demo
            }
        }
    }

    // Public methods used by MCS-4 system
    pub fn get_program_counter(&self) -> u16 {
        self.program_counter.value()
    }

    pub fn set_program_counter(&mut self, address: u16) {
        self.program_counter.set(address);
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

    pub fn get_stack_pointer(&self) -> u8 {
        self.stack_pointer
    }

    pub fn get_cycle_count(&self) -> u64 {
        self.cycle_count
    }

    pub fn get_clock_speed(&self) -> f64 {
        self.clock_speed
    }

    pub fn set_register(&mut self, index: u8, value: u8) -> Result<(), String> {
        if index < 16 {
            self.index_registers[index as usize] = value & 0x0F;
            Ok(())
        } else {
            Err("Register index out of range".to_string())
        }
    }

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

    fn update(&mut self) {
        if !self.is_running() {
            return;
        }

        self.handle_clock();
        self.execute_instruction();
    }

    fn run(&mut self) {
        self.base.set_running(true);
        self.reset();

        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10));
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