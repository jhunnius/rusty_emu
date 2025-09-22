use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::component::Component;
use crate::components::clock::generic_clock::GenericClock;
use crate::components::cpu::intel_4004::Intel4004;
use crate::components::memory::intel_4001::Intel4001;
use crate::components::memory::intel_4002::Intel4002;
use crate::connection::ConnectionManager;
use crate::pin::Pin;

/// Complete Intel MCS-4 Microcomputer System
/// Contains: 4004 CPU, clock, 4002 RAM, and two 4001 ROM chips
pub struct IntelMcs4 {
    components: HashMap<String, Box<dyn Component>>,
    connection_manager: ConnectionManager,
    is_running: bool,
}

impl IntelMcs4 {
    pub fn new() -> Self {
        let mut system = IntelMcs4 {
            components: HashMap::new(),
            connection_manager: ConnectionManager::new(),
            is_running: false,
        };

        system.initialize_system();
        system
    }

    fn initialize_system(&mut self) {
        // Create components with historically accurate configuration

        // 4004 CPU - 750kHz clock speed (original MCS-4 speed)
        let cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0)
            .with_initial_pc(0x000);

        // System clock - 750kHz, 50% duty cycle
        let clock = GenericClock::new("SYSTEM_CLOCK".to_string(), 750_000.0)
            .with_duty_cycle(0.5);

        // 4002 RAM - 320 bits (40 nibbles)
        let ram = Intel4002::new("RAM_4002".to_string());

        // Two 4001 ROM chips (256 bytes each = 512 bytes total ROM)
        let rom1 = Intel4001::new("ROM_4001_1".to_string());
        let rom2 = Intel4001::new("ROM_4001_2".to_string());

        // Store components
        self.components.insert("cpu".to_string(), Box::new(cpu));
        self.components.insert("clock".to_string(), Box::new(clock));
        self.components.insert("ram".to_string(), Box::new(ram));
        self.components.insert("rom1".to_string(), Box::new(rom1));
        self.components.insert("rom2".to_string(), Box::new(rom2));

