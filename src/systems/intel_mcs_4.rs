use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::components::clock::generic_clock::GenericClock;
use crate::components::cpu::intel_4004::Intel4004;
use crate::components::memory::intel_4001::Intel4001;
use crate::components::memory::intel_4002::Intel4002;
use crate::{connect_pins, Component, PinValue};
use crate::bus::GenericBus;
#[derive(Clone, Copy, PartialEq)]
pub enum MCS4Phase {
    AddressLow,    // Send address low nibble
    AddressHigh,   // Send address high nibble + middle
    Instruction,   // Fetch instruction
    DataRead,      // Read data
    DataWrite,     // Write data
}
pub fn run_fibonacci_calculator() {
    println!("Starting MCS-4 Fibonacci Calculator...");

    // Create components
    let mut clock = GenericClock::new("CLK".parse().unwrap(), 1000);
    let mut bus = GenericBus::new("BUS_MCS-4".parse().unwrap(), &[("B0", PinValue::HighZ),("B1", PinValue::HighZ),("B2", PinValue::HighZ),("B3", PinValue::HighZ)]);
    let cpu = Arc::new(Mutex::new(Intel4004::new("cpu4004".to_string())));

    // ROM 0: responds to addresses with high nibble 0x0
    let rom0 = Arc::new(Mutex::new(Intel4001::new("rom0".to_string(), 0x0)));

    // ROM 1: responds to addresses with high nibble 0x1
    let rom1 = Arc::new(Mutex::new(Intel4001::new("rom1".to_string(), 0x1)));

    // RAM bank 0: responds to addresses 0x400-0x47F
    let ram0 = Arc::new(Mutex::new(Intel4002::new("ram0".to_string(), 0x400, 3, 2)));

    // Connect all devices to the bus
    bus.connect_device(cpu.clone());
    bus.connect_device(rom0.clone());
    bus.connect_device(rom1.clone());
    bus.connect_device(ram0.clone());

    // Connect clock to CPU
    if let (Some(clock_out), Some(cpu_clk)) = (clock.out(), cpu.clk()) {
        connect_pins(&clock_out, &cpu_clk).expect("Clock connection failed");
        println!("Connected clock to CPU");
    }

    // Start components
    clock.run();
    cpu.run();

    load_fibonacci_program(rom0.clone());

    // Let it run for a while
    thread::sleep(Duration::from_secs(10));

    // Read results from RAM
    let bus_lock = bus.lock().unwrap();
    let result1 = bus_lock.read(0x0200); // Fibonacci numbers stored here
    let result2 = bus_lock.read(0x0201);
    println!("Fibonacci results: {}, {}", result1, result2);

    // Stop components
    clock.stop();
    cpu.stop();

    println!("Fibonacci calculation completed");
}

fn load_fibonacci_program(rom: Arc<Mutex<Intel4001>>) {
    let mut rom_lock = rom.lock().unwrap();

    // Simple Fibonacci program in 4004 machine code
    let program = vec![
        0x20, 0x00,       // FIM R0,R1 = 0,0 (initialize)
        0x20, 0x11,       // FIM R2,R3 = 1,1 (Fibonacci seed)
        0x20, 0x00,       // FIM R4,R5 = 0,0 (counter)

        // Main loop
        0xA0,             // LD R0 (load first number)
        0xA2,             // LD R2 (load second number)
        0xF0,             // ADD R0 (R0 + R2)
        0xE0,             // WRM (store result)
        0xB0,             // XCH R2 (move R2 to R0)
        0xA0,             // LD R0 (get result)
        0xB2,             // XCH R2 (update second number)

        // Increment counter
        0x64,             // INC R4
        0x10, 0x01, 0x00, // JCN (loop condition)

        // Store results
        0xA0, 0xE0,       // LD R0, WRM (store final result 1)
        0xA2, 0xE1,       // LD R2, WRM (store final result 2)
        0x00,             // NOP
    ];

    // Load program into ROM
    let mut rom_data = Vec::new();
    for &byte in &program {
        rom_data.push(byte as u64);
    }
    rom_lock.load_data(&rom_data);

    println!("Loaded Fibonacci program into ROM");
}