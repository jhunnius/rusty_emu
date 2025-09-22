use crate::components::memory::generic_rom::GenericRom;
use crate::{Component, PinValue};
use crate::systems::intel_mcs_4::MCS4Phase;

// 4001 ROM: 256 bytes, 4-bit data width
pub type Intel4001 = GenericRom;
impl Intel4001 {
    fn update(&mut self) -> Result<(), String> {
        // Read the current bus phase from the pins
        let phase = self.detect_bus_phase();

        match phase {
            MCS4Phase::AddressLow => {
                self.latch_address_low();
            }
            MCS4Phase::AddressHigh => {
                self.latch_address_high();
                if self.is_selected() {
                    self.prepare_data_output();
                }
            }
            MCS4Phase::DataRead => {
                if self.is_selected() {
                    self.output_data(/* u8 */);
                }
            }
            _ => {
                // Put pins in high impedance
                self.set_pins_high_z();
            }
        }

        Ok(())
    }
    fn detect_bus_phase(&self) -> MCS4Phase {
        // Detect phase based on sync and other control signals
        if let Some(sync_pin) = self.base.get_pin("pin_0") {
            match sync_pin.read() {
                PinValue::High => MCS4Phase::AddressLow, // Sync high = address phase
                PinValue::Low => MCS4Phase::DataRead,    // Sync low = data phase
                _ => MCS4Phase::AddressLow,
            }
        } else {
            MCS4Phase::AddressLow
        }
    }
    fn latch_address_low(&mut self) {
        let addr_low = self.read_nibble_from_pins(9..12);
        self.latched_address = (self.latched_address & 0xFF0) | (addr_low as u16);
    }
    fn latch_address_high(&mut self) {
        let addr_high = self.read_nibble_from_pins(9..12);
        let addr_mid = self.read_nibble_from_pins(13..16);
        self.latched_address = ((addr_mid as u16) << 8) | ((addr_high as u16) << 4) | (self.latched_address & 0x00F);
    }
}