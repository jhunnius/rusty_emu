//! Integration tests for Intel 400x common functionality
//!
//! These tests verify that concrete implementations correctly use
//! the intel_400x traits and that the common functionality works
//! as expected in real usage scenarios.

use rusty_emu::component::Component;
use rusty_emu::components::common::intel_400x::*;
use rusty_emu::components::memory::intel_4001::Intel4001;
use std::time::Duration;

#[cfg(test)]
mod intel_4001_integration_tests {
    use super::*;

    #[test]
    fn test_intel4001_uses_common_traits() {
        let rom = Intel4001::new("ROM_4001".to_string());

        // Verify that Intel4001 implements all the common traits
        // These should compile without errors if the trait implementations are correct

        // Test Intel400xClockHandling trait
        let _: &dyn Intel400xClockHandling = &rom;

        // Test Intel400xDataBus trait
        let _: &dyn Intel400xDataBus = &rom;

        // Test Intel400xAddressHandling trait
        let _: &dyn Intel400xAddressHandling = &rom;

        // Test Intel400xControlPins trait
        let _: &dyn Intel400xControlPins = &rom;

        // Test Intel400xResetHandling trait
        let _: &dyn Intel400xResetHandling = &rom;

        // Test Intel400xTimingState trait
        let _: &dyn Intel400xTimingState = &rom;

        // Verify the component has the expected name
        assert_eq!(rom.name(), "ROM_4001");
    }

    #[test]
    fn test_intel4001_timing_state_integration() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Test that timing state works correctly with the trait
        let initial_state: TimingState = rom.get_timing_state();
        assert_eq!(initial_state, TimingState::Idle);

        // Test state changes through the trait
        rom.set_timing_state(TimingState::AddressPhase);
        let address_state: TimingState = rom.get_timing_state();
        assert_eq!(address_state, TimingState::AddressPhase);

        rom.set_timing_state(TimingState::WaitLatency);
        let wait_state: TimingState = rom.get_timing_state();
        assert_eq!(wait_state, TimingState::WaitLatency);