        // Connect the system
        self.connect_system();
    }

    fn connect_system(&mut self) {
        // Get references to all components' pins
        let cpu = self.components.get("cpu").unwrap();
        let clock = self.components.get("clock").unwrap();
        let ram = self.components.get("ram").unwrap();
        let rom1 = self.components.get("rom1").unwrap();
        let rom2 = self.components.get("rom2").unwrap();

        // Connect clock to CPU
        if let (Ok(cpu_phi1), Ok(cpu_phi2), Ok(clk_out)) = (
            cpu.get_pin("PHI1"),
            cpu.get_pin("PHI2"),
            clock.get_pin("CLK")
        ) {
            // Clock drives both CPU phase inputs (inverted for phase 2)
            let _ = self.connection_manager.connect_pins(clk_out.clone(), cpu_phi1.clone());
            // Note: In real system, φ2 would be inverted φ1
        }

        // Connect data bus (D0-D3) between all components
        for i in 0..4 {
            let data_pin_name = format!("D{}", i);

            if let (Ok(cpu_data), Ok(ram_data), Ok(rom1_data), Ok(rom2_data)) = (
                cpu.get_pin(&data_pin_name),
                ram.get_pin(&data_pin_name),
                rom1.get_pin(&data_pin_name),
                rom2.get_pin(&data_pin_name),
            ) {
                // Connect all data pins together (bus)
                let bus_pins = vec![cpu_data.clone(), ram_data.clone(), rom1_data.clone(), rom2_data.clone()];
                let _ = self.connection_manager.connect_bus(&bus_pins);
            }
        }

        // Connect control signals

        // SYNC signal (CPU → all chips)
        if let (Ok(cpu_sync), Ok(ram_sync), Ok(rom1_sync), Ok(rom2_sync)) = (
            cpu.get_pin("SYNC"),
            ram.get_pin("SYNC"),
            rom1.get_pin("SYNC"),
            rom2.get_pin("SYNC"),
        ) {
            let sync_pins = vec![cpu_sync.clone(), ram_sync.clone(), rom1_sync.clone(), rom2_sync.clone()];
            let _ = self.connection_manager.connect_bus(&sync_pins);
        }

        // CM-ROM signal (CPU → ROM chips)
        if let (Ok(cpu_cm_rom), Ok(rom1_cm_rom), Ok(rom2_cm_rom)) = (
            cpu.get_pin("CM_ROM"),
            rom1.get_pin("CM_ROM"),
            rom2.get_pin("CM_ROM"),
        ) {
            let cm_rom_pins = vec![cpu_cm_rom.clone(), rom1_cm_rom.clone(), rom2_cm_rom.clone()];
            let _ = self.connection_manager.connect_bus(&cm_rom_pins);
        }

        // CM-RAM signal (CPU → RAM chip)
        if let (Ok(cpu_cm_ram), Ok(ram_cm_ram)) = (
            cpu.get_pin("CM_RAM"),
            ram.get_pin("CM_RAM"),
        ) {
            let _ = self.connection_manager.connect_pins(cpu_cm_ram.clone(), ram_cm_ram.clone());
        }

        // RESET signal (external → all chips)
        // Create a system reset pin
        let reset_pin = Arc::new(Mutex::new(Pin::new("SYSTEM_RESET".to_string())));

        if let (Ok(cpu_reset), Ok(ram_reset), Ok(rom1_reset), Ok(rom2_reset)) = (
            cpu.get_pin("RESET"),
            ram.get_pin("RESET"),
            rom1.get_pin("RESET"),
            rom2.get_pin("RESET"),
        ) {
            let reset_pins = vec![reset_pin.clone(), cpu_reset.clone(), ram_reset.clone(), rom1_reset.clone(), rom2_reset.clone()];
            let _ = self.connection_manager.connect_bus(&reset_pins);
        }

        // Connect ROM output ports to form higher address lines
        // In MCS-4, ROM output ports provide address lines A8-A11
        if let (Ok(rom1_io0), Ok(rom2_io0)) = (rom1.get_pin("IO0"), rom2.get_pin("IO0")) {
            // These would be connected to address decoding logic in a real system
            // For now, we'll connect them together as a simple bus
            let _ = self.connection_manager.connect_pins(rom1_io0.clone(), rom2_io0.clone());
        }

        // Note: In a real MCS-4 system, there would be more complex wiring
        // for bank selection and I/O expansion
    }

    pub fn load_rom_data(&mut self, rom_chip: usize, data: Vec<u8>, offset: usize) -> Result<(), String> {
        let rom_key = match rom_chip {
            1 => "rom1",
            2 => "rom2",
            _ => return Err("Invalid ROM chip number (1 or 2)".to_string()),
        };

        if let Some(rom_component) = self.components.get_mut(rom_key) {
            // Downcast to Intel4001 to call ROM-specific methods
            if let Some(rom) = rom_component.as_any_mut().downcast_mut::<Intel4001>() {
                rom.load_rom_data(data, offset)
            } else {
                Err("Component is not an Intel4001".to_string())
            }
        } else {
            Err(format!("ROM chip {} not found", rom_chip))
        }
    }

    pub fn load_ram_data(&mut self, data: &[u8], offset: usize) -> Result<(), String> {
        if let Some(ram_component) = self.components.get_mut("ram") {
            if let Some(ram) = ram_component.as_any_mut().downcast_mut::<Intel4002>() {
                ram.initialize_ram(data)
            } else {
                Err("Component is not an Intel4002".to_string())
            }
        } else {
            Err("RAM component not found".to_string())
        }
    }

    pub fn set_cpu_program_counter(&mut self, address: u16) -> Result<(), String> {
        if let Some(cpu_component) = self.components.get_mut("cpu") {
            if let Some(cpu) = cpu_component.as_any_mut().downcast_mut::<Intel4004>() {
                cpu.set_program_counter(address);
                Ok(())
            } else {
                Err("Component is not an Intel4004".to_string())
            }
        } else {
            Err("CPU component not found".to_string())
        }
    }

    pub fn get_cpu_state(&self) -> Result<CpuState, String> {
        if let Some(cpu_component) = self.components.get("cpu") {
            if let Some(cpu) = cpu_component.as_any().downcast_ref::<Intel4004>() {
                Ok(CpuState {
                    program_counter: cpu.get_program_counter(),
                    accumulator: cpu.get_accumulator(),
                    carry: cpu.get_carry(),
                    stack_pointer: cpu.get_stack_pointer(),
                    cycle_count: cpu.get_cycle_count(),
                })
            } else {
                Err("Component is not an Intel4004".to_string())
            }
        } else {
            Err("CPU component not found".to_string())
        }
    }

    pub fn reset_system(&mut self) {
        // Reset all components
        for component in self.components.values_mut() {
            if let Some(cpu) = component.as_any_mut().downcast_mut::<Intel4004>() {
                cpu.reset();
            } else if let Some(ram) = component.as_any_mut().downcast_mut::<Intel4002>() {
                // RAM reset would clear registers but not memory
            } else if let Some(rom) = component.as_any_mut().downcast_mut::<Intel4001>() {
                // ROM reset would clear I/O latches
            }
        }

        // Pulse the system reset line
        if let Some(reset_pin) = self.connection_manager.get_pin("SYSTEM_RESET") {
            if let Ok(mut pin_guard) = reset_pin.lock() {
                // Pulse reset high then low
                pin_guard.set_driver(Some("system".to_string()), crate::pin::PinValue::High);
                // In a real implementation, we'd wait then set low
            }
        }
    }

    pub fn single_step(&mut self) -> Result<(), String> {
        // Execute one instruction cycle
        if let Some(cpu_component) = self.components.get_mut("cpu") {
            if let Some(cpu) = cpu_component.as_any_mut().downcast_mut::<Intel4004>() {
                // Manually advance the CPU by one cycle
                // This would require exposing cycle-level control in the CPU
                cpu.update();
                Ok(())
            } else {
                Err("Component is not an Intel4004".to_string())
            }
        } else {
            Err("CPU component not found".to_string())
        }
    }
}

