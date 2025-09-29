//! Test library for Intel 400x common functionality
//!
//! This module provides a centralized entry point for all tests
//! and exports common testing utilities.

#![cfg(test)]

// Re-export the rusty_emu crate for use in tests
pub use rusty_emu;

// Module declarations for test files
mod integration_tests;
mod intel_400x_tests;
mod mock_based_tests;
mod mocks;
mod property_based_tests;

// Common test utilities and helpers
pub mod test_utils {
    use std::time::{Duration, Instant};

    /// Create a deterministic time source for testing
    pub fn create_test_time() -> Instant {
        Instant::now()
    }

    /// Create a test duration for timing-sensitive tests
    pub fn create_test_duration(nanos: u64) -> Duration {
        Duration::from_nanos(nanos)
    }

    /// Helper to advance time in tests
    pub fn advance_test_time(duration: Duration) {
        std::thread::sleep(duration);
    }

    /// Create a test scenario with known good values
    pub fn create_standard_test_scenario() -> crate::mocks::MockScenario {
        let scenario = crate::mocks::MockScenario::new("StandardTest");
        scenario
            .component
            .set_pin_value("SYNC", rusty_emu::pin::PinValue::High);
        scenario
            .component
            .set_pin_value("CM", rusty_emu::pin::PinValue::High);
        scenario
            .component
            .set_pin_value("CI", rusty_emu::pin::PinValue::Low);
        scenario
    }

    /// Verify that a component implements all required traits
    pub fn verify_intel400x_traits<C>(component: &C)
    where
        C: rusty_emu::components::common::intel_400x::Intel400xClockHandling,
        C: rusty_emu::components::common::intel_400x::Intel400xDataBus,
        C: rusty_emu::components::common::intel_400x::Intel400xAddressHandling,
        C: rusty_emu::components::common::intel_400x::Intel400xControlPins,
        C: rusty_emu::components::common::intel_400x::Intel400xResetHandling,
        C: rusty_emu::components::common::intel_400x::Intel400xTimingState,
    {
        // Test Intel400xClockHandling
        let _: &dyn rusty_emu::components::common::intel_400x::Intel400xClockHandling = component;

        // Test Intel400xDataBus
        let _: &dyn rusty_emu::components::common::intel_400x::Intel400xDataBus = component;

        // Test Intel400xAddressHandling
        let _: &dyn rusty_emu::components::common::intel_400x::Intel400xAddressHandling = component;

        // Test Intel400xControlPins
        let _: &dyn rusty_emu::components::common::intel_400x::Intel400xControlPins = component;

        // Test Intel400xResetHandling
        let _: &dyn rusty_emu::components::common::intel_400x::Intel400xResetHandling = component;

        // Test Intel400xTimingState
        let _: &dyn rusty_emu::components::common::intel_400x::Intel400xTimingState = component;
    }

    /// Test that timing constants are reasonable
    #[allow(dead_code)]
    pub fn verify_timing_constants() {
        use rusty_emu::components::common::intel_400x::TimingConstants;

        assert!(TimingConstants::DEFAULT_ACCESS_TIME > Duration::from_nanos(0));
        assert!(TimingConstants::FAST_ACCESS_TIME > Duration::from_nanos(0));
        assert!(TimingConstants::ADDRESS_SETUP > Duration::from_nanos(0));
        assert!(TimingConstants::DATA_VALID > Duration::from_nanos(0));

        // Fast access should be faster than default
        assert!(TimingConstants::FAST_ACCESS_TIME < TimingConstants::DEFAULT_ACCESS_TIME);
    }

    /// Test that state machine properties hold
    #[allow(dead_code)]
    pub fn verify_state_machine_properties() {
        use rusty_emu::components::common::intel_400x::TimingState;

        let states = vec![
            TimingState::Idle,
            TimingState::AddressPhase,
            TimingState::WaitLatency,
            TimingState::DriveData,
        ];

        for state in states {
            // Each state should have consistent query results
            match state {
                TimingState::Idle => {
                    assert!(state.is_idle());
                    assert!(!state.is_address_phase());
                    assert!(!state.is_waiting_latency());
                    assert!(!state.is_driving_data());
                }
                TimingState::AddressPhase => {
                    assert!(!state.is_idle());
                    assert!(state.is_address_phase());
                    assert!(!state.is_waiting_latency());
                    assert!(!state.is_driving_data());
                }
                TimingState::WaitLatency => {
                    assert!(!state.is_idle());
                    assert!(!state.is_address_phase());
                    assert!(state.is_waiting_latency());
                    assert!(!state.is_driving_data());
                }
                TimingState::DriveData => {
                    assert!(!state.is_idle());
                    assert!(!state.is_address_phase());
                    assert!(!state.is_waiting_latency());
                    assert!(state.is_driving_data());
                }
            }
        }
    }
}

// Test configuration and setup
pub mod test_config {
    /// Test configuration for running the test suite
    pub struct TestConfig {
        pub enable_detailed_output: bool,
        pub enable_timing_tests: bool,
        pub enable_property_tests: bool,
    }

    impl Default for TestConfig {
        fn default() -> Self {
            Self {
                enable_detailed_output: false,
                enable_timing_tests: true,
                enable_property_tests: true,
            }
        }
    }

    impl TestConfig {
        pub fn with_detailed_output(mut self) -> Self {
            self.enable_detailed_output = true;
            self
        }

        pub fn without_timing_tests(mut self) -> Self {
            self.enable_timing_tests = false;
            self
        }

        pub fn without_property_tests(mut self) -> Self {
            self.enable_property_tests = false;
            self
        }
    }
}