        rom.set_timing_state(TimingState::DriveData);
        let drive_state: TimingState = rom.get_timing_state();
        assert_eq!(drive_state, TimingState::DriveData);
    }

    #[test]
    fn test_intel4001_address_handling_integration() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Test address handling through the trait
        rom.set_address_high_nibble(Some(0x12));
        rom.set_address_low_nibble(Some(0x34));
        rom.set_full_address_ready(true);

        // Verify the values are accessible through the trait
        assert_eq!(rom.get_address_high_nibble(), Some(0x12));
        assert_eq!(rom.get_address_low_nibble(), Some(0x34));
        assert_eq!(rom.get_full_address_ready(), true);

        // Test access time configuration
        assert_eq!(rom.get_access_time(), 1);

        // Test address latch time handling
        let test_time = std::time::Instant::now();
        rom.set_address_latch_time(Some(test_time));
        assert_eq!(rom.get_address_latch_time(), Some(test_time));
    }

    #[test]
    fn test_common_functionality_with_real_component() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Test that common functionality works with the real component
        let test_data = vec![0xAB, 0xCD];
        rom.load_rom_data(test_data, 0).unwrap();

        // Test direct memory access (bypassing the pin interface)
        assert_eq!(rom.read_rom(0x00).unwrap(), 0xAB);
        assert_eq!(rom.read_rom(0x01).unwrap(), 0xCD);

        // Test that the component maintains its state correctly
        assert_eq!(rom.get_rom_size(), 256);
        assert_eq!(rom.get_access_time(), 1);
    }

    #[test]
    fn test_timing_constants_usage() {
        // Test that timing constants are used appropriately
        let fast_rom = Intel4001::new_with_access_time(
            "FAST_ROM".to_string(),
            TimingConstants::FAST_ACCESS_TIME.as_nanos() as u64,
        );
        let default_rom = Intel4001::new_with_access_time(
            "DEFAULT_ROM".to_string(),
            TimingConstants::DEFAULT_ACCESS_TIME.as_nanos() as u64,
        );

        assert_eq!(
            fast_rom.get_access_time(),
            TimingConstants::FAST_ACCESS_TIME.as_nanos() as u64
        );
        assert_eq!(
            default_rom.get_access_time(),
            TimingConstants::DEFAULT_ACCESS_TIME.as_nanos() as u64
        );

        assert!(TimingConstants::FAST_ACCESS_TIME < TimingConstants::DEFAULT_ACCESS_TIME);
        assert!(TimingConstants::ADDRESS_SETUP > Duration::from_nanos(0));
        assert!(TimingConstants::DATA_VALID > Duration::from_nanos(0));
    }

    #[test]
    fn test_state_machine_integration() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Test state machine progression through the trait interface
        assert_eq!(rom.get_timing_state(), TimingState::Idle);

        // Progress through states
        rom.set_timing_state(TimingState::AddressPhase);
        assert_eq!(rom.get_timing_state(), TimingState::AddressPhase);

        rom.set_timing_state(TimingState::WaitLatency);
        assert_eq!(rom.get_timing_state(), TimingState::WaitLatency);

        rom.set_timing_state(TimingState::DriveData);
        assert_eq!(rom.get_timing_state(), TimingState::DriveData);

        // Return to idle
        rom.set_timing_state(TimingState::Idle);
        assert_eq!(rom.get_timing_state(), TimingState::Idle);
    }

    #[test]
    fn test_address_latching_integration() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Test address latching through the trait
        rom.set_address_high_nibble(Some(0x0F));
        rom.set_address_low_nibble(Some(0x23));
        rom.set_full_address_ready(true);

        // Verify the address was assembled correctly
        assert_eq!(rom.get_address_high_nibble(), Some(0x0F));
        assert_eq!(rom.get_address_low_nibble(), Some(0x23));
        assert_eq!(rom.get_full_address_ready(), true);

        // Test clearing the address
        rom.set_address_high_nibble(None);
        rom.set_address_low_nibble(None);
        rom.set_full_address_ready(false);

        assert_eq!(rom.get_address_high_nibble(), None);
        assert_eq!(rom.get_address_low_nibble(), None);
        assert_eq!(rom.get_full_address_ready(), false);
    }

    #[test]
    fn test_multiple_component_interaction() {
        // Test how multiple components using the same traits interact
        let rom1 = Intel4001::new_with_access_time("ROM1".to_string(), 1);
        let rom2 = Intel4001::new_with_access_time("ROM2".to_string(), 2);

        // Both should implement the same traits
        let _: &dyn Intel400xClockHandling = &rom1;
        let _: &dyn Intel400xClockHandling = &rom2;

        let _: &dyn Intel400xDataBus = &rom1;
        let _: &dyn Intel400xDataBus = &rom2;

        // They should have different access times
        assert_eq!(rom1.get_access_time(), 1);
        assert_eq!(rom2.get_access_time(), 2);

        // Both should start in the same initial state
        assert_eq!(rom1.get_timing_state(), TimingState::Idle);
        assert_eq!(rom2.get_timing_state(), TimingState::Idle);
    }

    #[test]
    fn test_performance_characteristics() {
        // Test that the common functionality maintains expected performance characteristics
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Test that access time is correctly configured
        assert_eq!(rom.get_access_time(), 1);

        // Test that the component can handle rapid state changes
        for _ in 0..100 {
            rom.set_timing_state(TimingState::Idle);
            rom.set_timing_state(TimingState::AddressPhase);
            rom.set_timing_state(TimingState::WaitLatency);
            rom.set_timing_state(TimingState::DriveData);
        }

        // Should end up in the expected final state
        assert_eq!(rom.get_timing_state(), TimingState::DriveData);
    }

    #[test]
    fn test_memory_operation_simulation() {
        let mut rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Load test data
        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        rom.load_rom_data(test_data.clone(), 0).unwrap();

        // Test that the ROM correctly stores and retrieves data
        for (i, &expected) in test_data.iter().enumerate() {
            assert_eq!(rom.read_rom(i as u8).unwrap(), expected);
        }

        // Test that the component maintains correct metadata
        assert_eq!(rom.get_rom_size(), 256);
        assert_eq!(rom.get_access_time(), 1);
        assert_eq!(rom.name(), "ROM_4001");
    }

    #[test]
    fn test_trait_consistency_across_implementations() {
        // Test that different components implement the traits consistently
        let rom = Intel4001::new_with_access_time("ROM_4001".to_string(), 1);

        // Test that all trait methods are available and work consistently
        let base_component: &dyn Component = &rom;
        assert_eq!(base_component.name(), "ROM_4001");

        // Test that timing state trait works
        let timing_state: &dyn Intel400xTimingState = &rom;
        assert_eq!(timing_state.get_timing_state(), TimingState::Idle);
        assert_eq!(timing_state.get_access_time(), Duration::from_nanos(1));

        // Test that clock handling trait works
        let clock_handler: &dyn Intel400xClockHandling = &rom;
        let base_for_clock = clock_handler.get_base();
        assert_eq!(base_for_clock.get_name(), "ROM_4001");

        // Test that data bus trait works
        let data_bus: &dyn Intel400xDataBus = &rom;
        let base_for_data = data_bus.get_base();
        assert_eq!(base_for_data.get_name(), "ROM_4001");

        // Test that address handling trait works
        let address_handler: &dyn Intel400xAddressHandling = &rom;
        let base_for_address = address_handler.get_base();
        assert_eq!(base_for_address.get_name(), "ROM_4001");

        // Test that control pins trait works
        let control_pins: &dyn Intel400xControlPins = &rom;
        let base_for_control = control_pins.get_base();
        assert_eq!(base_for_control.get_name(), "ROM_4001");

        // Test that reset handling trait works
        let reset_handler: &dyn Intel400xResetHandling = &rom;
        let base_for_reset = reset_handler.get_base();
        assert_eq!(base_for_reset.get_name(), "ROM_4001");
    }

    #[test]
    fn test_cross_component_compatibility() {
        // Test that components using the same traits are compatible
        let rom1 = Intel4001::new_with_access_time("ROM1".to_string(), 1);
        let rom2 = Intel4001::new_with_access_time("ROM2".to_string(), 2);

        // Both should implement the same interface
        fn use_intel400x_component(component: &dyn Intel400xTimingState) {
            let _state = component.get_timing_state();
            let _access_time = component.get_access_time();
        }

        use_intel400x_component(&rom1);
        use_intel400x_component(&rom2);

        // Both should have the same initial state
        assert_eq!(rom1.get_timing_state(), rom2.get_timing_state());
        assert_eq!(rom1.get_timing_state(), TimingState::Idle);

        // But different access times
        assert_ne!(rom1.get_access_time(), rom2.get_access_time());
    }
}

