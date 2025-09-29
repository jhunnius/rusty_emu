//! Common functionality for Intel 400x series chips
//!
//! This module provides shared timing, clock handling, and bus operations
//! used across all Intel 400x family components (4001, 4002, 4003, 4004).

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Memory operation state machine states (shared across all 400x chips)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryState {
    Idle,         // No memory operation in progress
    AddressPhase, // Currently latching address nibbles
    WaitLatency,  // Address latched, waiting for access time
    DriveData,    // Latency elapsed, driving data on bus
}

/// Common timing constants for Intel 400x series
pub struct TimingConstants;

impl TimingConstants {
    pub const DEFAULT_ACCESS_TIME: Duration = Duration::from_nanos(500); // 500ns default
    pub const FAST_ACCESS_TIME: Duration = Duration::from_nanos(200); // 200ns for shift registers
    pub const ADDRESS_SETUP: Duration = Duration::from_nanos(100); // Address setup time
    pub const DATA_VALID: Duration = Duration::from_nanos(200); // Data valid delay
}

/// Common clock edge detection and timing functionality
pub trait Intel400xClockHandling {
    fn get_base(&self) -> &BaseComponent;

    /// Read the two-phase clock pins from CPU
    /// Returns: (PHI1_value, PHI2_value)
    fn read_clock_pins(&self) -> (PinValue, PinValue) {
        let phi1 = self.get_clock_pin("PHI1");
        let phi2 = self.get_clock_pin("PHI2");
        (phi1, phi2)
    }

    /// Get a clock pin value safely
    fn get_clock_pin(&self, name: &str) -> PinValue {
        if let Ok(pin) = self.get_base().get_pin(name) {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read()
            } else {
                PinValue::Low
            }
        } else {
            PinValue::Low
        }
    }

    /// Check for Φ1 rising edge
    fn is_phi1_rising_edge(&self, prev_phi1: PinValue) -> bool {
        let (phi1, _) = self.read_clock_pins();
        phi1 == PinValue::High && prev_phi1 == PinValue::Low
    }

    /// Check for Φ1 falling edge
    fn is_phi1_falling_edge(&self, prev_phi1: PinValue) -> bool {
        let (phi1, _) = self.read_clock_pins();
        phi1 == PinValue::Low && prev_phi1 == PinValue::High
    }

    /// Check for Φ2 rising edge
    fn is_phi2_rising_edge(&self, prev_phi2: PinValue) -> bool {
        let (_, phi2) = self.read_clock_pins();
        phi2 == PinValue::High && prev_phi2 == PinValue::Low
    }

    /// Check for Φ2 falling edge
    fn is_phi2_falling_edge(&self, prev_phi2: PinValue) -> bool {
        let (_, phi2) = self.read_clock_pins();
        phi2 == PinValue::Low && prev_phi2 == PinValue::High
    }

    /// Update clock state tracking for edge detection
    fn update_clock_states(&self, prev_phi1: &mut PinValue, prev_phi2: &mut PinValue) {
        let (phi1, phi2) = self.read_clock_pins();
        *prev_phi1 = phi1;
        *prev_phi2 = phi2;
    }
}

/// Common data bus operations
pub trait Intel400xDataBus {
    fn get_base(&self) -> &BaseComponent;

    /// Read the 4-bit data bus from D0-D3 pins
    /// Returns: 4-bit value from data bus pins
    fn read_data_bus(&self) -> u8 {
        let mut data = 0;

        for i in 0..4 {
            if let Ok(pin) = self.get_base().get_pin(&format!("D{}", i)) {
                if let Ok(pin_guard) = pin.lock() {
                    if pin_guard.read() == PinValue::High {
                        data |= 1 << i;
                    }
                }
            }
        }

        data & 0x0F
    }

    /// Drive the 4-bit data bus with the specified value
    /// Parameters: data - 4-bit value to drive on D0-D3 pins
    fn write_data_bus(&self, data: u8) {
        let nibble = data & 0x0F;

        for i in 0..4 {
            if let Ok(pin) = self.get_base().get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    let bit_value = (nibble >> i) & 1;
                    let pin_value = if bit_value == 1 {
                        PinValue::High
                    } else {
                        PinValue::Low
                    };
                    pin_guard.set_driver(
                        Some(format!("{}_DATA", self.get_base().get_name())),
                        pin_value,
                    );
                }
            }
        }
    }

    /// Set data bus to high-impedance state to avoid bus contention
    fn tri_state_data_bus(&self) {
        for i in 0..4 {
            if let Ok(pin) = self.get_base().get_pin(&format!("D{}", i)) {
                if let Ok(mut pin_guard) = pin.lock() {
                    pin_guard.set_driver(
                        Some(format!("{}_DATA", self.get_base().get_name())),
                        PinValue::HighZ,
                    );
                }
            }
        }
    }
}

