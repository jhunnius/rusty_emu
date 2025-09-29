//! Mock implementations for testing Intel 400x components
//!
//! This module provides mock implementations of pins, components, and timing
//! to enable comprehensive testing of the intel_400x traits and functionality.

use rusty_emu::component::{BaseComponent, Component};
use rusty_emu::pin::{Pin, PinValue};
use rusty_emu::components::common::intel_400x::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Mock pin implementation for testing
#[derive(Debug, Clone)]
pub struct MockPin {
    pub name: String,
    pub value: PinValue,
    pub driver: Option<(String, PinValue)>,
    pub read_count: usize,
    pub write_count: usize,
}

impl MockPin {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: PinValue::HighZ,
            driver: None,
            read_count: 0,
            write_count: 0,
        }
    }

    pub fn read(&mut self) -> PinValue {
        self.read_count += 1;
        self.value
    }

    pub fn set_driver(&mut self, driver: Option<String>, value: PinValue) {
        self.write_count += 1;
        self.driver = driver.map(|name| (name, value));
        self.value = value;
    }

    pub fn set_value(&mut self, value: PinValue) {
        self.value = value;
    }

    pub fn get_read_count(&self) -> usize {
        self.read_count
    }

    pub fn get_write_count(&self) -> usize {
        self.write_count
    }
}

/// Mock component for testing trait implementations
#[derive(Debug)]
pub struct MockIntel400xComponent {
    pub name: String,
    pub pins: HashMap<String, Arc<Mutex<MockPin>>>,
    pub timing_state: TimingState,
    pub address_latch_time: Option<Instant>,
    pub full_address_ready: bool,
    pub address_high_nibble: Option<u8>,
    pub address_low_nibble: Option<u8>,
    pub access_time: Duration,
}

impl MockIntel400xComponent {
    pub fn new(name: &str) -> Self {
        let mut pins = HashMap::new();

        // Add standard Intel 400x pins
        let clock_pins = ["PHI1", "PHI2"];
        let data_pins = ["D0", "D1", "D2", "D3"];
        let control_pins = ["SYNC", "CM", "RESET"];

        for pin_name in clock_pins.iter().chain(data_pins.iter()).chain(control_pins.iter()) {
            pins.insert(pin_name.to_string(), Arc::new(Mutex::new(MockPin::new(pin_name))));
        }

        Self {
            name: name.to_string(),
            pins,
            timing_state: TimingState::Idle,
            address_latch_time: None,
            full_address_ready: false,
            address_high_nibble: None,
            address_low_nibble: None,
            access_time: TimingConstants::DEFAULT_ACCESS_TIME,
        }
    }

    pub fn set_pin_value(&self, pin_name: &str, value: PinValue) {
        if let Some(pin) = self.pins.get(pin_name) {
            if let Ok(mut pin_guard) = pin.lock() {
                pin_guard.set_value(value);
            }
        }
    }

    pub fn get_pin_value(&self, pin_name: &str) -> Option<PinValue> {
        if let Some(pin) = self.pins.get(pin_name) {
            if let Ok(pin_guard) = pin.lock() {
                return Some(pin_guard.value);
            }
        }
        None
    }

    pub fn set_clock_values(&self, phi1: PinValue, phi2: PinValue) {
        self.set_pin_value("PHI1", phi1);
        self.set_pin_value("PHI2", phi2);
    }

    pub fn set_data_bus(&self, value: u8) {
        for i in 0..4 {
            let bit_value = (value >> i) & 1;
            let pin_value = if bit_value == 1 { PinValue::High } else { PinValue::Low };
            self.set_pin_value(&format!("D{}", i), pin_value);
        }
    }

    pub fn get_pin_read_count(&self, pin_name: &str) -> Option<usize> {
        if let Some(pin) = self.pins.get(pin_name) {
            if let Ok(pin_guard) = pin.lock() {
                return Some(pin_guard.get_read_count());
            }
        }
        None
    }

