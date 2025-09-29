use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::component::Component;
use crate::components::clock::generic_clock::GenericClock;
use crate::components::cpu::intel_4004::Intel4004;
use crate::components::memory::intel_4001::Intel4001;
use crate::components::memory::intel_4002::{Intel4002, RamVariant};
use crate::components::memory::intel_4003::Intel4003;
use crate::pin::Pin;

/// Intel MCS-4 System implementation according to Fig.1 configuration
/// Features 16 ROMs and 16 RAMs with specific connectivity requirements
pub struct IntelMcs4Max {
    components: HashMap<String, Arc<Mutex<dyn Component>>>,
    is_running: bool,
    // Fig.1 configuration: 16 ROMs and 16 RAMs
    rom_chips: Vec<Arc<Mutex<Intel4001>>>,
    ram_chips: Vec<Arc<Mutex<Intel4002>>>,
    shift_registers: Vec<Arc<Mutex<Intel4003>>>,
}

impl IntelMcs4Max {
    pub fn new() -> Self {
        let mut system = IntelMcs4Max {
            components: HashMap::new(),
            is_running: false,
            rom_chips: Vec::new(),
            ram_chips: Vec::new(),
            shift_registers: Vec::new(),
        };

        system.initialize_fig1_system();
        system
    }

    /// Initialize the MCS-4 system according to Fig.1 configuration
    /// 16 ROMs and 16 RAMs with specific connectivity
    fn initialize_fig1_system(&mut self) {
        // Create CPU with proper clock speed
        let cpu = Intel4004::new("CPU_4004".to_string(), 750_000.0);
        self.components.insert("cpu".to_string(), Arc::new(Mutex::new(cpu)));

        // Create clock generator
        let clock = GenericClock::new("SYSTEM_CLOCK".to_string(), 750_000.0);
        self.components.insert("clock".to_string(), Arc::new(Mutex::new(clock)));

        // Create 16 ROM chips (4001)
        for i in 0..16 {
            let rom_name = format!("ROM_4001_{:02}", i);
            let rom = Intel4001::new(rom_name.clone());
            let rom_arc = Arc::new(Mutex::new(rom));
            self.components.insert(format!("rom_{:02}", i), rom_arc.clone());
            self.rom_chips.push(rom_arc);
        }

        // Create 16 RAM chips (4002) with variants
        for i in 0..16 {
            let ram_name = format!("RAM_4002_{:02}", i);
            // Use 4002-1 variant for most chips, 4002-2 for specific ones as per requirements
            let variant = if i == 3 { RamVariant::Type2 } else { RamVariant::Type1 };
            let ram = Intel4002::new_with_variant_and_access_time(ram_name.clone(), variant, 500);
            let ram_arc = Arc::new(Mutex::new(ram));
            self.components.insert(format!("ram_{:02}", i), ram_arc.clone());
            self.ram_chips.push(ram_arc);
        }

        // Create shift registers (4003) for serial connections
        // ROM 15 feeds a 4003
        let shift_reg1 = Intel4003::new("SHIFT_4003_ROM15".to_string());
        let shift_reg1_arc = Arc::new(Mutex::new(shift_reg1));
        self.components.insert("shift_reg_rom15".to_string(), shift_reg1_arc.clone());
        self.shift_registers.push(shift_reg1_arc);

        // 4002-1 on CM-RAM3 line feeds series of two 4003 chips
        let shift_reg2 = Intel4003::new("SHIFT_4003_RAM3_1".to_string());
        let shift_reg2_arc = Arc::new(Mutex::new(shift_reg2));
        self.components.insert("shift_reg_ram3_1".to_string(), shift_reg2_arc.clone());
        self.shift_registers.push(shift_reg2_arc);

        let shift_reg3 = Intel4003::new("SHIFT_4003_RAM3_2".to_string());
        let shift_reg3_arc = Arc::new(Mutex::new(shift_reg3));
        self.components.insert("shift_reg_ram3_2".to_string(), shift_reg3_arc.clone());
        self.shift_registers.push(shift_reg3_arc);

        // Connect components via pin connections
        self.connect_fig1_components();
    }

