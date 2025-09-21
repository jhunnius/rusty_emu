use std::thread;
use std::time::Duration;
use crate::Component;
use crate::components::clock::generic_clock::GenericClock;
use crate::components::cpu::intel_4004::Intel4004;

fn emulation() {
    println!("Starting MCS-4 emulation with pin state resolution...");

    // Create components
    let mut clock = GenericClock::new("sys_clock".to_string(), 2); // 2Hz clock
    let mut cpu = Intel4004::new("cpu4004".to_string());

    // Connect clock to CPU using the connection macro
    if let (Some(clock_out), Some(cpu_clk)) = (clock.out(), cpu.clk()) {
        connect_pin!(&clock_out, &cpu_clk);
        println!("Connected clock to CPU");
    }

    // Start components
    clock.run();
    cpu.run();

    // Let it run for a while
    thread::sleep(Duration::from_secs(5));

    // Stop components
    clock.stop();
    cpu.stop();

    println!("Emulation completed");
}