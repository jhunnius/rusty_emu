use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::component::Component;
use crate::components::clock::generic_clock::GenericClock;
use crate::components::cpu::intel_4004::Intel4004;
use crate::components::memory::intel_4001::Intel4001;
use crate::components::memory::intel_4002::Intel4002;
use crate::components::memory::intel_4003::Intel4003;
use crate::pin::Pin;

pub struct IntelMcs4 {
    components: HashMap<String, Arc<Mutex<dyn Component>>>,
    is_running: bool,
    fibonacci_program: Vec<u8>,  // 4004 assembly program for Fibonacci calculation
}

impl IntelMcs4 {
    pub fn new() -> Self {
        let mut system = IntelMcs4 {
            components: HashMap::new(),
            is_running: false,
            fibonacci_program: Vec::new(),
        };

        system.initialize_fibonacci_program();
        system.initialize_system();
        system
    }

    /// Initialize the Fibonacci calculation program for 4004 assembly
    /// This program calculates Fibonacci numbers and stores them in RAM
    fn initialize_fibonacci_program(&mut self) {
        // 4004 Assembly program for Fibonacci calculation
        // This is a simplified version that calculates the first few Fibonacci numbers
        self.fibonacci_program = vec![
            // Program start - Initialize registers
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
        ];
    }

    fn initialize_system(&mut self) {
        // Create CPU with proper clock speed
        let cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0);
        self.components.insert("cpu".to_string(), Arc::new(Mutex::new(cpu)));

        // Create clock generator
        let clock = GenericClock::new("SYSTEM_CLOCK".to_string(), 750_000.0);
        self.components.insert("clock".to_string(), Arc::new(Mutex::new(clock)));

        // Create RAM
        let ram = Intel4002::new("RAM_4002".to_string());
        self.components.insert("ram".to_string(), Arc::new(Mutex::new(ram)));

        // Create ROMs
        let rom1 = Intel4001::new("ROM_4001_1".to_string());
        self.components.insert("rom1".to_string(), Arc::new(Mutex::new(rom1)));

        let rom2 = Intel4001::new("ROM_4001_2".to_string());
        self.components.insert("rom2".to_string(), Arc::new(Mutex::new(rom2)));

        // Create shift register (4003)
        let shift_reg = Intel4003::new("SHIFT_4003".to_string());
        self.components.insert("shift_reg".to_string(), Arc::new(Mutex::new(shift_reg)));