    /// Connect components according to Fig.1 configuration
    fn connect_fig1_components(&mut self) {
        // Connect clock signals to all components
        self.connect_clock_signals();

        // Connect CPU control signals to memory components
        self.connect_control_signals();

        // Connect data bus between CPU and memory components
        self.connect_data_bus();

        // Connect specific Fig.1 requirements
        self.connect_fig1_specific_connections();
    }

    /// Connect clock signals from clock generator to all components
    fn connect_clock_signals(&mut self) {
        let clock_phi1_pin = self.components.get("clock").unwrap().lock().unwrap().get_pin("CLK").unwrap();
        let clock_phi2_pin = clock_phi1_pin.clone(); // Simplified for now

        // Connect to CPU
        if let Some(cpu_component) = self.components.get("cpu") {
            if let Ok(cpu) = cpu_component.lock() {
                let cpu_phi1 = cpu.get_pin("PHI1").unwrap();
                let cpu_phi2 = cpu.get_pin("PHI2").unwrap();

                if let Ok(_clock_phi1_guard) = clock_phi1_pin.lock() {
                    if let Ok(mut cpu_phi1_guard) = cpu_phi1.lock() {
                        cpu_phi1_guard.connect_to(clock_phi1_pin.clone());
                    }
                }

                if let Ok(_clock_phi2_guard) = clock_phi2_pin.lock() {
                    if let Ok(mut cpu_phi2_guard) = cpu_phi2.lock() {
                        cpu_phi2_guard.connect_to(clock_phi2_pin.clone());
                    }
                }
            }
        }

        // Connect to all ROMs
        for rom_arc in &self.rom_chips {
            if let Ok(mut rom) = rom_arc.lock() {
                let rom_phi1 = rom.get_pin("PHI1").unwrap();
                let rom_phi2 = rom.get_pin("PHI2").unwrap();

                if let Ok(_clock_phi1_guard) = clock_phi1_pin.lock() {
                    if let Ok(mut rom_phi1_guard) = rom_phi1.lock() {
                        rom_phi1_guard.connect_to(clock_phi1_pin.clone());
                    }
                }

                if let Ok(_clock_phi2_guard) = clock_phi2_pin.lock() {
                    if let Ok(mut rom_phi2_guard) = rom_phi2.lock() {
                        rom_phi2_guard.connect_to(clock_phi2_pin.clone());
                    }
                }
            }
        }

        // Connect to all RAMs
        for ram_arc in &self.ram_chips {
            if let Ok(mut ram) = ram_arc.lock() {
                let ram_phi1 = ram.get_pin("PHI1").unwrap();
                let ram_phi2 = ram.get_pin("PHI2").unwrap();

                if let Ok(_clock_phi1_guard) = clock_phi1_pin.lock() {
                    if let Ok(mut ram_phi1_guard) = ram_phi1.lock() {
                        ram_phi1_guard.connect_to(clock_phi1_pin.clone());
                    }
                }

                if let Ok(_clock_phi2_guard) = clock_phi2_pin.lock() {
                    if let Ok(mut ram_phi2_guard) = ram_phi2.lock() {
                        ram_phi2_guard.connect_to(clock_phi2_pin.clone());
                    }
                }
            }
        }

        // Connect to all shift registers
        for sr_arc in &self.shift_registers {
            if let Ok(mut sr) = sr_arc.lock() {
                let sr_phi1 = sr.get_pin("PHI1").unwrap();
                let sr_phi2 = sr.get_pin("PHI2").unwrap();

                if let Ok(_clock_phi1_guard) = clock_phi1_pin.lock() {
                    if let Ok(mut sr_phi1_guard) = sr_phi1.lock() {
                        sr_phi1_guard.connect_to(clock_phi1_pin.clone());
                    }
                }

                if let Ok(_clock_phi2_guard) = clock_phi2_pin.lock() {
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
                let _cpu_cm_ram = cpu.get_pin("CM_RAM").unwrap();

                // Connect to all ROMs
                for (_i, rom_arc) in self.rom_chips.iter().enumerate() {
                    if let Ok(mut rom) = rom_arc.lock() {
                        let rom_sync = rom.get_pin("SYNC").unwrap();
                        let rom_cm = rom.get_pin("CM").unwrap();
                        let rom_ci = rom.get_pin("CI").unwrap();

                        if let Ok(_cpu_sync_guard) = cpu_sync.lock() {
                            if let Ok(mut rom_sync_guard) = rom_sync.lock() {
                                rom_sync_guard.connect_to(cpu_sync.clone());
                            }
                        }

                        if let Ok(_cpu_cm_rom_guard) = cpu_cm_rom.lock() {
                            if let Ok(mut rom_cm_guard) = rom_cm.lock() {
                                rom_cm_guard.connect_to(cpu_cm_rom.clone());
                            }
                            if let Ok(mut rom_ci_guard) = rom_ci.lock() {
                                rom_ci_guard.connect_to(cpu_cm_rom.clone());
                            }
                        }
                    }
                }

                // Connect to all RAMs
                for (_i, ram_arc) in self.ram_chips.iter().enumerate() {
                    if let Ok(mut ram) = ram_arc.lock() {
                        let ram_sync = ram.get_pin("SYNC").unwrap();
                        let ram_p0 = ram.get_pin("P0").unwrap();

                        if let Ok(_cpu_sync_guard) = cpu_sync.lock() {
                            if let Ok(mut ram_sync_guard) = ram_sync.lock() {
                                ram_sync_guard.connect_to(cpu_sync.clone());
                            }
                            if let Ok(mut ram_p0_guard) = ram_p0.lock() {
                                ram_p0_guard.connect_to(cpu_sync.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Connect data bus between CPU and memory components
    fn connect_data_bus(&mut self) {
        if let Some(cpu_component) = self.components.get("cpu") {
            if let Ok(cpu) = cpu_component.lock() {
                // Connect CPU data pins to all memory component data pins
                for data_pin_idx in 0..4 {
                    let pin_name = format!("D{}", data_pin_idx);
                    let cpu_data_pin = cpu.get_pin(&pin_name).unwrap();

                    // Connect to all ROMs
                    for rom_arc in &self.rom_chips {
                        if let Ok(mut rom) = rom_arc.lock() {
                            let rom_data_pin = rom.get_pin(&pin_name).unwrap();
                            if let Ok(_cpu_pin_guard) = cpu_data_pin.lock() {
                                if let Ok(mut rom_pin_guard) = rom_data_pin.lock() {
                                    rom_pin_guard.connect_to(cpu_data_pin.clone());
                                }
                            }
                        }
                    }

                    // Connect to all RAMs
                    for ram_arc in &self.ram_chips {
                        if let Ok(mut ram) = ram_arc.lock() {
                            let ram_data_pin = ram.get_pin(&pin_name).unwrap();
                            if let Ok(_cpu_pin_guard) = cpu_data_pin.lock() {
                                if let Ok(mut ram_pin_guard) = ram_data_pin.lock() {
                                    ram_pin_guard.connect_to(cpu_data_pin.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Connect Fig.1 specific connections
    fn connect_fig1_specific_connections(&mut self) {
        // ROM 15 feeds a 4003
        if let Some(rom15) = self.rom_chips.get(15) {
            if let Some(shift_reg) = self.shift_registers.get(0) {
                // Connect ROM 15 I/O pins to shift register serial input
                for i in 0..4 {
                    if let Ok(rom) = rom15.lock() {
                        if let Ok(sr) = shift_reg.lock() {
                            if let Ok(rom_io_pin) = rom.get_pin(&format!("IO{}", i)) {
                                if let Ok(sr_data_pin) = sr.get_pin(&format!("D{}", i)) {
                                    if let Ok(_rom_io_guard) = rom_io_pin.lock() {
                                        if let Ok(mut sr_data_guard) = sr_data_pin.lock() {
                                            sr_data_guard.connect_to(rom_io_pin.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4002-1 on CM-RAM3 line feeds series of two 4003 chips
        if let Some(ram3) = self.ram_chips.get(3) {
            if let Some(shift_reg1) = self.shift_registers.get(1) {
                if let Some(shift_reg2) = self.shift_registers.get(2) {
                    // Connect RAM 3 output ports to first shift register
                    for i in 0..4 {
                        if let Ok(ram) = ram3.lock() {
                            if let Ok(sr1) = shift_reg1.lock() {
                                if let Ok(ram_output_pin) = ram.get_pin(&format!("O{}", i)) {
                                    if let Ok(sr1_data_pin) = sr1.get_pin(&format!("D{}", i)) {
                                        if let Ok(_ram_output_guard) = ram_output_pin.lock() {
                                            if let Ok(mut sr1_data_guard) = sr1_data_pin.lock() {
                                                sr1_data_guard.connect_to(ram_output_pin.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Connect first shift register to second shift register (serial connection)
                    for i in 0..4 {
                        if let Ok(sr1) = shift_reg1.lock() {
                            if let Ok(sr2) = shift_reg2.lock() {
                                if let Ok(sr1_output_pin) = sr1.get_pin(&format!("O{}", i)) {
                                    if let Ok(sr2_data_pin) = sr2.get_pin(&format!("D{}", i)) {
                                        if let Ok(_sr1_output_guard) = sr1_output_pin.lock() {
                                            if let Ok(mut sr2_data_guard) = sr2_data_pin.lock() {
                                                sr2_data_guard.connect_to(sr1_output_pin.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get the 60 I/O lines from ROMs
    pub fn get_rom_io_lines(&self) -> Vec<Arc<Mutex<Pin>>> {
        let mut io_lines = Vec::new();

        for rom_arc in &self.rom_chips {
            if let Ok(rom) = rom_arc.lock() {
                for i in 0..4 {
                    if let Ok(pin) = rom.get_pin(&format!("IO{}", i)) {
                        io_lines.push(pin.clone());
                    }
                }
            }
        }

        io_lines
    }

    /// Get the 60 output lines from RAMs
    pub fn get_ram_output_lines(&self) -> Vec<Arc<Mutex<Pin>>> {
        let mut output_lines = Vec::new();

        for ram_arc in &self.ram_chips {
            if let Ok(ram) = ram_arc.lock() {
                for i in 0..4 {
                    if let Ok(pin) = ram.get_pin(&format!("O{}", i)) {
                        output_lines.push(pin.clone());
                    }
                }
            }
        }

        output_lines
    }

    /// Get the 2 serial ports
    pub fn get_serial_ports(&self) -> Vec<Arc<Mutex<Pin>>> {
        let mut serial_ports = Vec::new();

        // Serial ports are the output pins of the last shift registers in each chain
        if let Some(sr1) = self.shift_registers.get(1) {
            if let Ok(shift_reg) = sr1.lock() {
                for i in 0..10 {
                    if let Ok(pin) = shift_reg.get_pin(&format!("O{}", i)) {
                        serial_ports.push(pin.clone());
                    }
                }
            }
        }

        if let Some(sr2) = self.shift_registers.get(2) {
            if let Ok(shift_reg) = sr2.lock() {
                for i in 0..10 {
                    if let Ok(pin) = shift_reg.get_pin(&format!("O{}", i)) {
                        serial_ports.push(pin.clone());
                    }
                }
            }
        }

        serial_ports
    }

    pub fn run(&mut self) {
        self.is_running = true;
        println!("Starting MCS-4 Fig.1 system with 16 ROMs and 16 RAMs...");

        for (name, component) in &self.components {
            let comp_clone = Arc::clone(component);
            let name_clone = name.clone();

            std::thread::spawn(move || {
                println!("Starting component: {}", name_clone);
                if let Ok(mut comp) = comp_clone.lock() {
                    comp.run();
                }
                println!("Component {} stopped", name_clone);
            });
        }

        println!("All components started. Fig.1 system running...");
        println!("System exposes:");
        println!("- 60 I/O lines from {} ROM chips", self.rom_chips.len());
        println!("- 60 output lines from {} RAM chips", self.ram_chips.len());
        println!("- 2 serial ports from shift register chains");

        // Keep system running for demonstration
        std::thread::sleep(std::time::Duration::from_secs(5));

        self.is_running = false;
        println!("Fig.1 system stopped.");
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }
}