#[cfg(test)]
mod cross_component_integration_tests {
    use super::*;

    #[test]
    fn test_multiple_intel4001_components() {
        // Test that multiple Intel4001 components work together
        let mut rom1 = Intel4001::new_with_access_time("ROM1".to_string(), 1);
        let mut rom2 = Intel4001::new_with_access_time("ROM2".to_string(), 2);

        // Load different data into each
        rom1.load_rom_data(vec![0x11, 0x22], 0).unwrap();
        rom2.load_rom_data(vec![0x33, 0x44], 0).unwrap();

        // Both should implement the same traits
        assert_eq!(rom1.read_rom(0x00).unwrap(), 0x11);
        assert_eq!(rom2.read_rom(0x00).unwrap(), 0x33);

        // Both should have the same interface
        assert_eq!(rom1.get_timing_state(), TimingState::Idle);
        assert_eq!(rom2.get_timing_state(), TimingState::Idle);

        // Test state manipulation on both
        rom1.set_timing_state(TimingState::AddressPhase);
        rom2.set_timing_state(TimingState::AddressPhase);

        assert_eq!(rom1.get_timing_state(), TimingState::AddressPhase);
        assert_eq!(rom2.get_timing_state(), TimingState::AddressPhase);
    }

    #[test]
    fn test_timing_consistency_across_components() {
        // Test that timing behavior is consistent across different components
        let mut rom1 = Intel4001::new_with_access_time("ROM1".to_string(), 100);
        let mut rom2 = Intel4001::new_with_access_time("ROM2".to_string(), 200);

        // Both should start with the same timing state
        assert_eq!(rom1.get_timing_state(), rom2.get_timing_state());
        assert_eq!(rom1.get_timing_state(), TimingState::Idle);

        // But have different access times
        assert_eq!(rom1.get_access_time(), 100);
        assert_eq!(rom2.get_access_time(), 200);

        // Both should support the same state transitions
        rom1.set_timing_state(TimingState::AddressPhase);
        rom2.set_timing_state(TimingState::AddressPhase);

        assert_eq!(rom1.get_timing_state(), TimingState::AddressPhase);
        assert_eq!(rom2.get_timing_state(), TimingState::AddressPhase);
    }
}
