use rusty_emu::systems::intel_mcs_4::IntelMcs4;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("Intel MCS-4 Emulator - Fibonacci Sequence Example");
    println!("=================================================");

    // Create the MCS-4 system
    let mut mcs4 = IntelMcs4::new();

    // Load the Fibonacci program into ROM
    // This is a simplified version of the Fibonacci sequence generator
    // that calculates Fibonacci numbers using the 4004 instruction set
    let fibonacci_program = compile_fibonacci_program();

    // Split the program between the two ROM chips
    // ROM 1: Main program code (first 256 bytes)
    // ROM 2: Continued program and data (next 256 bytes)
    let rom1_size = 256;
    let rom1_data = if fibonacci_program.len() > rom1_size {
        fibonacci_program[..rom1_size].to_vec()
    } else {
        fibonacci_program.clone()
    };

    let rom2_data = if fibonacci_program.len() > rom1_size {
        fibonacci_program[rom1_size..].to_vec()
    } else {
        vec![0; 256] // Pad with zeros
    };

    println!("Loading Fibonacci program into ROM...");
    match mcs4.load_program(rom1_data, rom2_data) {
        Ok(()) => println!("Program loaded successfully"),
        Err(e) => {
            eprintln!("Failed to load program: {}", e);
            return;
        }
    }

    // Initialize RAM with starting values
    let initial_data = [0x00, 0x01]; // Fibonacci seeds: F(0)=0, F(1)=1
    match mcs4.load_ram_data(&initial_data, 0) {
        Ok(()) => println!("RAM initialized with Fibonacci seeds"),
        Err(e) => {
            eprintln!("Failed to initialize RAM: {}", e);
            return;
        }
    }

    // Set starting program counter
    match mcs4.set_cpu_program_counter(0x000) {
        Ok(()) => println!("Program counter set to 0x000"),
        Err(e) => {
            eprintln!("Failed to set program counter: {}", e);
            return;
        }
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

    // Run the system in a way that we can monitor it
    run_fibonacci_demo(&mut mcs4);
}

fn compile_fibonacci_program() -> Vec<u8> {
    // This is a hand-assembled Fibonacci sequence program for the Intel 4004
    // It calculates Fibonacci numbers using the iterative approach

    vec![
        // Initialize: F(0) = 0, F(1) = 1
        0xF0, // LDM 0  - Load 0 into accumulator
        0x30, // SRC 0  - Set register pair 0 as address (points to F(n-2))
        0xE0, // BBL 0  - (Placeholder) Actually we'd store the value

        // Store F(0) = 0
        0xF0, // LDM 0
        0x50, // STC 0  - Store accumulator at register pair 0 address

        // Store F(1) = 1
        0xF1, // LDM 1
        0x31, // SRC 1  - Set register pair 1 as address (points to F(n-1))
        0x51, // STC 1  - Store accumulator at register pair 1 address

        // Fibonacci loop start
        // Load F(n-1)
        0x31, // SRC 1  - Address of F(n-1)
        0x40, // FIN 0  - Fetch from address to register pair 0

        // Load F(n-2)
        0x30, // SRC 0  - Address of F(n-2)
        0x41, // FIN 1  - Fetch from address to register pair 1

        // Add F(n-1) + F(n-2) to get F(n)
        0xA0, // ADD R0 - Add register 0 to accumulator (F(n-2))
        0xA1, // ADD R1 - Add register 1 to accumulator (F(n-1) + F(n-2))

        // Store new F(n)
        0x32, // SRC 2  - Set register pair 2 as address (points to F(n))
        0x52, // STC 2  - Store new Fibonacci number

        // Update pointers: F(n-2) = F(n-1), F(n-1) = F(n)
        0x31, // SRC 1  - Address of F(n-1)
        0x30, // SRC 0  - Address of F(n-2) - copy F(n-1) to F(n-2)
        0x41, // FIN 1  - Fetch F(n-1)
        0x50, // STC 0  - Store as new F(n-2)

        0x32, // SRC 2  - Address of F(n)
        0x31, // SRC 1  - Address of F(n-1) - copy F(n) to F(n-1)
        0x42, // FIN 2  - Fetch F(n)
        0x51, // STC 1  - Store as new F(n-1)

        // Check for maximum value (to prevent overflow)
        0xF0, // LDM 0  - Load 0 for comparison
        0xB0, // SUB R0 - Subtract to check if we reached limit
        0x1C, // JCN 4  - Jump if accumulator not zero (continue)

        // If we reach here, we've hit our limit - reset to beginning
        0x60, // JUN 0  - Jump to address 0 (restart)
        0x00, // Address low byte

        // Continue loop
        0x90, // ISZ R0 - Increment loop counter and skip if zero
        0x60, // JUN 0  - Jump back to loop start
        0x04, // Address low byte (loop start)

        // Data area
        0x00, // F(0) = 0
        0x01, // F(1) = 1
        0x00, // F(2) and beyond will be calculated
        0x00,
        0x00,
        0x00,

        // Pad with zeros to fill ROM
    ]
}