    pub fn get_pin_write_count(&self, pin_name: &str) -> Option<usize> {
        if let Some(pin) = self.pins.get(pin_name) {
            if let Ok(pin_guard) = pin.lock() {
                return Some(pin_guard.get_write_count());
            }
        }
        None
    }
}

impl Component for MockIntel400xComponent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        // Return empty map for testing - in real implementation would convert MockPins
        HashMap::new()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        // Convert our MockPin to the expected Pin type
        if let Some(_mock_pin) = self.pins.get(name) {
            // This is a simplified conversion - in a real implementation,
            // you'd need to create a proper adapter
            Err(format!("Mock pin conversion not implemented for {}", name))
        } else {
            Err(format!("Pin {} not found", name))
        }
    }

    fn update(&mut self) {
        // No-op for mock
    }

    fn run(&mut self) {
        // No-op for mock
    }

    fn stop(&mut self) {
        // No-op for mock
    }

    fn is_running(&self) -> bool {
        false
    }
}

impl Intel400xClockHandling for MockIntel400xComponent {
    fn get_base(&self) -> &BaseComponent {
        // For testing, we need to create a minimal BaseComponent
        // This is a limitation of the current test setup
        // In a real implementation, this would return a reference to an embedded BaseComponent
        unimplemented!("MockIntel400xComponent doesn't contain BaseComponent")
    }
}

impl Intel400xDataBus for MockIntel400xComponent {
    fn get_base(&self) -> &BaseComponent {
        unimplemented!("MockIntel400xComponent doesn't contain BaseComponent")
    }
}

impl Intel400xAddressHandling for MockIntel400xComponent {
    fn get_base(&self) -> &BaseComponent {
        unimplemented!("MockIntel400xComponent doesn't contain BaseComponent")
    }
}

impl Intel400xControlPins for MockIntel400xComponent {
    fn get_base(&self) -> &BaseComponent {
        unimplemented!("MockIntel400xComponent doesn't contain BaseComponent")
    }
}

impl Intel400xResetHandling for MockIntel400xComponent {
    fn get_base(&self) -> &BaseComponent {
        unimplemented!("MockIntel400xComponent doesn't contain BaseComponent")
    }

    fn perform_reset(&self) {
        // Reset implementation for testing - Note: This is a limitation
        // In a real implementation with proper BaseComponent, this would work
        // For testing purposes, we document the expected behavior
    }
}

impl Intel400xTimingState for MockIntel400xComponent {
    fn get_timing_state(&self) -> TimingState {
        self.timing_state
    }

    fn set_timing_state(&mut self, state: TimingState) {
        self.timing_state = state;
    }

    fn get_address_latch_time(&self) -> Option<Instant> {
        self.address_latch_time
    }

    fn set_address_latch_time(&mut self, time: Option<Instant>) {
        self.address_latch_time = time;
    }

    fn get_full_address_ready(&self) -> bool {
        self.full_address_ready
    }

    fn set_full_address_ready(&mut self, ready: bool) {
        self.full_address_ready = ready;
    }

    fn get_address_high_nibble(&self) -> Option<u8> {
        self.address_high_nibble
    }

    fn set_address_high_nibble(&mut self, nibble: Option<u8>) {
        self.address_high_nibble = nibble;
    }

    fn get_address_low_nibble(&self) -> Option<u8> {
        self.address_low_nibble
    }

    fn set_address_low_nibble(&mut self, nibble: Option<u8>) {
        self.address_low_nibble = nibble;
    }

    fn get_access_time(&self) -> Duration {
        self.access_time
    }
}

/// Mock time provider for deterministic testing
#[derive(Debug, Clone)]
pub struct MockTimeProvider {
    pub current_time: Instant,
    pub time_offset: Duration,
}

impl MockTimeProvider {
    pub fn new() -> Self {
        Self {
            current_time: Instant::now(),
            time_offset: Duration::from_nanos(0),
        }
    }

    pub fn advance(&mut self, duration: Duration) {
        self.time_offset += duration;
    }

