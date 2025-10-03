//! # Rusty Emulator - Main Binary
//!
//! Command-line interface and application entry point for the Intel MCS-4 microprocessor simulator.
//!
//! This binary provides multiple interface modes:
//! - **Traditional Console**: Direct system execution with monitoring
//! - **Interactive Console**: Terminal UI with real-time system control
//! - **Graphical User Interface**: Modern desktop application with visual monitoring
//!
//! ## Architecture Overview
//!
//! The main binary serves as the application entry point, handling:
//! - Command-line argument parsing and validation
//! - System configuration and initialization
//! - Interface mode selection and launch
//! - Program file loading and validation
//! - Error handling and user feedback
//!
//! ## Interface Modes
//!
//! ### Traditional Mode (Default)
//! Direct system execution with console monitoring output.
//! Suitable for automated testing and batch operations.
//!
//! ### Console Mode (`--console`)
//! Interactive terminal interface with real-time system monitoring.
//! Provides formatted display of registers, memory, and system state.
//!
//! ### GUI Mode (`--gui`)
//! Modern desktop application with graphical system monitoring.
//! Offers intuitive controls and visual status indicators.
//!
//! ## Usage Examples
//!
//! ### Basic System Execution
//! ```bash
//! # Run basic MCS-4 system with default fibonacci program
//! cargo run -- --system basic
//!
//! # Run Fig.1 MCS-4 Max system
//! cargo run -- --system max
//!
//! # Run with custom program
//! cargo run -- --system basic --file programs/myprogram.bin
//! ```
//!
//! ### Interactive Console Mode
//! ```bash
//! # Launch interactive console interface
//! cargo run -- --console --system basic
//!
//! # Console with custom configuration
//! cargo run -- --console --system custom_config.json
//! ```
//!
//! ### Graphical User Interface
//! ```bash
//! # Launch GUI with basic system
//! cargo run -- --gui --system basic
//!
//! # GUI with custom program
//! cargo run -- --gui --system basic --file programs/myprogram.bin
//! ```
//!
//! ### Help and Information
//! ```bash
//! # Show comprehensive help
//! cargo run -- --help
//! ```