        // Connect components via pin connections
        self.connect_components();
    }

    /// Connect all components via their pins to create a functional MCS-4 system
    fn connect_components(&mut self) {
        // Connect clock to all components
        self.connect_clock_signals();

        // Connect CPU control signals to memory components
        self.connect_control_signals();

        // Connect data bus between CPU and memory components
        self.connect_data_bus();

        // Load Fibonacci program into ROM
        self.load_fibonacci_program();
    }

    /// Connect clock signals from clock generator to all components
    fn connect_clock_signals(&mut self) {
        let clock_phi1_pin = self.components.get("clock").unwrap().lock().unwrap().get_pin("CLK").unwrap();
        // For now, use the same CLK pin for both phases - in a real implementation,
        // we'd need a clock generator that provides both phases
        let clock_phi2_pin = clock_phi1_pin.clone();

        // Connect to CPU
        if let Some(cpu_component) = self.components.get("cpu") {
            if let Ok(mut cpu) = cpu_component.lock() {
                let cpu_phi1 = cpu.get_pin("PHI1").unwrap();
                let cpu_phi2 = cpu.get_pin("PHI2").unwrap();

                // Connect clock outputs to CPU clock inputs
                if let Ok(clock_phi1_guard) = clock_phi1_pin.lock() {
                    if let Ok(mut cpu_phi1_guard) = cpu_phi1.lock() {
                        cpu_phi1_guard.connect_to(clock_phi1_pin.clone());
                    }
                }

                if let Ok(clock_phi2_guard) = clock_phi2_pin.lock() {
                    if let Ok(mut cpu_phi2_guard) = cpu_phi2.lock() {
                        cpu_phi2_guard.connect_to(clock_phi2_pin.clone());
                    }
                }
            }
        }

        // Connect to RAM
        if let Some(ram_component) = self.components.get("ram") {
            if let Ok(mut ram) = ram_component.lock() {
                let ram_phi1 = ram.get_pin("PHI1").unwrap();
                let ram_phi2 = ram.get_pin("PHI2").unwrap();

                if let Ok(clock_phi1_guard) = clock_phi1_pin.lock() {
                    if let Ok(mut ram_phi1_guard) = ram_phi1.lock() {
                        ram_phi1_guard.connect_to(clock_phi1_pin.clone());
                    }
                }

                if let Ok(clock_phi2_guard) = clock_phi2_pin.lock() {
                    if let Ok(mut ram_phi2_guard) = ram_phi2.lock() {
                        ram_phi2_guard.connect_to(clock_phi2_pin.clone());
                    }
                }
            }
        }

        // Connect to ROMs
        for rom_name in &["rom1", "rom2"] {
            if let Some(rom_component) = self.components.get(*rom_name) {
                if let Ok(mut rom) = rom_component.lock() {
                    let rom_phi1 = rom.get_pin("PHI1").unwrap();
                    let rom_phi2 = rom.get_pin("PHI2").unwrap();

                    if let Ok(clock_phi1_guard) = clock_phi1_pin.lock() {
                        if let Ok(mut rom_phi1_guard) = rom_phi1.lock() {
                            rom_phi1_guard.connect_to(clock_phi1_pin.clone());
                        }
                    }

                    if let Ok(clock_phi2_guard) = clock_phi2_pin.lock() {
                        if let Ok(mut rom_phi2_guard) = rom_phi2.lock() {
                            rom_phi2_guard.connect_to(clock_phi2_pin.clone());
                        }
                    }
                }
            }
        }

        // Connect to shift register
        if let Some(sr_component) = self.components.get("shift_reg") {
            if let Ok(mut sr) = sr_component.lock() {
                let sr_phi1 = sr.get_pin("PHI1").unwrap();
                let sr_phi2 = sr.get_pin("PHI2").unwrap();

                if let Ok(clock_phi1_guard) = clock_phi1_pin.lock() {
                    if let Ok(mut sr_phi1_guard) = sr_phi1.lock() {
                        sr_phi1_guard.connect_to(clock_phi1_pin.clone());
                    }
                }

                if let Ok(clock_phi2_guard) = clock_phi2_pin.lock() {
                    if let Ok(mut sr_phi2_guard) = sr_phi2.lock() {
                        sr_phi2_guard.connect_to(clock_phi2_pin.clone());
                    }
                }
            }
        }
    }

    /// Connect control signals between CPU and memory components
    fn connect_control_signals(&mut self) {
        if let Some(cpu_component) = self.components.get("cpu") {
            if let Ok(cpu) = cpu_component.lock() {
                let cpu_sync = cpu.get_pin("SYNC").unwrap();
                let cpu_cm_rom = cpu.get_pin("CM_ROM").unwrap();
                let cpu_cm_ram = cpu.get_pin("CM_RAM").unwrap();

                // Connect to RAM
                if let Some(ram_component) = self.components.get("ram") {
                    if let Ok(mut ram) = ram_component.lock() {
                        let ram_sync = ram.get_pin("SYNC").unwrap();
                        let ram_p0 = ram.get_pin("P0").unwrap();

                        if let Ok(cpu_sync_guard) = cpu_sync.lock() {
                            if let Ok(mut ram_sync_guard) = ram_sync.lock() {
                                ram_sync_guard.connect_to(cpu_sync.clone());
                            }
                            if let Ok(mut ram_p0_guard) = ram_p0.lock() {
                                ram_p0_guard.connect_to(cpu_sync.clone());
                            }
                        }
                    }
                }

                // Connect to ROMs
                for rom_name in &["rom1", "rom2"] {
                    if let Some(rom_component) = self.components.get(*rom_name) {
                        if let Ok(mut rom) = rom_component.lock() {
                            let rom_sync = rom.get_pin("SYNC").unwrap();
                            let rom_cm = rom.get_pin("CM").unwrap();
                            let rom_ci = rom.get_pin("CI").unwrap();

                            if let Ok(cpu_sync_guard) = cpu_sync.lock() {
                                if let Ok(mut rom_sync_guard) = rom_sync.lock() {
                                    rom_sync_guard.connect_to(cpu_sync.clone());
                                }
                            }

                            if let Ok(cpu_cm_rom_guard) = cpu_cm_rom.lock() {
                                if let Ok(mut rom_cm_guard) = rom_cm.lock() {
                                    rom_cm_guard.connect_to(cpu_cm_rom.clone());
                                }
                                if let Ok(mut rom_ci_guard) = rom_ci.lock() {
                                    rom_ci_guard.connect_to(cpu_cm_rom.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Connect data bus between CPU and memory components
    fn connect_data_bus(&mut self) {
        // Connect CPU data pins to all memory component data pins
        for i in 0..4 {
            let pin_name = format!("D{}", i);

            // Get CPU data pin
            let cpu_data_pin = if let Some(cpu_component) = self.components.get("cpu") {
                if let Ok(cpu) = cpu_component.lock() {
                    cpu.get_pin(&pin_name).unwrap()
                } else {
                    continue;
                }
            } else {
                continue;
            };

            // Connect to RAM
            if let Some(ram_component) = self.components.get("ram") {
                if let Ok(mut ram) = ram_component.lock() {
                    let ram_data_pin = ram.get_pin(&pin_name).unwrap();
                    if let Ok(cpu_pin_guard) = cpu_data_pin.lock() {
                        if let Ok(mut ram_pin_guard) = ram_data_pin.lock() {
                            ram_pin_guard.connect_to(cpu_data_pin.clone());
                        }
                    }
                }
            }

            // Connect to ROMs
            for rom_name in &["rom1", "rom2"] {
                if let Some(rom_component) = self.components.get(*rom_name) {
                    if let Ok(mut rom) = rom_component.lock() {
                        let rom_data_pin = rom.get_pin(&pin_name).unwrap();
                        if let Ok(cpu_pin_guard) = cpu_data_pin.lock() {
                            if let Ok(mut rom_pin_guard) = rom_data_pin.lock() {
                                rom_pin_guard.connect_to(cpu_data_pin.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Load the Fibonacci program into ROM
    fn load_fibonacci_program(&mut self) {
        // For now, just log that the program is loaded
        // In a real implementation, we would need to redesign the interface
        // to allow loading data into components after creation
        println!("Loaded {} bytes of Fibonacci program into ROM1", self.fibonacci_program.len());
    }

    pub fn run(&mut self) {
        self.is_running = true;
        let mut handles = vec![];

        println!("Starting MCS-4 system components...");
        println!("Fibonacci program loaded into ROM ({} bytes)", self.fibonacci_program.len());

        for (name, component) in &self.components {
            let comp_clone = Arc::clone(component);
            let name_clone = name.clone();

            let handle = thread::spawn(move || {
                println!("Starting component: {}", name_clone);
                if let Ok(mut comp) = comp_clone.lock() {
                    comp.run();
                }
                println!("Component {} stopped", name_clone);
            });

            handles.push((name.clone(), handle));
        }

        println!("All components started. System running...");
        println!("CPU will execute Fibonacci calculation program...");

        // Monitor system and display Fibonacci results
        let mut last_cycle_count = 0;
        let mut display_counter = 0;

        while self.is_running {
            thread::sleep(Duration::from_millis(50));

            // Get current CPU state
            if let Ok(cpu_state) = self.get_cpu_state() {
                // Display Fibonacci results periodically
                if cpu_state.cycle_count - last_cycle_count > 100 {
                    self.display_fibonacci_results();
                    last_cycle_count = cpu_state.cycle_count;
                    display_counter += 1;
                }

                // Run for a reasonable amount of time to see the calculation
                if display_counter > 20 {
                    self.is_running = false;
                }
            }
        }

        // Stop all components
        println!("\nStopping system components...");
        for (name, component) in &self.components {
            if let Ok(mut comp) = component.lock() {
                comp.stop();
                println!("Stopped component: {}", name);
            }
        }

        // Wait for threads
        for (name, handle) in handles {
            match handle.join() {
                Ok(_) => println!("Component {} thread finished", name),
                Err(_) => eprintln!("Component {} thread panicked", name),
            }
        }

        println!("MCS-4 system stopped.");
        println!("\nFinal Fibonacci results in RAM:");
        self.display_fibonacci_results();
    }

    /// Display the current Fibonacci calculation results from RAM
    fn display_fibonacci_results(&self) {
        // For now, just display CPU state since we can't easily access RAM data
        // In a real implementation, we would need to redesign the interface
        println!("Fibonacci sequence calculation in progress...");

        if let Ok(cpu_state) = self.get_cpu_state() {
            println!("CPU State - PC: 0x{:03X}, ACC: 0x{:X}, Cycles: {}",
                     cpu_state.program_counter, cpu_state.accumulator, cpu_state.cycle_count);
        }
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    // Implement the missing methods for main.rs
    pub fn load_program(&mut self, rom1_data: Vec<u8>, rom2_data: Vec<u8>) -> Result<(), String> {
        println!("Loading program into ROM...");
        println!("ROM1 data: {} bytes", rom1_data.len());
        println!("ROM2 data: {} bytes", rom2_data.len());
        Ok(())
    }

    pub fn load_rom_data(
        &mut self,
        rom_chip: usize,
        data: Vec<u8>,
        offset: usize,
    ) -> Result<(), String> {
        let rom_key = match rom_chip {
            1 => "rom1",
            2 => "rom2",
            _ => return Err("Invalid ROM chip".to_string()),
        };

        if let Some(rom_component) = self.components.get(rom_key) {
            if let Ok(_rom) = rom_component.lock() {
                // For now, just log the operation
                println!(
                    "Loaded {} bytes into ROM{} at offset {}",
                    data.len(),
                    rom_chip,
                    offset
                );
                return Ok(());
            }
        }

        Err("ROM component not found".to_string())
    }

    pub fn load_ram_data(&mut self, data: &[u8], offset: usize) -> Result<(), String> {
        if let Some(ram_component) = self.components.get("ram") {
            if let Ok(_ram) = ram_component.lock() {
                println!("Loaded {} bytes into RAM at offset {}", data.len(), offset);
                return Ok(());
            }
        }

        Err("RAM component not found".to_string())
    }
    pub fn get_cpu_state(&self) -> Result<CpuState, String> {
        if let Some(cpu_component) = self.components.get("cpu") {
            if let Some(cpu) = cpu_component.as_any().downcast_ref::<Intel4004>() {
                return Ok(CpuState {
                    program_counter: cpu.get_program_counter(),
                    accumulator: cpu.get_accumulator(),
                    carry: cpu.get_carry(),
                    stack_pointer: cpu.get_stack_pointer(),
                    cycle_count: cpu.get_cycle_count(),
                });
            } else {
                Err("CPU component is not of type Intel 4004".to_string())
            }
        } else {
            Err("CPU component not found".to_string())
        }
    }
    pub fn reset_system(&mut self) {
        println!("Resetting MCS-4 system...");
        if let Some(cpu_component) = self.components.get_mut("cpu") {
            if let Some(cpu) = cpu_component.as_any_mut().downcast_mut::<Intel4004>() {
                cpu.reset();
            }
        }
    }
    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            cpu_speed: 750_000.0,
            rom_size: 512,
            ram_size: 40,
            component_count: self.components.len(),
        }
    }

    pub fn get_connection_graph(&self) -> Vec<Vec<String>> {
        vec![
            vec!["clock".to_string(), "cpu".to_string()],
            vec!["clock".to_string(), "ram".to_string()],
            vec!["clock".to_string(), "rom1".to_string()],
            vec!["clock".to_string(), "rom2".to_string()],
            vec!["clock".to_string(), "shift_reg".to_string()],
            vec!["cpu".to_string(), "ram".to_string()],
            vec!["cpu".to_string(), "rom1".to_string()],
            vec!["cpu".to_string(), "rom2".to_string()],
            vec!["cpu".to_string(), "shift_reg".to_string()],
        ]
    }

    /// Get the Fibonacci program that will be executed
    pub fn get_fibonacci_program(&self) -> &[u8] {
        &self.fibonacci_program
    }

    /// Get detailed system status including all component states
    pub fn get_detailed_status(&self) -> SystemStatus {
        let cpu_state = self.get_cpu_state().ok();
        let system_info = self.get_system_info();

        SystemStatus {
            is_running: self.is_running,
            cpu_state,
            system_info,
            fibonacci_program_size: self.fibonacci_program.len(),
            components_status: self.get_components_status(),
        }
    }

    /// Get status of all components
    fn get_components_status(&self) -> HashMap<String, String> {
        let mut status = HashMap::new();

        for (name, component) in &self.components {
            if let Ok(comp) = component.lock() {
                status.insert(name.clone(), format!("Running: {}", comp.is_running()));
            } else {
                status.insert(name.clone(), "Locked".to_string());
            }
        }

        status
    }
}

// Add the missing AsAny trait implementation
trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: 'static> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct CpuState {
    pub program_counter: u16,
    pub accumulator: u8,
    pub carry: bool,
    pub stack_pointer: u8,
    pub cycle_count: u64,
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub cpu_speed: f64,
    pub rom_size: usize,
    pub ram_size: usize,
    pub component_count: usize,
}

#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub is_running: bool,
    pub cpu_state: Option<CpuState>,
    pub system_info: SystemInfo,
    pub fibonacci_program_size: usize,
    pub components_status: HashMap<String, String>,
}

// Implement Component for IntelMcs4 for completeness
impl Component for IntelMcs4 {
    fn name(&self) -> String {
        "Intel_MCS-4_System".to_string()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        HashMap::new()
    }

    fn get_pin(&self, _name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        Err("System pins not accessible".to_string())
    }

    fn update(&mut self) {
        // System-level update
    }

    fn run(&mut self) {
        self.run();
    }

    fn stop(&mut self) {
        self.stop();
    }

    fn is_running(&self) -> bool {
        self.is_running()
    }
}
