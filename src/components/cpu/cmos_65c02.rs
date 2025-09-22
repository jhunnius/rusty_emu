use std::sync::Arc;
use crate::component::Component;
use crate::pin::Pin;
use crate::components::cpu::mos_6502::MOS6502;

pub struct CMOS65C02 {
    pub(crate) base_6502: MOS6502,
    // 65C02-specific state
    stopped: bool,
    waiting: bool,
}
impl CMOS65C02 {
    pub fn new(name: String) -> Self {
        let base_6502 = MOS6502::new(name);

        Self {
            base_6502,
            stopped: false,
            waiting: false,
        }
    }

    // Inherit methods from MOS6502
    pub fn get_register_a(&self) -> u8 {
        self.base_6502.get_register_a()
    }

    pub fn set_register_a(&mut self, value: u8) {
        self.base_6502.set_register_a(value)
    }

    // 65C02-specific instructions
    pub fn bra(&mut self, address: u16) {
        // Branch always - new in 65C02
        self.base_6502.registers.pc = address;
    }

    pub fn phx(&mut self) {
        // Push X - new in 65C02
        // Stack push implementation would go here
        let x = self.base_6502.registers.x;
        println!("PHX: Pushing X register ({:02X}) to stack", x);
    }

    pub fn ply(&mut self) {
        // Pull Y - new in 65C02
        // Stack pull implementation would go here
        println!("PLY: Pulling value from stack to Y register");
    }

    pub fn stp(&mut self) {
        // Stop instruction - new in 65C02
        self.stopped = true;
        println!("STP: Processor stopped");
    }

    pub fn wai(&mut self) {
        // Wait for interrupt - new in 65C02
        self.waiting = true;
        println!("WAI: Processor waiting for interrupt");
    }

    // Enhanced addressing modes or modified instructions
    pub fn jmp_indirect(&mut self, address: u16) {
        // 65C02 fixes the JMP indirect bug
        // Implementation would go here
        println!("JMP (indirect) to {:04X}", address);
        self.base_6502.registers.pc = address;
    }

    // Additional 65C02-specific methods
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting
    }
}
// Implement Component by delegating to the base 6502
impl Component for CMOS65C02 {
    fn name(&self) -> &str {
        self.base_6502.name()
    }
    fn pins(&self) -> &std::collections::HashMap<String, Arc<Pin>> {
        self.base_6502.pins()
    }
    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.base_6502.get_pin(name)
    }
    fn update(&mut self) -> Result<(), String> {
        if self.stopped {
            // Don't execute instructions if stopped
            return Ok(());
        }

        // Delegate to base implementation
        self.base_6502.update()
    }
    fn run(&mut self) {
        self.base_6502.run()
    }
    fn stop(&mut self) {
        self.base_6502.stop()
    }
}