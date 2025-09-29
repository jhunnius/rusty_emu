//! Simple test to verify the intel_400x functionality works
//!
//! This is a minimal test that demonstrates the core functionality
//! without complex mock setups.

use rusty_emu::components::common::intel_400x::*;
use rusty_emu::components::memory::intel_4001::Intel4001;

#[cfg(test)]
mod simple_tests {
    use super::*;

    #[test]
    fn test_basic_intel400x_functionality() {
        println!("=== Testing Basic Intel 400x Functionality ===");

        // Test 1: Verify timing constants
        println!("\n1. Testing timing constants...");
        assert!(TimingConstants::DEFAULT_ACCESS_TIME > std::time::Duration::from_nanos(0));
        assert!(TimingConstants::FAST_ACCESS_TIME > std::time::Duration::from_nanos(0));
        assert!(TimingConstants::FAST_ACCESS_TIME < TimingConstants::DEFAULT_ACCESS_TIME);
        println!("✓ Timing constants are valid");

        // Test 2: Test state machine
        println!("\n2. Testing state machine...");
        let idle_state = TimingState::Idle;
        let address_state = TimingState::AddressPhase;
        let wait_state = TimingState::WaitLatency;
        let drive_state = TimingState::DriveData;

        assert!(idle_state.is_idle());
        assert!(address_state.is_address_phase());
        assert!(wait_state.is_waiting_latency());
        assert!(drive_state.is_driving_data());
        println!("✓ State machine works correctly");

        // Test 3: Test address assembly logic
        println!("\n3. Testing address assembly...");

        // Test address assembly with different values
        let test_cases = vec![
            (0x00, 0x00, 0x0000),
            (0x12, 0x34, 0x1234),
            (0xFF, 0xFF, 0xFFFF),
            (0x0A, 0x0B, 0x0A0B),
        ];

        for (high, low, expected) in test_cases {
            let assembled = ((high as u16) << 4) | (low as u16);
            assert_eq!(assembled, expected);
        }
        println!("✓ Address assembly works correctly");

        // Test 4: Test real component creation
        println!("\n4. Testing real component...");
        let rom = Intel4001::new("TestROM".to_string());
        assert_eq!(rom.get_rom_size(), 256);
        println!("✓ Real component creation works");

        // Test 5: Test data loading and reading
        println!("\n5. Testing data operations...");
        let mut rom = Intel4001::new("DataTestROM".to_string());
        let test_data = vec![0x12, 0x34, 0x56, 0x78];
        rom.load_rom_data(test_data, 0).unwrap();

        assert_eq!(rom.read_rom(0x00).unwrap(), 0x12);
        assert_eq!(rom.read_rom(0x01).unwrap(), 0x34);
        assert_eq!(rom.read_rom(0x02).unwrap(), 0x56);
        assert_eq!(rom.read_rom(0x03).unwrap(), 0x78);
        println!("✓ Data operations work correctly");

        println!("\n=== All basic tests passed! ===");
        println!("The intel_400x common functionality is working correctly.");
    }

    #[test]
    fn test_state_transitions() {
        println!("=== Testing State Transitions ===");

        // Test that state transitions work correctly
        let mut rom = Intel4001::new("StateTestROM".to_string());

        // Test initial state
        assert_eq!(rom.get_timing_state(), TimingState::Idle);
        println!("✓ Initial state is Idle");

        // Test state changes
        rom.set_timing_state(TimingState::AddressPhase);
        assert_eq!(rom.get_timing_state(), TimingState::AddressPhase);
        println!("✓ Can transition to AddressPhase");

        rom.set_timing_state(TimingState::WaitLatency);
        assert_eq!(rom.get_timing_state(), TimingState::WaitLatency);
        println!("✓ Can transition to WaitLatency");

        rom.set_timing_state(TimingState::DriveData);
        assert_eq!(rom.get_timing_state(), TimingState::DriveData);
        println!("✓ Can transition to DriveData");

        // Test return to idle
        rom.set_timing_state(TimingState::Idle);
        assert_eq!(rom.get_timing_state(), TimingState::Idle);
        println!("✓ Can return to Idle");

        println!("=== State transition tests completed ===");
    }

    #[test]
    fn test_address_handling() {
        println!("=== Testing Address Handling ===");

        let mut rom = Intel4001::new("AddressTestROM".to_string());

        // Test address nibble handling
        rom.set_address_high_nibble(Some(0x0F));
        rom.set_address_low_nibble(Some(0x23));
        rom.set_full_address_ready(true);

        assert_eq!(rom.get_address_high_nibble(), Some(0x0F));
        assert_eq!(rom.get_address_low_nibble(), Some(0x23));
        assert_eq!(rom.get_full_address_ready(), true);
        println!("✓ Address nibbles handled correctly");

        // Test clearing address
        rom.set_address_high_nibble(None);
        rom.set_address_low_nibble(None);
        rom.set_full_address_ready(false);

        assert_eq!(rom.get_address_high_nibble(), None);
        assert_eq!(rom.get_address_low_nibble(), None);
        assert_eq!(rom.get_full_address_ready(), false);
        println!("✓ Address clearing works correctly");

        println!("=== Address handling tests completed ===");
    }

    #[test]
    fn test_reset_functionality() {
        println!("=== Testing Reset Functionality ===");

        let mut rom = Intel4001::new("ResetTestROM".to_string());

        // Set up some state
        rom.set_timing_state(TimingState::DriveData);
        rom.set_address_high_nibble(Some(0x12));
        rom.set_address_low_nibble(Some(0x34));
        rom.set_full_address_ready(true);

        // Verify state is set
        assert_eq!(rom.get_timing_state(), TimingState::DriveData);
        assert_eq!(rom.get_address_high_nibble(), Some(0x12));
        assert_eq!(rom.get_address_low_nibble(), Some(0x34));
        assert_eq!(rom.get_full_address_ready(), true);
        println!("✓ State setup completed");

        // Perform reset
        rom.perform_reset();

        // Verify reset behavior
        assert_eq!(rom.get_timing_state(), TimingState::Idle);
        assert_eq!(rom.get_address_high_nibble(), None);
        assert_eq!(rom.get_address_low_nibble(), None);
        assert_eq!(rom.get_full_address_ready(), false);
        println!("✓ Reset functionality works correctly");

        println!("=== Reset tests completed ===");
    }

    #[test]
    fn test_timing_configuration() {
        println!("=== Testing Timing Configuration ===");

        // Test different access times
        let fast_rom = Intel4001::new_with_access_time("FastROM".to_string(), 100);
        let slow_rom = Intel4001::new_with_access_time("SlowROM".to_string(), 1000);

        assert_eq!(fast_rom.get_access_time(), 100);
        assert_eq!(slow_rom.get_access_time(), 1000);
        println!("✓ Different access times configured correctly");

        // Test access time modification
        let mut rom = Intel4001::new("ConfigTestROM".to_string());
        assert_eq!(rom.get_access_time(), 500); // Default

        rom.set_access_time(250);
        assert_eq!(rom.get_access_time(), 250);
        println!("✓ Access time modification works correctly");

        println!("=== Timing configuration tests completed ===");
    }
}