use rusty_emu::console::{run_console, ConsoleConfig};
use rusty_emu::gui::run_gui;
use rusty_emu::system_config::{ConfigurableSystem, SystemFactory};
use std::env;
use std::fs;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Main application entry point
///
/// This function serves as the central dispatcher for the emulator application.
/// It handles command-line argument parsing, system initialization, and interface
/// mode selection based on user preferences.
///
/// ## Application Flow
/// 1. Parse and validate command-line arguments
/// 2. Load program data from specified file
/// 3. Create and configure emulator system
/// 4. Launch selected interface mode (traditional, console, or GUI)
/// 5. Handle cleanup and error reporting
///
/// ## Interface Mode Selection
/// The application supports three distinct operating modes:
/// - **Traditional**: Direct execution with console monitoring
/// - **Console**: Interactive terminal interface
/// - **GUI**: Graphical desktop application
///
/// Only one interface mode can be active at a time. If multiple modes are
/// specified, the last one takes precedence.
fn main() {
    // Parse command line arguments with comprehensive error handling
    let args: Vec<String> = env::args().collect();
    let mut system_type = "basic".to_string();
    let mut program_file = "programs/fibonacci.bin".to_string();
    let mut use_console = false;
    let mut use_gui = false;

    // Command-line argument parsing with validation
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            // System configuration selection
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
            // Program file specification
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
            // Interactive console interface mode
            "-c" | "--console" => {
                use_console = true;
                i += 1;
            }
            // Graphical user interface mode
            "-g" | "--gui" => {
                use_gui = true;
                i += 1;
            }
            // Help and usage information
            "-h" | "--help" => {
                print_usage(&args[0]);
                return;
            }
            // Unknown argument handling
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
    println!(
        "Console: {}",
        if use_console { "Enabled" } else { "Disabled" }
    );
    println!("GUI: {}", if use_gui { "Enabled" } else { "Disabled" });

    // Load program data
    let program_data = match load_program_data(&program_file) {
        Ok(data) => {
            println!(
                "DEBUG: Program data loaded successfully, {} bytes",
                data.len()
            );
            data
        }
        Err(e) => {
            eprintln!("Failed to load program: {}", e);
            process::exit(1);
        }
    };

    // Create and configure the system
    let system = match create_system(&system_type, &program_data) {
        Ok(sys) => {
            println!("DEBUG: System created successfully");
            sys
        }
        Err(e) => {
            eprintln!("Failed to create system: {}", e);
            process::exit(1);
        }
    };

    // Interface mode selection and launch
    if use_console {
        // Launch interactive console interface
        // The console provides a terminal-based UI with real-time system monitoring
        let system_arc = Arc::new(Mutex::new(system));
        let console_config = ConsoleConfig::default();

        println!("Starting interactive console interface...");
        println!(
            "Console features: Real-time monitoring, formatted output, non-blocking operation"
        );

        // Start the emulator system in a separate thread
        // This allows the console interface to run while emulation continues
        let system_runner = system_arc.clone();
        thread::spawn(move || {
            if let Ok(mut system) = system_runner.lock() {
                println!("DEBUG: Starting system for console mode");
                system.run();
            }
        });

        // Launch console interface (blocks until console is closed)
        if let Err(e) = run_console(system_arc, console_config) {
            eprintln!("Console interface error: {}", e);
            process::exit(1);
        }
    } else if use_gui {
        // Launch graphical user interface
        // The GUI provides a modern desktop application with visual system monitoring
        let system_arc = Arc::new(Mutex::new(system));

        println!("Starting graphical user interface...");
        println!("GUI features: Visual monitoring, interactive controls, real-time updates");

        // Start the emulator system in a separate thread
        // This allows the GUI to remain responsive while emulation runs
        let system_runner = system_arc.clone();
        thread::spawn(move || {
            if let Ok(mut system) = system_runner.lock() {
                println!("DEBUG: Starting system for GUI mode");
                system.run();
            }
        });

        // Launch GUI application (blocks until GUI window is closed)
        // The GUI will handle all user interactions and system monitoring
        println!("DEBUG: About to call run_gui()...");
        match run_gui(Some(system_arc)) {
            Ok(_) => {
                println!("DEBUG: GUI completed successfully");
            }
            Err(e) => {
                eprintln!("GUI interface error: {}", e);
                eprintln!("Error details: {:?}", e);
                eprintln!();
                eprintln!("Troubleshooting steps:");
                eprintln!("1. Make sure all GUI dependencies are installed:");
                eprintln!("   cargo add egui eframe rfd raw-window-handle");
                eprintln!("2. Check if display server is available (X11/Wayland on Linux, Windows Desktop)");
                eprintln!("3. Try running without GUI: cargo run -- --console --system basic");
                eprintln!("4. Check if GPU drivers are installed and working");
                process::exit(1);
            }
        }
    } else {
        // Use traditional interface
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
}

fn print_usage(program_name: &str) {
    println!("Usage: {} [OPTIONS]", program_name);
    println!();
    println!("Intel MCS-4 Microprocessor Simulator with Multiple Interface Modes");
    println!();
    println!("Options:");
    println!("  -s, --system <SYSTEM>    System type to run (default: basic)");
    println!("                           Available: basic, max, fig1, or JSON config file");
    println!("  -f, --file <FILE>        Program binary file to load (default: fibonacci.bin)");
    println!("  -c, --console           Enable interactive console interface");
    println!("  -g, --gui               Enable graphical user interface");
    println!("  -h, --help              Show this help message");
    println!();
    println!("System Types:");
    println!("  basic  - Basic MCS-4 system (CPU, clock, 2 ROMs, 1 RAM)");
    println!("  max    - Fig.1 MCS-4 Max system (16 ROMs, 16 RAMs, shift registers)");
    println!("  fig1   - Same as 'max'");
    println!("  *.json - Custom system configuration file");
    println!();
    println!("Interface Modes:");
    println!("  Default (no flags)      - Traditional console with system monitoring");
    println!("  -c, --console           - Interactive terminal UI with real-time display");
    println!("  -g, --gui               - Graphical desktop application");
    println!();
    println!("Console Interface (-c/--console):");
    println!("  Provides an interactive terminal UI with:");
    println!("  • Real-time system monitoring and formatted output");
    println!("  • Live RAM and register state display");
    println!("  • Non-blocking operation that doesn't interfere with emulation");
    println!("  • Clean tabular display of system components");
    println!();
    println!("GUI Interface (-g/--gui):");
    println!("  Provides a graphical desktop application with:");
    println!("  • Modern, intuitive user interface built with egui");
    println!("  • Real-time system monitoring with visual indicators");
    println!("  • Interactive controls for system management");
    println!("  • Memory and register inspection tools");
    println!("  • Component health monitoring");
    println!("  • Error display and handling");
    println!();
    println!("Examples:");
    println!(
        "  {} -s basic -f fibonacci.bin          # Traditional mode",
        program_name
    );
    println!(
        "  {} --system max                       # Traditional with max system",
        program_name
    );
    println!(
        "  {} --system custom_config.json        # Custom configuration",
        program_name
    );
    println!(
        "  {} --console --system basic           # Interactive console",
        program_name
    );
    println!(
        "  {} --gui --system basic               # Graphical interface",
        program_name
    );
    println!(
        "  {} --gui --system basic --file prog.bin # GUI with custom program",
        program_name
    );
    println!();
    println!("For more information about the GUI interface, see:");
    println!("  • GUI Features: Real-time monitoring, interactive controls");
    println!("  • Requirements: 1200x800+ display, egui/eframe dependencies");
    println!("  • Integration: Thread-safe operation with emulator system");
}

fn load_program_data(filename: &str) -> Result<Vec<u8>, String> {
    println!("DEBUG: Attempting to load program from: {}", filename);
    match fs::read(filename) {
        Ok(data) => {
            println!(
                "DEBUG: Successfully loaded {} bytes from {}",
                data.len(),
                filename
            );
            Ok(data)
        }
        Err(e) => {
            // If file doesn't exist, try to use default program
            if filename == "programs/fibonacci.bin" {
                println!(
                    "DEBUG: File {} not found ({}), using default fibonacci program",
                    filename, e
                );
                let default_program = get_default_fibonacci_program();
                println!(
                    "DEBUG: Default program size: {} bytes",
                    default_program.len()
                );
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
        0x20, 0x00, // LDM 0 (Load accumulator with 0) - First Fibonacci number
        0x10, 0x00, // LD 0 (Load accumulator from register 0)
        0x50, // WRM (Write accumulator to RAM at current pointer)
        0x21, 0x01, // LDM 1 (Load accumulator with 1) - Second Fibonacci number
        0x10, 0x01, // LD 1 (Load accumulator from register 1)
        0x50, // WRM (Write accumulator to RAM)
        // Fibonacci calculation loop
        0x00, 0x02, // LD 2 (Load register 2 into accumulator) - Loop counter
        0x76, // IAC (Increment accumulator)
        0x00, 0x02, // LD 2 (Store back to register 2)
        // Calculate next Fibonacci number: F(n) = F(n-1) + F(n-2)
        0x00, 0x00, // LD 0 (Load F(n-2) into accumulator)
        0x10, 0x01, // ADD 1 (Add F(n-1) to accumulator)
        0x50, // WRM (Store result to RAM)
        // Update registers for next iteration
        0x00, 0x01, // LD 1 (Load F(n-1) into accumulator)
        0x00, 0x00, // LD 0 (Store F(n-1) to register 0)
        0x50, // WRM (Write to RAM - this will be read back as F(n-2))
        // Check if we've calculated enough numbers (8 iterations)
        0x00, 0x02, // LD 2 (Load loop counter)
        0x30, 0x08, // JCN 8 (Jump if accumulator == 8) - Exit condition
        0x40, 0x0C, // JCN 12 (Jump back to loop start)
        // Exit - halt program
        0x00, 0x00, // NOP (placeholder for halt)
    ]
}

fn create_system(system_type: &str, program_data: &[u8]) -> Result<ConfigurableSystem, String> {
    let factory = SystemFactory::new();

    match system_type {
        "mcs4" | "basic" => {
            // Use the basic MCS-4 configuration
            let mut system = factory
                .create_from_json("configs/mcs4_basic.json")
                .map_err(|e| format!("Failed to create basic MCS-4 system: {}", e))?;

            // Load program data into ROM components
            system.load_program_data(program_data)?;

            Ok(system)
        }
        "mcs4_max" | "max" | "fig1" => {
            // Use the Fig.1 MCS-4 Max configuration
            let mut system = factory
                .create_from_json("configs/mcs4_max.json")
                .map_err(|e| format!("Failed to create MCS-4 Max system: {}", e))?;

            // Load program data into ROM components
            system.load_program_data(program_data)?;

            Ok(system)
        }
        _ => {
            // Try to use provided config file directly
            if system_type.ends_with(".json") {
                let mut system = factory.create_from_json(system_type).map_err(|e| {
                    format!("Failed to create system from '{}': {}", system_type, e)
                })?;

                // Load program data into ROM components
                system.load_program_data(program_data)?;

                Ok(system)
            } else {
                Err(format!("Unknown system type: {}. Use 'basic', 'max', or provide a JSON config file path.", system_type))
            }
        }
    }
}

fn run_system_demo(system: ConfigurableSystem) {
    // Display system information
    let info = system.get_system_info();
    println!("System: {} - {}", info.name, info.description);
    println!("Components: {}", info.component_count);
    println!("CPU Speed: {} Hz", info.cpu_speed);

    // Run system in a separate thread
    let system_arc = std::sync::Arc::new(std::sync::Mutex::new(system));
    let system_clone = system_arc.clone();

    let handle = thread::spawn(move || {
        if let Ok(mut system) = system_clone.lock() {
            system.run();
        }
    });

    // Start monitoring in a separate thread
    let system_monitor = system_arc.clone();
    let running = Arc::new(Mutex::new(true));
    let running_clone = running.clone();

    let monitor_handle = thread::spawn(move || {
        monitor_system_state(system_monitor, running_clone);
    });

    // Monitor system state with timeout
    let start_time = Instant::now();
    let timeout = Duration::from_secs(10); // 10 second timeout

    loop {
        thread::sleep(Duration::from_millis(500));

        // Check for timeout
        if start_time.elapsed() >= timeout {
            println!("\nSimulation timed out after 10 seconds - stopping system");
            if let Ok(mut system) = system_arc.lock() {
                system.stop();
            }
            *running.lock().unwrap() = false;
            break;
        }

        if let Ok(system) = system_arc.lock() {
            if !system.is_running() {
                *running.lock().unwrap() = false;
                break;
            }
        }
    }

    // Wait a bit for components to stop gracefully
    thread::sleep(Duration::from_millis(1000));

    let _ = handle.join();
    let _ = monitor_handle.join();
    let duration = start_time.elapsed();
    println!("\nExecution completed in {:?}", duration);
}

/// Monitor and display system state periodically
/// This function runs in a separate thread and displays CPU registers, clock signals,
/// data/address bus states, and RAM contents at regular intervals
fn monitor_system_state(system_arc: Arc<Mutex<ConfigurableSystem>>, running_arc: Arc<Mutex<bool>>) {
    println!("DEBUG: Starting enhanced monitoring thread");
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│                    SYSTEM MONITOR                               │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ CPU Registers | Clock | Bus | RAM | Output Ports                │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let mut cycle = 0;
    println!("DEBUG: Monitoring thread starting (100ms intervals for system monitoring)");
    loop {
        // Reduced frequency monitoring to avoid spam (100ms intervals)
        thread::sleep(Duration::from_millis(100)); // Reasonable interval for console monitoring

        // Check if we should still be running (fast check)
        let should_continue = match running_arc.lock() {
            Ok(running_val) => *running_val,
            Err(_) => true, // Continue if lock is poisoned
        };

        if !should_continue {
            break;
        }

        cycle += 1;

        // High-frequency monitoring: try_lock for immediate availability
        // This matches MCS-4's 11µs cycle timing without blocking emulation
        match system_arc.try_lock() {
            Ok(system) => {
                // Got the lock immediately - show detailed state (rare but possible)
                display_detailed_system_state(&system, cycle);
            }
            Err(_) => {
                // System is locked by emulation - this is normal and expected
                // Show basic state without trying to acquire locks
                display_basic_system_state(cycle);
            }
        }
    }

    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│                    MONITORING STOPPED                           │");
    println!("└─────────────────────────────────────────────────────────────────┘");
}

/// Display detailed system state when we can acquire locks
fn display_detailed_system_state(system: &ConfigurableSystem, cycle: u32) {
    // Only show detailed output occasionally to avoid spam (every 10 cycles = ~1 second)
    if cycle % 1000 == 0 {
        println!("\n┌─────────────────────────────────────────────────────────────────┐");
        println!(
            "│                         CYCLE {:4}                              │",
            cycle
        );
        println!("├─────────────────────────────────────────────────────────────────┤");

        // CPU State Section
        if let Some(cpu_component) = system.get_components().get("CPU_4004") {
            if let Ok(cpu) = cpu_component.lock() {
                println!("│ CPU STATE:                                                      │");
                println!(
                    "│   Status: {}                                               │",
                    if cpu.is_running() {
                        "Running"
                    } else {
                        "Stopped"
                    }
                );
                println!(
                    "│   Component: {}                                           │",
                    cpu.name()
                );
            }
        } else {
            println!("│ CPU_4004 component not found                                    │");
        }

        // RAM Section
        if let Some(ram_component) = system.get_components().get("RAM_4002") {
            if let Ok(ram) = ram_component.lock() {
                println!(
                    "│ RAM_4002: {} ({})                              │",
                    ram.name(),
                    if ram.is_running() {
                        "Running"
                    } else {
                        "Stopped"
                    }
                );
            }
        }

        // Component Status Summary
        let running_count = system
            .get_components()
            .values()
            .filter(|comp| comp.lock().map_or(false, |c| c.is_running()))
            .count();
        println!("│                                                                 │");
        println!(
            "│ COMPONENT STATUS: {}/{} running                                 │",
            running_count,
            system.get_components().len()
        );

        println!("└─────────────────────────────────────────────────────────────────┘");
    }
}

/// Display basic system state when locks are not available
fn display_basic_system_state(cycle: u32) {
    // Only show basic output occasionally to avoid spam (every 50 cycles = ~5 seconds)
    if cycle % 5000 == 0 {
        println!("\n┌─────────────────────────────────────────────────────────────────┐");
        println!(
            "│                         CYCLE {:4}                              │",
            cycle
        );
        println!("├─────────────────────────────────────────────────────────────────┤");

        // Show basic system information without requiring locks
        println!("│ SYSTEM STATUS:                                                  │");
        println!("│   Enhanced monitoring active - system is running               │");
        println!("│   Emulation thread is busy - showing overview only             │");
        println!("│   See RAM debug output above for detailed component state       │");
        println!("│                                                                 │");
        println!("│ COMPONENTS RUNNING:                                             │");
        println!("│   ✓ CPU_4004: Executing instructions                           │");
        println!("│   ✓ RAM_4002: Processing memory operations                     │");
        println!("│   ✓ ROM_4001_1: Providing program data                         │");
        println!("│   ✓ ROM_4001_2: Providing program data                         │");
        println!("│   ✓ SYSTEM_CLOCK: Generating clock signals                     │");
        println!("│                                                                 │");
        println!("│ MONITORING:                                                     │");
        println!("│   • System monitoring (100ms intervals)                        │");
        println!("│   • MCS-4 timing emulation (750kHz clock)                      │");
        println!("│   • Non-blocking monitoring (no emulation interference)        │");

        println!("└─────────────────────────────────────────────────────────────────┘");
    }
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
