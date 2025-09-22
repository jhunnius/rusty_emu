use std::sync::Arc;
use crate::{Component, Pin, PinValue};

#[derive(Clone)]
pub struct GenericRam<const SIZE: usize, const DATA_WIDTH: usize, const ADDR_WIDTH: usize> {
    base: crate::component::BaseComponent,
    pub(crate) memory: Vec<u64>,
    base_address: u64,
    address_mask: u64,
    read_delay: u32,    // Read access time in cycles
    write_delay: u32,   // Write access time in cycles
    current_operation: Option<RamOperation>,
    operation_cycles: u32,
    output_buffer: Option<u64>,
}

#[derive(Clone, Copy, PartialEq)]
enum RamOperation {
    Read(u64),  // address
    Write(u64, u64), // address, data
}

impl<const SIZE: usize, const DATA_WIDTH: usize, const ADDR_WIDTH: usize>
GenericRam<SIZE, DATA_WIDTH, ADDR_WIDTH>
{
    pub fn new(name: String, base_address: u64, read_delay: u32, write_delay: u32) -> Self {
        let mut base = crate::component::BaseComponent::new(name);

        // Add address pins
        for i in 0..ADDR_WIDTH {
            base.add_pin(format!("addr_{}", i), PinValue::Low);
        }

        // Add data pins
        for i in 0..DATA_WIDTH {
            base.add_pin(format!("data_{}", i), PinValue::HighZ);
        }

        // Add control pins
        base.add_pin("cs".to_string(), PinValue::High);    // Chip Select (active low)
        base.add_pin("we".to_string(), PinValue::High);    // Write Enable (active low)
        base.add_pin("oe".to_string(), PinValue::High);    // Output Enable (active low)
        base.add_pin("clk".to_string(), PinValue::Low);    // Clock for synchronization

        // Calculate address mask based on size
        let address_mask = ((1 << ADDR_WIDTH) - 1) as u64;

        Self {
            base,
            memory: vec![0; SIZE],
            base_address,
            address_mask,
            read_delay,
            write_delay,
            current_operation: None,
            operation_cycles: 0,
            output_buffer: None,
        }
    }

    fn read_address_bus(&self) -> u64 {
        let mut address = 0;
        for i in 0..ADDR_WIDTH {
            if let Some(pin) = self.base.get_pin(&format!("addr_{}", i)) {
                if pin.read() == PinValue::High {
                    address |= 1 << i;
                }
            }
        }
        address
    }

    fn read_data_bus(&self) -> u64 {
        let mut data = 0;
        for i in 0..DATA_WIDTH {
            if let Some(pin) = self.base.get_pin(&format!("data_{}", i)) {
                if pin.read() == PinValue::High {
                    data |= 1 << i;
                }
            }
        }
        data
    }

    fn write_data_bus(&self, data: u64) {
        for i in 0..DATA_WIDTH {
            if let Some(pin) = self.base.get_pin(&format!("data_{}", i)) {
                let bit = (data >> i) & 1;
                let value = if bit == 1 { PinValue::High } else { PinValue::Low };
                pin.write(value, true);
            }
        }
    }

    fn set_data_bus_high_z(&self) {
        for i in 0..DATA_WIDTH {
            if let Some(pin) = self.base.get_pin(&format!("data_{}", i)) {
                pin.write(PinValue::HighZ, false);
            }
        }
    }

    fn is_selected(&self) -> bool {
        if let Some(cs_pin) = self.base.get_pin("cs") {
            cs_pin.read() == PinValue::Low // Active low
        } else {
            false
        }
    }

    fn is_write_enabled(&self) -> bool {
        if let Some(we_pin) = self.base.get_pin("we") {
            we_pin.read() == PinValue::Low // Active low
        } else {
            false
        }
    }

    fn is_output_enabled(&self) -> bool {
        if let Some(oe_pin) = self.base.get_pin("oe") {
            oe_pin.read() == PinValue::Low // Active low
        } else {
            false
        }
    }

    fn is_clock_rising_edge(&self, last_clock: &mut PinValue) -> bool {
        if let Some(clk_pin) = self.base.get_pin("clk") {
            let current_clock = clk_pin.read();
            let rising_edge = *last_clock == PinValue::Low && current_clock == PinValue::High;
            *last_clock = current_clock;
            rising_edge
        } else {
            false
        }
    }

    pub(crate) fn decode_address(&self, bus_address: u64) -> Option<usize> {
        // Check if address is in our range
        let relative_address = bus_address & self.address_mask;

        if (bus_address & !self.address_mask) == self.base_address {
            // Address matches our base + mask
            let index = relative_address as usize;
            if index < SIZE {
                Some(index)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn start_read_operation(&mut self, address: u64) {
        if let Some(index) = self.decode_address(address) {
            self.current_operation = Some(RamOperation::Read(address));
            self.operation_cycles = 0;
            println!("RAM {}: Starting read from address {:X} (index {})",
                     self.base.name, address, index);
        }
    }

    fn start_write_operation(&mut self, address: u64, data: u64) {
        if let Some(index) = self.decode_address(address) {
            self.current_operation = Some(RamOperation::Write(address, data));
            self.operation_cycles = 0;
            println!("RAM {}: Starting write {:X} to address {:X} (index {})",
                     self.base.name, data, address, index);
        }
    }

    fn complete_read_operation(&mut self, address: u64) {
        if let Some(index) = self.decode_address(address) {
            let data = self.memory[index];
            self.output_buffer = Some(data);
            println!("RAM {}: Read complete - data {:X} from address {:X}",
                     self.base.name, data, address);
        }
        self.current_operation = None;
    }

    fn complete_write_operation(&mut self, address: u64, data: u64) {
        if let Some(index) = self.decode_address(address) {
            self.memory[index] = data & ((1 << DATA_WIDTH) - 1); // Mask to data width
            println!("RAM {}: Write complete - data {:X} to address {:X}",
                     self.base.name, data, address);
        }
        self.current_operation = None;
    }

    fn update_operation(&mut self) {
        if let Some(operation) = self.current_operation {
            self.operation_cycles += 1;

            match operation {
                RamOperation::Read(addr) => {
                    if self.operation_cycles >= self.read_delay {
                        self.complete_read_operation(addr);
                    }
                }
                RamOperation::Write(addr, data) => {
                    if self.operation_cycles >= self.write_delay {
                        self.complete_write_operation(addr, data);
                    }
                }
            }
        }
    }

    fn handle_bus_signals(&mut self, bus_address: u64, last_clock: &mut PinValue) {
        if !self.is_selected() {
            // Not selected - high impedance and clear any pending operations
            self.set_data_bus_high_z();
            self.current_operation = None;
            return;
        }

        let clock_rising_edge = self.is_clock_rising_edge(last_clock);

        if clock_rising_edge {
            // On clock edge, check what operation is requested
            if self.is_write_enabled() {
                // Write operation
                let data = self.read_data_bus();
                self.start_write_operation(bus_address, data);
            } else if self.is_output_enabled() {
                // Read operation
                self.start_read_operation(bus_address);
            }
        }

        // Handle ongoing operations
        self.update_operation();

        // Output data if we have it and output is enabled
        if self.is_output_enabled() {
            if let Some(data) = self.output_buffer {
                self.write_data_bus(data);
            } else {
                self.set_data_bus_high_z();
            }
        } else {
            self.set_data_bus_high_z();
        }
    }
}

impl<const SIZE: usize, const DATA_WIDTH: usize, const ADDR_WIDTH: usize>
crate::component::Component for GenericRam<SIZE, DATA_WIDTH, ADDR_WIDTH>
{
    fn name(&self) -> &str {
        self.base.name()
    }

    fn pins(&self) -> &std::collections::HashMap<String, Arc<Pin>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.base.get_pin(name)
    }

    fn update(&mut self) -> Result<(), String> {
        let bus_address = self.read_address_bus();
        let mut last_clock = PinValue::Low;

        self.handle_bus_signals(bus_address, &mut last_clock);
        Ok(())
    }

    fn run(&mut self) {
        self.base.run();
    }

    fn stop(&mut self) {
        self.base.stop();
    }
}

// BusDevice implementation
impl<const SIZE: usize, const DATA_WIDTH: usize, const ADDR_WIDTH: usize>
crate::bus::BusDevice for GenericRam<SIZE, DATA_WIDTH, ADDR_WIDTH>
{
    fn update(&mut self) {
        // Delegate to component update
        let _ = Component::update(self);
    }

    fn connect_to_bus(&mut self, bus_pins: Vec<Arc<Pin>>) {
        // Connect to bus pins - in a real system, this would connect address, data, and control pins
        // This is simplified for the example
    }

    fn get_address_range(&self) -> Option<std::ops::Range<u64>> {
        Some(self.base_address..self.base_address + SIZE as u64)
    }

    fn get_name(&self) -> &str {
        todo!()
    }
}