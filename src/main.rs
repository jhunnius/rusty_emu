use rusty_emu::systems::intel_mcs_4::IntelMcs4;
use std::env;
use std::fs;
use std::process;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut system_type = "mcs4".to_string();
    let mut program_file = "fibonacci.bin".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-s" | "--system" => {
                if i + 1 < args.len() {
                    system_type = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --system requires a value");
                    print_usage(&args[0]);
                    process::exit(1);
                }
            }
            "-f" | "--file" => {
                if i + 1 < args.len() {
                    program_file = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --file requires a value");
                    print_usage(&args[0]);
                    process::exit(1);
                }
            }
            "-h" | "--help" => {
                print_usage(&args[0]);
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage(&args[0]);
                process::exit(1);
            }
        }
    }

    println!("Intel MCS-4 Emulator");
    println!("===================");
    println!("System: {}", system_type);
    println!("Program: {}", program_file);

    // Load program data
    let program_data = match load_program_data(&program_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to load program: {}", e);
            process::exit(1);
        }
    };

    // Create and configure the system
    let system = match create_system(&system_type, &program_data) {
        Ok(sys) => sys,
        Err(e) => {
            eprintln!("Failed to create system: {}", e);
            process::exit(1);
        }
    };

    // Display system information
    let info = system.get_system_info();
    println!("\nSystem Information:");
    println!("  CPU Speed: {} Hz", info.cpu_speed);
    println!("  ROM Size: {} bytes", info.rom_size);
    println!("  RAM Size: {} nibbles", info.ram_size);
    println!("  Components: {}", info.component_count);

    println!("\nStarting execution...");
    println!("Press Ctrl+C to stop execution");
    println!();

    // Run the system
    run_system_demo(system);
}

fn print_usage(program_name: &str) {
    println!("Usage: {} [OPTIONS]", program_name);
    println!();
    println!("Options:");
    println!("  -s, --system <SYSTEM>    System type to run (default: mcs4)");
    println!("  -f, --file <FILE>        Program binary file to load (default: fibonacci.bin)");
    println!("  -h, --help              Show this help message");
    println!();
    println!("Examples:");
    println!("  {} -s mcs4 -f fibonacci.bin", program_name);
    println!("  {} --system mcs4 --file myprogram.bin", program_name);
}

fn load_program_data(filename: &str) -> Result<Vec<u8>, String> {
    println!("DEBUG: Attempting to load program from: {}", filename);
    match fs::read(filename) {
        Ok(data) => {
            println!("DEBUG: Successfully loaded {} bytes from {}", data.len(), filename);
            Ok(data)
        }
        Err(e) => {
            // If file doesn't exist, try to use default program
            if filename == "fibonacci.bin" {
                println!("DEBUG: File {} not found ({}), using default fibonacci program", filename, e);
                let default_program = get_default_fibonacci_program();
                println!("DEBUG: Default program size: {} bytes", default_program.len());
                Ok(default_program)
            } else {
                println!("DEBUG: Failed to read file {}: {}", filename, e);
                Err(format!("Failed to read file {}: {}", filename, e))
            }
        }
    }
}

fn get_default_fibonacci_program() -> Vec<u8> {
    // Fibonacci program for Intel 4004
    // This implements a simple Fibonacci sequence generator
    vec![
        0x20, 0x00,  // LDM 0 (Load accumulator with 0) - First Fibonacci number
        0x10, 0x00,  // LD 0 (Load accumulator from register 0)
        0x50,        // WRM (Write accumulator to RAM at current pointer)
        0x21, 0x01,  // LDM 1 (Load accumulator with 1) - Second Fibonacci number
        0x10, 0x01,  // LD 1 (Load accumulator from register 1)
        0x50,        // WRM (Write accumulator to RAM)

        // Fibonacci calculation loop
        0x00, 0x02,  // LD 2 (Load register 2 into accumulator) - Loop counter
        0x76,        // IAC (Increment accumulator)
        0x00, 0x02,  // LD 2 (Store back to register 2)

        // Calculate next Fibonacci number: F(n) = F(n-1) + F(n-2)
        0x00, 0x00,  // LD 0 (Load F(n-2) into accumulator)
        0x10, 0x01,  // ADD 1 (Add F(n-1) to accumulator)
        0x50,        // WRM (Store result to RAM)

        // Update registers for next iteration
        0x00, 0x01,  // LD 1 (Load F(n-1) into accumulator)
        0x00, 0x00,  // LD 0 (Store F(n-1) to register 0)
        0x50,        // WRM (Write to RAM - this will be read back as F(n-2))

        // Check if we've calculated enough numbers (8 iterations)
        0x00, 0x02,  // LD 2 (Load loop counter)
        0x30, 0x08,  // JCN 8 (Jump if accumulator == 8) - Exit condition
        0x40, 0x0C,  // JCN 12 (Jump back to loop start)

        // Exit - halt program
        0x00, 0x00,  // NOP (placeholder for halt)
    ]
}

fn create_system(system_type: &str, program_data: &[u8]) -> Result<IntelMcs4, String> {
    match system_type {
        "mcs4" => {
            let system = IntelMcs4::new();

            // Split program between ROMs
            let rom1_size = 256;
            let rom1_data = if program_data.len() > rom1_size {
                program_data[..rom1_size].to_vec()
            } else {
                program_data.to_vec()
            };

            let rom2_data = if program_data.len() > rom1_size {
                program_data[rom1_size..].to_vec()
            } else {
                vec![0; 256]
            };

            // Load program into ROMs
            let mut system = system;
            system.load_program(rom1_data, rom2_data)
                .map_err(|e| format!("Failed to load program: {}", e))?;

            // Initialize RAM with some data for the program
            let initial_data = [0x00, 0x01];
            system.load_ram_data(&initial_data, 0)
                .map_err(|e| format!("Failed to initialize RAM: {}", e))?;

            system.reset_system();
            Ok(system)
        }
        _ => Err(format!("Unknown system type: {}", system_type))
    }
}

fn run_system_demo(system: IntelMcs4) {
    let start_time = Instant::now();

    println!("Cycle | Accumulator | PC");
    println!("------|-------------|------");

    // Run system in a separate thread
    let system_arc = std::sync::Arc::new(std::sync::Mutex::new(system));
    let system_clone = system_arc.clone();

    let handle = thread::spawn(move || {
        if let Ok(mut system) = system_clone.lock() {
            system.run();
        }
    });

    // Monitor CPU state in main thread
    let mut iteration = 0;

    while iteration < 20 {
        thread::sleep(Duration::from_millis(200));

        if let Ok(system) = system_arc.lock() {
            if let Ok(state) = system.get_cpu_state() {
                println!(
                    "{:5} | {:11} | {:04X}",
                    state.cycle_count,
                    state.accumulator,
                    state.program_counter
                );
                iteration += 1;
            }
        }

        if iteration >= 15 {
            if let Ok(mut system) = system_arc.lock() {
                system.stop();
            }
            break;
        }
    }

    let _ = handle.join();
    let duration = start_time.elapsed();
    println!("\nExecution completed in {:?}", duration);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_program() {
        let program = get_default_fibonacci_program();
        assert!(!program.is_empty());
        println!("Fibonacci program size: {} bytes", program.len());
    }
}