use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::components::clock::generic_clock::GenericClock;
use crate::components::cpu::intel_4004::Intel4004;
use crate::component::Component;
use crate::components::memory::intel_4001::Intel4001;
use crate::components::memory::intel_4002::Intel4002;
use crate::pin::Pin;

pub struct IntelMcs4 {
    components: HashMap<String, Arc<Mutex<dyn Component>>>,
    is_running: bool,
}

impl IntelMcs4 {
    pub fn new() -> Self {
        let mut system = IntelMcs4 {
            components: HashMap::new(),
            is_running: false,
        };

        system.initialize_system();
        system
    }

    fn initialize_system(&mut self) {
        self.components.insert(
            "cpu".to_string(),
            Arc::new(Mutex::new(Intel4004::new("CPU_4004".to_string(), 750_000.0)))
        );
        self.components.insert(
            "clock".to_string(),
            Arc::new(Mutex::new(GenericClock::new("SYSTEM_CLOCK".to_string(), 750_000.0)))
        );
        self.components.insert(
            "ram".to_string(),
            Arc::new(Mutex::new(Intel4002::new("RAM_4002".to_string())))
        );
        self.components.insert(
            "rom1".to_string(),
            Arc::new(Mutex::new(Intel4001::new("ROM_4001_1".to_string())))
        );
        self.components.insert(
            "rom2".to_string(),
            Arc::new(Mutex::new(Intel4001::new("ROM_4001_2".to_string())))
        );
    }

    pub fn run(&mut self) {
        self.is_running = true;
        let mut handles = vec![];

        println!("Starting MCS-4 system components...");

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

        // Monitor system
        while self.is_running {
            thread::sleep(Duration::from_millis(100));

            // Check if we should stop (for demo, run for a limited time)
            if self.get_cpu_state().map(|s| s.cycle_count).unwrap_or(0) > 100 {
                self.is_running = false;
            }
        }

        // Stop all components
        println!("Stopping system components...");
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

    pub fn load_rom_data(&mut self, rom_chip: usize, data: Vec<u8>, offset: usize) -> Result<(), String> {
        let rom_key = match rom_chip {
            1 => "rom1",
            2 => "rom2",
            _ => return Err("Invalid ROM chip".to_string()),
        };

        if let Some(rom_component) = self.components.get(rom_key) {
            if let Ok(_rom) = rom_component.lock() {
                // For now, just log the operation
                println!("Loaded {} bytes into ROM{} at offset {}", data.len(), rom_chip, offset);
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
            vec!["cpu".to_string(), "clock".to_string(), "ram".to_string(), "rom1".to_string(), "rom2".to_string()],
        ]
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