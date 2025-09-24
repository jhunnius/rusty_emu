use crate::component::Component;
use crate::components::cpu::mos_6502::MOS6502;
use crate::components::cpu::WDC65C02;

pub trait Registers {}

pub trait CPU: Component {
    fn reset(&mut self);
    fn execute_instruction(&mut self);
    fn get_registers(&self) -> &dyn Registers;
    fn get_registers_mut(&mut self) -> &mut dyn Registers;
    fn read_memory(&self, address: u16) -> u8;
    fn write_memory(&mut self, address: u16, value: u8);
}

pub trait MOS6502Family: CPU {
    // Common 6502 family methods
    fn lda(&mut self, value: u8);
    fn sta(&mut self, address: u16);
    fn tax(&mut self);
    // More common instructions...
}

pub trait CMOS65C02Extensions: MOS6502Family {
    // 65C02-specific methods
    fn bra(&mut self, address: u16);
    fn phx(&mut self);
    fn ply(&mut self);
    fn stp(&mut self);
    fn wai(&mut self);
}

impl CPU for MOS6502 {
    fn reset(&mut self) {
        todo!()
    }

    fn execute_instruction(&mut self) {
        todo!()
    }

    fn get_registers(&self) -> &dyn Registers {
        todo!()
    }

    fn get_registers_mut(&mut self) -> &mut dyn Registers {
        todo!()
    }

    fn read_memory(&self, address: u16) -> u8 {
        todo!()
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        todo!()
    }
}

// Implement for both MOS6502 and CMOS65C02
impl MOS6502Family for MOS6502 {
    fn lda(&mut self, value: u8) {
        self.lda(value)
    }

    fn sta(&mut self, address: u16) {
        todo!()
    }

    fn tax(&mut self) {
        todo!()
    }

    // Implement other methods...
}

impl CPU for WDC65C02 {
    fn reset(&mut self) {
        todo!()
    }

    fn execute_instruction(&mut self) {
        todo!()
    }

    fn get_registers(&self) -> &dyn Registers {
        todo!()
    }

    fn get_registers_mut(&mut self) -> &mut dyn Registers {
        todo!()
    }

    fn read_memory(&self, address: u16) -> u8 {
        todo!()
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        todo!()
    }
}

impl MOS6502Family for WDC65C02 {
    fn lda(&mut self, value: u8) {
        self.base.lda(value)
    }

    fn sta(&mut self, address: u16) {
        todo!()
    }

    fn tax(&mut self) {
        todo!()
    }

    // Implement other methods...
}

impl CMOS65C02Extensions for WDC65C02 {
    fn bra(&mut self, address: u16) {
        self.bra(address)
    }

    fn phx(&mut self) {
        todo!()
    }

    fn ply(&mut self) {
        todo!()
    }

    fn stp(&mut self) {
        todo!()
    }

    fn wai(&mut self) {
        todo!()
    }

    // Implement other 65C02-specific methods...
}