    pub fn set_time(&mut self, time: Instant) {
        self.current_time = time;
    }

    pub fn now(&self) -> Instant {
        self.current_time + self.time_offset
    }

    pub fn elapsed(&self, since: Instant) -> Duration {
        self.now() - since
    }
}

/// Test helper for creating mock scenarios
pub struct MockScenario {
    pub component: MockIntel400xComponent,
    pub time_provider: MockTimeProvider,
}

impl MockScenario {
    pub fn new(name: &str) -> Self {
        Self {
            component: MockIntel400xComponent::new(name),
            time_provider: MockTimeProvider::new(),
        }
    }

    pub fn set_clock_high(&mut self) {
        self.component.set_clock_values(PinValue::High, PinValue::High);
    }

    pub fn set_clock_low(&mut self) {
        self.component.set_clock_values(PinValue::Low, PinValue::Low);
    }

    pub fn set_phi1_rising_edge(&mut self) {
        self.component.set_clock_values(PinValue::High, PinValue::Low);
    }

    pub fn set_phi1_falling_edge(&mut self) {
        self.component.set_clock_values(PinValue::Low, PinValue::Low);
    }

    pub fn set_phi2_rising_edge(&mut self) {
        self.component.set_clock_values(PinValue::Low, PinValue::High);
    }

    pub fn set_phi2_falling_edge(&mut self) {
        self.component.set_clock_values(PinValue::Low, PinValue::Low);
    }

    pub fn set_data_bus_value(&mut self, value: u8) {
        self.component.set_data_bus(value);
    }

    pub fn get_data_bus_value(&self) -> u8 {
        let mut value = 0;
        for i in 0..4 {
            if let Some(pin_value) = self.component.get_pin_value(&format!("D{}", i)) {
                if pin_value == PinValue::High {
                    value |= 1 << i;
                }
            }
        }
        value
    }

    pub fn advance_time(&mut self, duration: Duration) {
        self.time_provider.advance(duration);
    }

    pub fn set_access_time(&mut self, duration: Duration) {
        // This would need to be implemented in the actual component
        // For now, it's a placeholder
    }
}

/// Property-based testing helpers
pub mod proptest_helpers {
    use super::*;
    use proptest::prelude::*;

    pub fn arb_pin_value() -> impl Strategy<Value = PinValue> {
        prop_oneof![
            Just(PinValue::Low),
            Just(PinValue::High),
            Just(PinValue::HighZ),
        ]
    }

    pub fn arb_timing_state() -> impl Strategy<Value = TimingState> {
        prop_oneof![
            Just(TimingState::Idle),
            Just(TimingState::AddressPhase),
            Just(TimingState::WaitLatency),
            Just(TimingState::DriveData),
        ]
    }

    pub fn arb_memory_state() -> impl Strategy<Value = MemoryState> {
        prop_oneof![
            Just(MemoryState::Idle),
            Just(MemoryState::AddressPhase),
            Just(MemoryState::WaitLatency),
            Just(MemoryState::DriveData),
        ]
    }

    pub fn arb_ram_state() -> impl Strategy<Value = RamState> {
        prop_oneof![
            Just(RamState::Idle),
            Just(RamState::AddressPhase),
            Just(RamState::WaitLatency),
            Just(RamState::ReadData),
            Just(RamState::WriteData),
            Just(RamState::OutputPort),
        ]
    }

    pub fn arb_address_nibble() -> impl Strategy<Value = u8> {
        0u8..16u8
    }

    pub fn arb_data_value() -> impl Strategy<Value = u8> {
        0u8..16u8 // 4-bit values for Intel 4004
    }

    pub fn arb_duration_nanos() -> impl Strategy<Value = u64> {
        0u64..1_000_000_000u64 // Up to 1 second in nanoseconds
    }

    pub fn arb_duration() -> impl Strategy<Value = Duration> {
        (0u64..1_000_000_000u64).prop_map(Duration::from_nanos)
    }
}