/// Common address handling functionality
pub trait Intel400xAddressHandling {
    fn get_base(&self) -> &BaseComponent;

    /// Assemble complete 8-bit address from high and low nibbles
    /// Hardware: Intel 4004 provides address in two 4-bit phases
    /// Format: (high_nibble << 4) | low_nibble
    fn assemble_full_address(
        &self,
        high_nibble: Option<u8>,
        low_nibble: Option<u8>,
    ) -> Option<u16> {
        if let (Some(high), Some(low)) = (high_nibble, low_nibble) {
            Some(((high as u16) << 4) | (low as u16))
        } else {
            None
        }
    }

    /// Handle address nibble latching during address phase
    /// Returns: (high_nibble, low_nibble, address_ready)
    fn handle_address_latching(
        &self,
        nibble: u8,
        address_high_nibble: &mut Option<u8>,
        address_low_nibble: &mut Option<u8>,
        full_address_ready: &mut bool,
        address_latch_time: &mut Option<Instant>,
        _access_time: Duration,
    ) {
        if address_high_nibble.is_none() {
            // First cycle: latch high nibble (bits 7-4)
            *address_high_nibble = Some(nibble);
        } else if address_low_nibble.is_none() {
            // Second cycle: latch low nibble (bits 3-0) and transition to latency wait
            *address_low_nibble = Some(nibble);

            // Try to assemble full address
            if let Some(_address) =
                self.assemble_full_address(*address_high_nibble, *address_low_nibble)
            {
                *full_address_ready = true;
                *address_latch_time = Some(Instant::now());

                // Clear nibble storage for next address
                *address_high_nibble = None;
                *address_low_nibble = None;
            }
        }
    }

    /// Handle latency timing during wait state
    /// Returns: true if latency has elapsed
    fn handle_latency_wait(
        &self,
        address_latch_time: &Option<Instant>,
        access_time: Duration,
    ) -> bool {
        if let Some(latch_time) = address_latch_time {
            latch_time.elapsed() >= access_time
        } else {
            false
        }
    }
}

/// Common control pin reading functionality
pub trait Intel400xControlPins {
    fn get_base(&self) -> &BaseComponent;

