//! # GUI State Management
//!
//! This module manages the state of the GUI application, including system status,
//! component states, and user interface state. It provides thread-safe access
//! to emulator state for real-time GUI updates.

use crate::system_config::ConfigurableSystem;
use std::sync::{Arc, Mutex};

/// GUI state structure containing all UI-relevant data
#[derive(Debug, Clone)]
pub struct GuiState {
    /// Whether a system is currently loaded
    pub system_loaded: bool,
    /// Whether the system is currently running
    pub system_running: bool,
    /// Current cycle count
    pub cycle_count: u64,
    /// System information
    pub system_info: Option<SystemInfo>,
    /// Component states
    pub component_states: ComponentStates,
    /// Memory state
    pub memory_state: MemoryState,
    /// CPU register state
    pub register_state: RegisterState,
    /// Last error message
    pub last_error: Option<String>,
}

/// System information for display
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub name: String,
    pub description: String,
    pub component_count: usize,
    pub cpu_speed: f64,
    pub rom_size: usize,
    pub ram_size: usize,
}

/// Component states for monitoring
#[derive(Debug, Clone)]
pub struct ComponentStates {
    pub cpu_running: bool,
    pub ram_running: bool,
    pub rom_running: bool,
    pub clock_running: bool,
}

/// Memory state for display
#[derive(Debug, Clone)]
pub struct MemoryState {
    pub ram_contents: Vec<[u8; 4]>, // 4 banks of RAM data
    pub selected_bank: usize,
    pub selected_address: usize,
}

/// CPU register state for display
#[derive(Debug, Clone)]
pub struct RegisterState {
    pub accumulator: u8,
    pub carry_flag: bool,
    pub program_counter: u16,
    pub index_registers: [u8; 16], // 16 index registers (R0-R15)
    pub stack_pointer: u8,
}

impl GuiState {
    /// Create a new GUI state instance
    pub fn new() -> Self {
        Self {
            system_loaded: false,
            system_running: false,
            cycle_count: 0,
            system_info: None,
            component_states: ComponentStates {
                cpu_running: false,
                ram_running: false,
                rom_running: false,
                clock_running: false,
            },
            memory_state: MemoryState {
                ram_contents: vec![[0; 4]; 4], // Initialize 4 banks with 4 bytes each
                selected_bank: 0,
                selected_address: 0,
            },
            register_state: RegisterState {
                accumulator: 0,
                carry_flag: false,
                program_counter: 0,
                index_registers: [0; 16],
                stack_pointer: 0,
            },
            last_error: None,
        }
    }

    /// Update state from the current system
    pub fn update_from_system(&mut self, system: &Arc<Mutex<ConfigurableSystem>>) {
        if let Ok(system_guard) = system.lock() {
            // Update basic system state
            self.system_running = system_guard.is_running();

            // Update system info if not already set
            if self.system_info.is_none() {
                self.system_info = Some(system_guard.get_system_info().into());
            }

            // Update component states
            self.update_component_states(&system_guard);

            // Update cycle count (simulate for now)
            if self.system_running {
                self.cycle_count += 1;
            }
        }
    }

    /// Update component running states
    fn update_component_states(&mut self, system: &ConfigurableSystem) {
        let components = system.get_components();

        self.component_states.cpu_running = components
            .get("CPU_4004")
            .map_or(false, |comp| comp.lock().map_or(false, |c| c.is_running()));

        self.component_states.ram_running = components
            .get("RAM_4002")
            .map_or(false, |comp| comp.lock().map_or(false, |c| c.is_running()));

        self.component_states.rom_running = components
            .get("ROM_4001_1")
            .or_else(|| components.get("ROM_4001_2"))
            .map_or(false, |comp| comp.lock().map_or(false, |c| c.is_running()));

        self.component_states.clock_running = components
            .get("SYSTEM_CLOCK")
            .map_or(false, |comp| comp.lock().map_or(false, |c| c.is_running()));
    }

    /// Set an error message
    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    /// Clear the error message
    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    /// Get the current error message
    pub fn get_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

impl From<crate::system_config::SystemInfo> for SystemInfo {
    fn from(info: crate::system_config::SystemInfo) -> Self {
        Self {
            name: info.name,
            description: info.description,
            component_count: info.component_count,
            cpu_speed: info.cpu_speed,
            rom_size: info.rom_size,
            ram_size: info.ram_size,
        }
    }
}
