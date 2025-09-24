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
        // Store components directly - they're already thread-safe!
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

        // Start each component in its own thread
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

        // Main thread: monitor system and handle graceful shutdown
        while self.is_running {
            thread::sleep(Duration::from_millis(100));

            // Check if all components are still running
            let all_running = self.components.values().all(|comp| {
                comp.lock().map(|c| c.is_running()).unwrap_or(false)
            });

            if !all_running {
                println!("Some components stopped, shutting down system...");
                self.is_running = false;
            }
        }

        // Stop all components gracefully
        println!("Stopping system components...");
        for (name, component) in &self.components {
            if let Ok(mut comp) = component.lock() {
                comp.stop();
                println!("Stopped component: {}", name);
            }
        }

        // Wait for all threads to finish
        for (name, handle) in handles {
            match handle.join() {
                Ok(_) => println!("Component {} thread finished", name),
                Err(e) => eprintln!("Component {} thread panicked: {:?}", name, e),
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

    // Helper methods to access components
    pub fn get_component(&self, name: &str) -> Option<&Arc<Mutex<dyn Component>>> {
        self.components.get(name)
    }

    pub fn update_system(&mut self) {
        if !self.is_running {
            return;
        }

        // Update all components (for single-threaded mode)
        for component in self.components.values() {
            if let Ok(mut comp) = component.lock() {
                comp.update();
            }
        }
    }
}

// Single-threaded version for simpler operation
pub struct SingleThreadedMcs4 {
    components: HashMap<String, Box<dyn Component>>,
    is_running: bool,
}

impl SingleThreadedMcs4 {
    pub fn new() -> Self {
        let mut system = SingleThreadedMcs4 {
            components: HashMap::new(),
            is_running: false,
        };

        system.initialize_system();
        system
    }

    fn initialize_system(&mut self) {
        // Store components without Arc<Mutex> since we're single-threaded
        self.components.insert(
            "cpu".to_string(),
            Box::new(Intel4004::new("CPU_4004".to_string(), 750_000.0))
        );
        self.components.insert(
            "clock".to_string(),
            Box::new(GenericClock::new("SYSTEM_CLOCK".to_string(), 750_000.0))
        );
        self.components.insert(
            "ram".to_string(),
            Box::new(Intel4002::new("RAM_4002".to_string()))
        );
        self.components.insert(
            "rom1".to_string(),
            Box::new(Intel4001::new("ROM_4001_1".to_string()))
        );
        self.components.insert(
            "rom2".to_string(),
            Box::new(Intel4001::new("ROM_4001_2".to_string()))
        );
    }

    pub fn run_single_threaded(&mut self) {
        self.is_running = true;
        println!("Starting MCS-4 system in single-threaded mode...");

        // Run all components in the main thread
        while self.is_running {
            for component in self.components.values_mut() {
                component.update();
            }
            thread::sleep(Duration::from_micros(10));
        }

        // Stop all components
        for component in self.components.values_mut() {
            component.stop();
        }

        println!("Single-threaded MCS-4 system stopped.");
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }
}

// Rest of your existing MCS-4 code (CpuState, SystemInfo, etc.)
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

// Implement the remaining methods for IntelMcs4...
impl IntelMcs4 {
    pub fn load_program(&mut self, _rom1_data: Vec<u8>, _rom2_data: Vec<u8>) -> Result<(), String> {
        // Implementation here
        Ok(())
    }

    // ... other methods
}

impl Component for IntelMcs4 {
    fn name(&self) -> String {
        "Intel_MCS-4_System".to_string()
    }

    fn pins(&self) -> &HashMap<String, Arc<Mutex<Pin>>> {
        &HashMap::new()
    }

    fn get_pin(&self, _name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        Err("System pins not directly accessible".to_string())
    }

    fn update(&mut self) {
        self.update_system();
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