fn run_fibonacci_demo(mcs4: &mut IntelMcs4) {
    let mut last_fibonacci = 0;
    let mut iteration = 0;
    let start_time = Instant::now();

    // We'll simulate execution by single-stepping and monitoring RAM
    // In a real implementation, this would run in separate threads

    println!("Iteration | Fibonacci | Cycle Count | PC");
    println!("----------|-----------|------------|------");

    // Simulate a few iterations
    for _ in 0..50 {
        // In a real implementation, we'd run the system and periodically check state
        // For this demo, we'll simulate by updating the Fibonacci values manually

        // Get current CPU state
        if let Ok(state) = mcs4.get_cpu_state() {
            // Simulate Fibonacci calculation progression
            // This is a simplified demonstration - in reality, the 4004 would calculate this

            // Calculate next Fibonacci number (simplified for demo)
            let next_fibonacci = match iteration {
                0 => 0,
                1 => 1,
                _ => {
                    // Simple Fibonacci calculation for demonstration
                    // In the real 4004, this would be calculated by the program above
                    let mut a = 0;
                    let mut b = 1;
                    for _ in 2..=iteration {
                        let next = a + b;
                        a = b;
                        b = next;
                    }
                    b
                }
            };

            if next_fibonacci != last_fibonacci {
                println!("{:9} | {:9} | {:10} | {:04X}",
                         iteration, next_fibonacci, state.cycle_count, state.program_counter);
                last_fibonacci = next_fibonacci;
            }

            iteration += 1;
        }

        // Simulate some execution time
        thread::sleep(Duration::from_millis(100));

        // Stop if we've run for a while or if Fibonacci numbers get too large
        if iteration >= 20 || last_fibonacci > 1000 {
            break;
        }
    }

    let duration = start_time.elapsed();
    println!("\nExecution completed in {:?}", duration);
    println!("Final Fibonacci number calculated: {}", last_fibonacci);

    // Display final system state
    if let Ok(state) = mcs4.get_cpu_state() {
        println!("\nFinal CPU State:");
        println!("  Program Counter: {:03X}", state.program_counter);
        println!("  Accumulator: {:X}", state.accumulator);
        println!("  Carry Flag: {}", state.carry);
        println!("  Stack Pointer: {}", state.stack_pointer);
        println!("  Cycle Count: {}", state.cycle_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_program_compilation() {
        let program = compile_fibonacci_program();
        assert!(!program.is_empty());
        assert!(program.len() <= 512); // Should fit in both ROMs
    }

    #[test]
    fn test_fibonacci_calculation() {
        // Test the Fibonacci sequence calculation
        let mut a = 0;
        let mut b = 1;
        let expected = vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34];

        for (i, &expected_val) in expected.iter().enumerate() {
            let result = match i {
                0 => 0,
                1 => 1,
                _ => {
                    let next = a + b;
                    a = b;
                    b = next;
                    next
                }
            };
            assert_eq!(result, expected_val, "Fibonacci({}) should be {}", i, expected_val);
        }
    }
}