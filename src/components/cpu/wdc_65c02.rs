use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::component::Component;
use crate::components::cpu::MOS6502;
use crate::pin::Pin;

/// WDC 65C02 - CMOS version of 6502 with additional instructions
pub struct WDC65C02 {
    pub(crate) base: MOS6502,
    // 65C02-specific state
    stop_mode: bool,
    wait_mode: bool,
}

impl WDC65C02 {
    pub fn new(name: String) -> Self {
        WDC65C02 {
            base: MOS6502::new(name),
            stop_mode: false,
            wait_mode: false,
        }
    }

    // 65C02 specific methods
    pub fn enter_stop_mode(&mut self) {
        self.stop_mode = true;
        self.wait_mode = false;
    }

    pub fn enter_wait_mode(&mut self) {
        self.wait_mode = true;
        self.stop_mode = false;
    }

    pub fn exit_low_power_modes(&mut self) {
        self.stop_mode = false;
        self.wait_mode = false;
    }

    pub fn is_in_stop_mode(&self) -> bool {
        self.stop_mode
    }

    pub fn is_in_wait_mode(&self) -> bool {
        self.wait_mode
    }

    // 65C02 additional instructions would be implemented here
    fn execute_65c02_instruction(&mut self) {
        if self.stop_mode {
            // CPU is stopped - no operation
            return;
        }

        if self.wait_mode {
            // CPU is waiting for interrupt - no operation
            return;
        }

        // For compilation, just call the base 6502 implementation
        // In a full implementation, this would handle 65C02-specific instructions
    }
}

impl Component for WDC65C02 {
    fn name(&self) -> String {
        self.base.name()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.base.get_pin(name)
    }

    fn update(&mut self) {
        if self.stop_mode || self.wait_mode {
            // In low-power modes, only check for interrupts
            let (irq, nmi, reset, _) = self.base.read_control_pins();

            if nmi || reset || (irq && !self.base.get_status_register() & 0x04 == 0) {
                // Exit low-power modes on interrupt or reset
                self.exit_low_power_modes();
            } else {
                return; // Stay in low-power mode
            }
        }

        // Delegate to base 6502 implementation
        self.base.update();

        // 65C02-specific processing would go here
        self.execute_65c02_instruction();
    }

    fn run(&mut self) {
        self.base.run();
    }

    fn stop(&mut self) {
        self.base.stop();
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}

// 65C02-specific enhancements
impl WDC65C02 {
    pub fn get_base_cpu(&self) -> &MOS6502 {
        &self.base
    }

    pub fn get_base_cpu_mut(&mut self) -> &mut MOS6502 {
        &mut self.base
    }

    // Additional 65C02 functionality can be added here
    // - New addressing modes
    // - Additional instructions (STZ, BRA, etc.)
    // - Improved interrupt handling
    // - Low-power mode control
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_65c02_creation() {
        let cpu = WDC65C02::new("CPU_65C02".to_string());
        assert_eq!(cpu.name(), "CPU_65C02");
        assert!(!cpu.is_running());
    }

    #[test]
    fn test_65c02_low_power_modes() {
        let mut cpu = WDC65C02::new("CPU_65C02".to_string());

        assert!(!cpu.is_in_stop_mode());
        assert!(!cpu.is_in_wait_mode());

        cpu.enter_stop_mode();
        assert!(cpu.is_in_stop_mode());
        assert!(!cpu.is_in_wait_mode());

        cpu.enter_wait_mode();
        assert!(!cpu.is_in_stop_mode());
        assert!(cpu.is_in_wait_mode());

        cpu.exit_low_power_modes();
        assert!(!cpu.is_in_stop_mode());
        assert!(!cpu.is_in_wait_mode());
    }

    #[test]
    fn test_65c02_inheritance() {
        let mut cpu = WDC65C02::new("CPU_65C02".to_string());

        // Verify that 65C02 inherits 6502 functionality
        cpu.get_base_cpu_mut().set_program_counter(0x1000);
        assert_eq!(cpu.get_base_cpu().get_program_counter(), 0x1000);

        assert_eq!(cpu.get_base_cpu().get_accumulator(), 0);
        assert_eq!(cpu.get_base_cpu().get_x_register(), 0);
    }
}
