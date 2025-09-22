use crate::components::memory::generic_ram::GenericRam;

// 4002 RAM: 80 nibbles (4-bit words), 12-bit address bus
// Organized as 4 registers × 20 characters × 4 bits
pub type Intel4002 = GenericRam<80, 4, 12>;

impl Intel4002 {
    pub fn new_4002(name: String, base_address: u16) -> Self {
        // 4002 has specific timing characteristics
        let ram = Self::new(
            name,
            base_address as u64,
            2,  // 2-cycle read access
            3   // 3-cycle write access
        );

        ram
    }

    // 4002-specific methods
    pub fn write_to_register_character(&mut self, register: u8, character: u8, value: u8) -> Result<(), String> {
        if register >= 4 || character >= 20 {
            return Err("Invalid register or character index".to_string());
        }

        let address = self.calculate_address(register, character);
        let index = self.decode_address(address as u64)
            .ok_or("Address out of range".to_string())?;

        self.memory[index] = (value & 0x0F) as u64;
        Ok(())
    }

    pub fn read_from_register_character(&self, register: u8, character: u8) -> Result<u8, String> {
        if register >= 4 || character >= 20 {
            return Err("Invalid register or character index".to_string());
        }

        let address = self.calculate_address(register, character);
        let index = self.decode_address(address as u64)
            .ok_or("Address out of range".to_string())?;

        Ok(self.memory[index] as u8)
    }

    fn calculate_address(&self, register: u8, character: u8) -> u16 {
        // 4002 address calculation:
        // Each register has 20 characters, each character is 4 bits
        (register as u16 * 20) + (character as u16)
    }
}