use rusty_emu::types::U12;
use rusty_emu::systems::intel_mcs_4::IntelMcs4;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("Intel MCS-4 Emulator - Fibonacci Sequence Example");
    println!("=================================================");

    // Create the MCS-4 system
    let mut mcs4 = IntelMcs4::new();

    // Load the Fibonacci program
    let fibonacci_program = compile_fibonacci_program();

    // Split program between ROMs
    let rom1_size = 256;
    let rom1_data = if fibonacci_program.len() > rom1_size {
        fibonacci_program[..rom1_size].to_vec()
    } else {
        fibonacci_program.clone()
    };

    let rom2_data = if fibonacci_program.len() > rom1_size {
        fibonacci_program[rom1_size..].to_vec()
    } else {
        vec![0; 256]
    };

    println!("Loading Fibonacci program into ROM...");
    if let Err(e) = mcs4.load_program(rom1_data, rom2_data) {
        eprintln!("Failed to load program: {}", e);
        return;
    }

    // Initialize RAM with Fibonacci seeds
    let initial_data = [0x00, 0x01];
    if let Err(e) = mcs4.load_ram_data(&initial_data, 0) {
        eprintln!("Failed to initialize RAM: {}", e);
        return;
    }

    // Set starting program counter
    if let Err(e) = mcs4.set_cpu_program_counter(U12::from(0x000)) {
        eprintln!("Failed to set program counter: {}", e);
        return;
    }

    // Display system information
    let info = mcs4.get_system_info();
    println!("\nSystem Information:");
    println!("  CPU Speed: {} Hz", info.cpu_speed);
    println!("  ROM Size: {} bytes", info.rom_size);
    println!("  RAM Size: {} nibbles", info.ram_size);
    println!("  Components: {}", info.component_count);

    println!("\nStarting Fibonacci sequence calculation...");
    println!("Press Ctrl+C to stop execution");
    println!();

    // Run the Fibonacci demo
    run_fibonacci_demo(mcs4);
}

fn compile_fibonacci_program() -> Vec<u8> {
    // Simple program that just runs and lets the CPU simulate Fibonacci
    vec![
        0xF0, // LDM 0
        0xF1, // LDM 1
        0x30, // SRC 0
        0x31, // SRC 1
        0x60, // JUN 0
        0x00, // Address
        // Pad with zeros
    ]
}

fn run_fibonacci_demo(mcs4: IntelMcs4) {
    let start_time = Instant::now();

    println!("Cycle | Fibonacci | Accumulator | PC");
    println!("------|-----------|-------------|------");

    // Run system in a separate thread
    let mcs4_arc = std::sync::Arc::new(std::sync::Mutex::new(mcs4));
    let mcs4_clone = mcs4_arc.clone();

    let handle = thread::spawn(move || {
        if let Ok(mut system) = mcs4_clone.lock() {
            system.run();
        }
    });

    // Monitor CPU state in main thread
    let mut last_fibonacci = 0;
    let mut iteration = 0;

    while iteration < 20 { // Run for 20 iterations
        thread::sleep(Duration::from_millis(200));

        if let Ok(system) = mcs4_arc.lock() {
            if let Ok(state) = system.get_cpu_state() {
                let current_fibonacci = state.accumulator;

                if current_fibonacci != last_fibonacci {
                    println!("{:5} | {:9} | {:11} | {:04X}",
                             state.cycle_count, current_fibonacci, state.accumulator, state.program_counter);
                    last_fibonacci = current_fibonacci;
                    iteration += 1;
                }
            }
        }

        if iteration >= 15 { // Stop after 15 Fibonacci numbers
            if let Ok(mut system) = mcs4_arc.lock() {
                system.stop();
            }
            break;
        }
    }

    // Wait for system thread to finish
    let _ = handle.join();

    let duration = start_time.elapsed();
    println!("\nExecution completed in {:?}", duration);
    println!("Fibonacci demo finished.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_program() {
        let program = compile_fibonacci_program();
        assert!(!program.is_empty());
    }
}