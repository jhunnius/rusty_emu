use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::bus::GenericBus;
use crate::component::Component;
use crate::pin::{Pin, PinValue};

// Simple 4-bit and 12-bit types for the 4004
#[derive(Debug, Clone, Copy)]
struct U4(u8);
#[derive(Debug, Clone, Copy)]
struct U12(u16);
impl U4 {
    fn wrapping_add(self, other: U4) -> U4 {
        U4((self.0 + other.0) & 0xF)
    }
}
impl U12 {
    fn wrapping_add(self, other: U12) -> U12 {
        U12((self.0 + other.0) & 0xFFF)
    }
}
#[derive(Clone)]
pub struct Intel4004 {
    base: crate::component::BaseComponent,
    program_counter: U12,
    accumulator: U4,
    carry: bool,
    last_clock_state: PinValue,
    registers: [U4; 16], // 4004 has 16 4-bit registers
    stack: [U12; 3],     // 3-level subroutine stack
    stack_pointer: u8,
    bus: Option<Arc<Mutex<GenericBus>>>, // Connected bus
}
impl Intel4004 {
    pub fn new(name: String) -> Self {
        let mut base = crate::component::BaseComponent::new(name);

        base.add_pin("clk".to_string(), PinValue::Low, false);
        base.add_pin("reset".to_string(), PinValue::Low, false);
        base.add_pin("sync".to_string(), PinValue::Low, true);
        base.add_pin("data".to_string(), PinValue::HighZ, false);

        Self {
            base,
            program_counter: U12(0),
            accumulator: U4(0),
            carry: false,
            last_clock_state: PinValue::Low,
            registers: [U4(0); 16],
            stack: [U12(0); 3],
            stack_pointer: 0,
            bus: None,
        }
    }

    pub fn connect_to_bus(&mut self, bus: Arc<Mutex<GenericBus>>) {
        self.bus = Some(bus);
    }

    fn read_memory(&self, address: u16) -> u8 {
        if let Some(bus) = &self.bus {
            let bus = bus.lock().unwrap();
            bus.read(address as u64) as u8
        } else {
            0xFF
        }
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        if let Some(bus) = &self.bus {
            let mut bus = bus.lock().unwrap();
            bus.write(address as u64, value as u64);
        }
    }

    fn fetch_instruction(&mut self) -> u8 {
        let instruction = self.read_memory(self.program_counter.0);
        self.program_counter = self.program_counter.wrapping_add(U12(1));
        instruction
    }

    fn execute(&mut self, instruction: u8) {
        match instruction {
            // NOP
            0x00 => self.nop(),

            // JCN - Jump Conditional
            0x10 => self.jcn(),

            // FIM - Fetch Immediate
            0x20 => self.fim(),

            // SRC - Send Register Control
            //0x21 => self.src(),

            // FIN - Fetch Indirect
            //0x30 => self.fin(),

            // JIN - Jump Indirect
            //0x31 => self.jin(),

            // LDM - Load Immediate
            0xD0 => self.ldm(),

            // INC - Increment Register
            0x60 => self.inc(),

            // ADD - Add
            0xF0 => self.add(),

            // SUB - Subtract
            //0xF1 => self.sub(),

            // LD - Load from Register
            //0xA0 => self.ld(),

            // XCH - Exchange
            //0xB0 => self.xch(),

            // BBL - Branch Back and Load
            //0xC0 => self.bbl(),

            // WRM - Write Main Memory
            0xE0 => self.wrm(),

            // RDM - Read Main Memory
            0xE1 => self.rdm(),

            _ => println!("Unknown instruction: {:02X}", instruction),
        }
    }

    // Instruction implementations
    fn nop(&mut self) {
        // No operation
    }

    fn jcn(&mut self) {
        let condition = self.fetch_instruction();
        let address_low = self.fetch_instruction();
        let address_high = self.fetch_instruction();
        let address = ((address_high as u16) << 8) | address_low as u16;

        let mut jump = false;

        // Check conditions
        if condition & 0x01 != 0 && self.carry { jump = true; }
        if condition & 0x02 != 0 && self.accumulator.0 == 0 { jump = true; }
        if condition & 0x04 != 0 && self.test_pin() { jump = true; }
        if condition & 0x08 != 0 { jump = !jump; }

        if jump {
            self.program_counter = U12(address);
        }
    }

    fn fim(&mut self) {
        let register_pair = (self.fetch_instruction() & 0x0E) as usize; // Even registers only
        let data = self.fetch_instruction();

        self.registers[register_pair] = U4(data >> 4);
        self.registers[register_pair + 1] = U4(data & 0x0F);
    }

    fn ldm(&mut self) {
        let data = self.fetch_instruction();
        self.accumulator = U4(data & 0x0F);
    }

    fn inc(&mut self) {
        let register = (self.fetch_instruction() & 0x0F) as usize;
        let new_value = self.registers[register].0.wrapping_add(1) & 0x0F;
        self.registers[register] = U4(new_value);
    }

    fn add(&mut self) {
        let register = (self.fetch_instruction() & 0x0F) as usize;
        let result = self.accumulator.0 + self.registers[register].0;
        self.accumulator = U4(result & 0x0F);
        self.carry = result > 0x0F;
    }

    fn wrm(&mut self) {
        // Write accumulator to main memory
        let address = self.get_memory_address();
        self.write_memory(address, self.accumulator.0);
    }

    fn rdm(&mut self) {
        // Read from main memory to accumulator
        let address = self.get_memory_address();
        let data = self.read_memory(address);
        self.accumulator = U4(data & 0x0F);
    }

    fn get_memory_address(&self) -> u16 {
        // Simplified address calculation
        // In real 4004, this uses register pairs
        0x200 + self.registers[0].0 as u16 // Example: use register 0 as offset
    }

    fn test_pin(&self) -> bool {
        // Check test pin (simplified)
        false
    }

    pub fn get_state(&self) -> String {
        format!(
            "PC: {:03X} ACC: {:X} C: {} R0: {:X} R1: {:X}",
            self.program_counter.0,
            self.accumulator.0,
            self.carry as u8,
            self.registers[0].0,
            self.registers[1].0
        )
    }
    pub fn clk(&self) -> Option<Arc<Pin>> {
        self.base.get_pin("clk")
    }

    pub fn reset(&self) -> Option<Arc<Pin>> {
        self.base.get_pin("reset")
    }

    pub fn data(&self) -> Option<Arc<Pin>> {
        self.base.get_pin("data")
    }
}
impl Component for Intel4004 {
    fn name(&self) -> &str {
        self.base.name()
    }
    fn pins(&self) -> &HashMap<String, Arc<Pin>> {
        self.base.pins()
    }
    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.base.get_pin(name)
    }
    fn update(&mut self) -> Result<(), String> {
        let clk_pin = self.base.pins.get("clk").unwrap();
        let clk_value = clk_pin.read();

        // Detect rising edge
        if clk_value == PinValue::High && self.last_clock_state == PinValue::Low {
            // Fetch and execute instruction
            let instruction = self.fetch_instruction();
            self.execute(instruction);

            // Update sync pin
            let sync_pin = self.base.pins.get("sync").unwrap();
            sync_pin.write(PinValue::High, true);

            println!("{}", self.get_state());
        } else if clk_value == PinValue::Low {
            let sync_pin = self.base.pins.get("sync").unwrap();
            sync_pin.write(PinValue::Low, true);
        }

        self.last_clock_state = clk_value;
        Ok(())
    }
    fn run(&mut self) {
        self.base.run();
    }
    fn stop(&mut self) {
        self.base.stop();
    }
}