    /// Read SYNC pin state
    fn read_sync_pin(&self) -> bool {
        if let Ok(pin) = self.get_base().get_pin("SYNC") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Read CM-ROM pin state
    fn read_cm_rom_pin(&self) -> bool {
        if let Ok(pin) = self.get_base().get_pin("CM") {
            // Note: CM pin name varies by component
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Read RESET pin state
    fn read_reset_pin(&self) -> bool {
        if let Ok(pin) = self.get_base().get_pin("RESET") {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Common reset handling functionality
pub trait Intel400xResetHandling {
    fn get_base(&self) -> &BaseComponent;

    /// Handle system reset signal
    /// Hardware: RESET pin clears all internal state and tri-states outputs
    fn handle_reset(&mut self, reset_pin_name: &str) -> bool {
        let reset = if let Ok(pin) = self.get_base().get_pin(reset_pin_name) {
            if let Ok(pin_guard) = pin.lock() {
                pin_guard.read() == PinValue::High
            } else {
                false
            }
        } else {
            false
        };

        if reset {
            self.perform_reset();
        }

        reset
    }

    /// Perform the actual reset operations (to be implemented by each chip)
    fn perform_reset(&mut self);
}

/// Common timing state machine functionality
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimingState {
    Idle,         // No operation in progress
    AddressPhase, // Currently latching address nibbles
    WaitLatency,  // Address latched, waiting for access time
    DriveData,    // Latency elapsed, driving data on bus
}

// Conversion implementations for compatibility with existing code
impl From<MemoryState> for TimingState {
    fn from(state: MemoryState) -> Self {
        match state {
            MemoryState::Idle => TimingState::Idle,
            MemoryState::AddressPhase => TimingState::AddressPhase,
            MemoryState::WaitLatency => TimingState::WaitLatency,
            MemoryState::DriveData => TimingState::DriveData,
        }
    }
}

impl From<TimingState> for MemoryState {
    fn from(state: TimingState) -> Self {
        match state {
            TimingState::Idle => MemoryState::Idle,
            TimingState::AddressPhase => MemoryState::AddressPhase,
            TimingState::WaitLatency => MemoryState::WaitLatency,
            TimingState::DriveData => MemoryState::DriveData,
        }
    }
}

// Forward declaration for RamState enum (defined in individual chip modules)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RamState {
    Idle,
    AddressPhase,
    WaitLatency,
    ReadData,
    WriteData,
    OutputPort,
}

// Conversion implementations for compatibility with existing code
impl From<RamState> for TimingState {
    fn from(state: RamState) -> Self {
        match state {
            RamState::Idle => TimingState::Idle,
            RamState::AddressPhase => TimingState::AddressPhase,
            RamState::WaitLatency => TimingState::WaitLatency,
            RamState::ReadData => TimingState::DriveData,
            RamState::WriteData => TimingState::DriveData,
            RamState::OutputPort => TimingState::DriveData,
        }
    }
}

impl From<TimingState> for RamState {
    fn from(state: TimingState) -> Self {
        match state {
            TimingState::Idle => RamState::Idle,
            TimingState::AddressPhase => RamState::AddressPhase,
            TimingState::WaitLatency => RamState::WaitLatency,
            TimingState::DriveData => RamState::ReadData, // Default to ReadData for DriveData
        }
    }
}

impl TimingState {
    pub fn is_idle(&self) -> bool {
        matches!(self, TimingState::Idle)
    }

    pub fn is_address_phase(&self) -> bool {
        matches!(self, TimingState::AddressPhase)
    }

    pub fn is_waiting_latency(&self) -> bool {
        matches!(self, TimingState::WaitLatency)
    }

    pub fn is_driving_data(&self) -> bool {
        matches!(self, TimingState::DriveData)
    }
}

/// Common timing state machine trait
pub trait Intel400xTimingState {
    fn get_timing_state(&self) -> TimingState;
    fn set_timing_state(&mut self, state: TimingState);
    fn get_address_latch_time(&self) -> Option<Instant>;
    fn set_address_latch_time(&mut self, time: Option<Instant>);
    fn get_full_address_ready(&self) -> bool;
    fn set_full_address_ready(&mut self, ready: bool);
    fn get_address_high_nibble(&self) -> Option<u8>;
    fn set_address_high_nibble(&mut self, nibble: Option<u8>);
    fn get_address_low_nibble(&self) -> Option<u8>;
    fn set_address_low_nibble(&mut self, nibble: Option<u8>);
    fn get_access_time(&self) -> Duration;
}

/// Utility functions for common operations
pub mod utils {
    use super::*;

    /// Create a unique driver name for a component
    pub fn create_driver_name(component_name: &str, suffix: &str) -> String {
        format!("{}_{}", component_name, suffix)
    }

    /// Check if a pin value represents a logical high
    pub fn is_pin_high(pin: &Arc<Mutex<Pin>>) -> bool {
        if let Ok(pin_guard) = pin.lock() {
            pin_guard.read() == PinValue::High
        } else {
            false
        }
    }

    /// Check if a pin value represents high impedance
    pub fn is_pin_high_z(pin: &Arc<Mutex<Pin>>) -> bool {
        if let Ok(pin_guard) = pin.lock() {
            pin_guard.read() == PinValue::HighZ
        } else {
            false
        }
    }

    /// Safely read a pin value with fallback
    pub fn read_pin_safe(pin: &Arc<Mutex<Pin>>, default: PinValue) -> PinValue {
        if let Ok(pin_guard) = pin.lock() {
            pin_guard.read()
        } else {
            default
        }
    }

    /// Safely set a pin driver with error handling
    pub fn set_pin_driver_safe(pin: &Arc<Mutex<Pin>>, driver_name: String, value: PinValue) {
        if let Ok(mut pin_guard) = pin.lock() {
            pin_guard.set_driver(Some(driver_name), value);
        }
    }
}

// Re-export commonly used items for convenience
pub use utils::*;