// Trait for type erasure and downcasting
pub trait AsAny {
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

// CPU state for monitoring
#[derive(Debug, Clone)]
pub struct CpuState {
    pub program_counter: u16,
    pub accumulator: u8,
    pub carry: bool,
    pub stack_pointer: u8,
    pub cycle_count: u64,
}

impl Component for IntelMcs4 {
    fn name(&self) -> String {
        "Intel_MCS-4_System".to_string()
    }

    fn pins(&self) -> &HashMap<String, Arc<Mutex<Pin>>> {
        // System-level pins would include external interfaces
        // For now, return an empty map or system-level pins
        &HashMap::new()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        // Delegate to appropriate component or system pins
        Err(format!("System pin {} not directly accessible", name))
    }

    fn update(&mut self) {
        if !self.is_running {
            return;
        }

        // Update all components
        for component in self.components.values_mut() {
            component.update();
        }
    }

    fn run(&mut self) {
        self.is_running = true;

        // Start all components in separate threads
        let mut handles = vec![];

        for (name, component) in self.components.iter_mut() {
            let name_clone = name.clone();
            let mut comp = component; // We need to work with the component

            // For demonstration, we'll run CPU in main thread and others in background
            if name_clone == "cpu" {
                // CPU runs in main thread for now
            } else {
                let handle = thread::spawn(move || {
                    comp.run();
                });
                handles.push(handle);
            }
        }

        // Run CPU in main thread
        if let Some(cpu) = self.components.get_mut("cpu") {
            while self.is_running {
                cpu.update();
                thread::sleep(Duration::from_micros(10));
            }
        }

        // Wait for other components to stop
        for handle in handles {
            let _ = handle.join();
        }
    }

    fn stop(&mut self) {
        self.is_running = false;

        // Stop all components
        for component in self.components.values_mut() {
            component.stop();
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
    }
}

// System monitoring and control methods
impl IntelMcs4 {
    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            cpu_speed: 750_000.0,
            rom_size: 512, // 2 × 256 bytes
            ram_size: 40,  // 40 nibbles
            component_count: self.components.len(),
        }
    }

    pub fn get_connection_graph(&self) -> Vec<Vec<String>> {
        self.connection_manager.get_connection_groups()
    }

    pub fn load_program(&mut self, rom1_data: Vec<u8>, rom2_data: Vec<u8>) -> Result<(), String> {
        // Load program into both ROM chips
        self.load_rom_data(1, rom1_data, 0)?;
        self.load_rom_data(2, rom2_data, 0)?;

        // Reset system to start execution
        self.reset_system();

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub cpu_speed: f64,
    pub rom_size: usize,
    pub ram_size: usize,
    pub component_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcs4_creation() {
        let system = IntelMcs4::new();
        let info = system.get_system_info();

        assert_eq!(info.component_count, 5); // CPU, clock, RAM, 2×ROM
        assert_eq!(info.rom_size, 512);
        assert_eq!(info.ram_size, 40);
        assert!(!system.is_running());
    }

    #[test]
    fn test_mcs4_rom_loading() {
        let mut system = IntelMcs4::new();

        let rom1_data = vec![0x01, 0x02, 0x03, 0x04];
        let rom2_data = vec![0x05, 0x06, 0x07, 0x08];

        assert!(system.load_rom_data(1, rom1_data.clone(), 0).is_ok());
        assert!(system.load_rom_data(2, rom2_data.clone(), 0).is_ok());
    }

    #[test]
    fn test_mcs4_cpu_state() {
        let mut system = IntelMcs4::new();

        // Set initial PC
        assert!(system.set_cpu_program_counter(0x100).is_ok());

        let state = system.get_cpu_state().unwrap();
        assert_eq!(state.program_counter, 0x100);
    }

    #[test]
    fn test_mcs4_system_reset() {
        let mut system = IntelMcs4::new();

        // Set some state
        system.set_cpu_program_counter(0x123).unwrap();

        // Reset system
        system.reset_system();

        // Verify reset (PC should be 0 after reset)
        let state = system.get_cpu_state().unwrap();
        assert_eq!(state.program_counter, 0);
    }

    #[test]
    fn test_mcs4_connection_graph() {
        let system = IntelMcs4::new();
        let connections = system.get_connection_graph();

        // Should have several connection groups (data bus, control signals, etc.)
        assert!(!connections.is_empty());
    }
}