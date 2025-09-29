//! Demonstration of running Intel 400x tests
//!
//! This module demonstrates how to run the comprehensive test suite
//! for the intel_400x common functionality.

use rusty_emu::components::common::intel_400x::*;
use rusty_emu::components::memory::intel_4001::Intel4001;
use test_utils::*;
use test_config::TestConfig;

#[cfg(test)]
mod test_demonstration {
    use super::*;

    #[test]
    fn demo_basic_functionality() {
        println!("=== Intel 400x Common Functionality Test Demo ===");

        // Test 1: Verify timing constants
        println!("\n1. Testing timing constants...");
        assert!(TimingConstants::DEFAULT_ACCESS_TIME > std::time::Duration::from_nanos(0));
        assert!(TimingConstants::FAST_ACCESS_TIME > std::time::Duration::from_nanos(0));
        println!("✓ Timing constants are valid");

        // Test 2: Test address assembly
        println!("\n2. Testing address assembly...");
        let mut scenario = create_standard_test_scenario();
        scenario.component.set_address_high_nibble(Some(0x12));
        scenario.component.set_address_low_nibble(Some(0x34));
        scenario.component.set_full_address_ready(true);

        assert_eq!(scenario.component.get_address_high_nibble(), Some(0x12));
        assert_eq!(scenario.component.get_address_low_nibble(), Some(0x34));
        println!("✓ Address assembly works correctly");

        // Test 3: Test state machine
        println!("\n3. Testing state machine...");
        scenario.component.set_timing_state(TimingState::AddressPhase);
        assert_eq!(scenario.component.get_timing_state(), TimingState::AddressPhase);
        assert!(scenario.component.get_timing_state().is_address_phase());
        println!("✓ State machine transitions work correctly");

        // Test 4: Test real component integration
        println!("\n4. Testing real component integration...");
        let mut rom = Intel4001::new_with_access_time("DemoROM".to_string(), 1);
        rom.load_rom_data(vec![0xAB, 0xCD], 0).unwrap();

        assert_eq!(rom.read_rom(0x00).unwrap(), 0xAB);
        assert_eq!(rom.read_rom(0x01).unwrap(), 0xCD);
        println!("✓ Real component integration works");

        // Test 5: Test trait consistency
        println!("\n5. Testing trait consistency...");
        verify_intel400x_traits(&rom);
        println!("✓ All required traits are implemented correctly");

        println!("\n=== All tests passed! ===");
        println!("The intel_400x common functionality is working correctly.");
    }

    #[test]
    fn demo_comprehensive_testing() {
        println!("=== Comprehensive Intel 400x Testing Demo ===");

        // Create test configuration
        let config = TestConfig::default();

        if config.enable_timing_tests {
            println!("\n1. Running timing tests...");
            verify_timing_constants();
            println!("✓ Timing tests completed");
        }

        if config.enable_property_tests {
            println!("\n2. Running property tests...");
            verify_state_machine_properties();
            println!("✓ Property tests completed");
        }

        // Test with different scenarios
        println!("\n3. Testing different scenarios...");

        let mut fast_scenario = crate::mocks::MockScenario::new("FastTest");
        fast_scenario.set_pin_value("SYNC", rusty_emu::pin::PinValue::High);
        fast_scenario.set_pin_value("CM", rusty_emu::pin::PinValue::High);
        fast_scenario.set_pin_value("CI", rusty_emu::pin::PinValue::Low);

        // Test rapid state changes
        for i in 0..10 {
            fast_scenario.component.set_timing_state(TimingState::Idle);
            fast_scenario.component.set_timing_state(TimingState::AddressPhase);
            fast_scenario.component.set_timing_state(TimingState::WaitLatency);
            fast_scenario.component.set_timing_state(TimingState::DriveData);
        }

        assert_eq!(fast_scenario.component.get_timing_state(), TimingState::DriveData);
        println!("✓ Rapid state changes handled correctly");

        // Test data bus operations
        fast_scenario.set_data_bus_value(0x0F);
        assert_eq!(fast_scenario.get_data_bus_value(), 0x0F);
        println!("✓ Data bus operations work correctly");

        println!("\n=== Comprehensive testing completed successfully! ===");
    }

    #[test]
    fn demo_error_handling() {
        println!("=== Error Handling Demo ===");

        // Test graceful error handling
        let mut rom = Intel4001::new("ErrorTestROM".to_string());

        // Test invalid memory access
        assert_eq!(rom.read_rom(0xFF), Some(0x00)); // Default value
        assert_eq!(rom.read_rom(0x100), None); // Out of bounds
        println!("✓ Invalid memory access handled gracefully");

        // Test invalid data loading
        assert!(rom.load_rom_data(vec![0x12], 255).is_ok()); // Valid
        assert!(rom.load_rom_data(vec![0x12], 256).is_err()); // Invalid
        println!("✓ Invalid data loading handled gracefully");

        println!("=== Error handling demo completed ===");
    }

    #[test]
    fn demo_performance_characteristics() {
        println!("=== Performance Characteristics Demo ===");

        // Test that components can handle rapid operations
        let mut rom = Intel4001::new_with_access_time("PerfTestROM".to_string(), 1);

        let start_time = std::time::Instant::now();

        // Perform many state transitions
        for _ in 0..1000 {
            rom.set_timing_state(TimingState::Idle);
            rom.set_timing_state(TimingState::AddressPhase);
            rom.set_timing_state(TimingState::WaitLatency);
            rom.set_timing_state(TimingState::DriveData);
        }

        let elapsed = start_time.elapsed();
        println!("✓ 4000 state transitions completed in {:?}", elapsed);

        // Test address operations
        for i in 0..256 {
            rom.set_address_high_nibble(Some((i >> 4) as u8));
            rom.set_address_low_nibble(Some((i & 0x0F) as u8));
            rom.set_full_address_ready(i % 2 == 0);
        }

        let elapsed = start_time.elapsed();
        println!("✓ 256 address operations completed in {:?}", elapsed);

        println!("=== Performance demo completed ===");
    }
}