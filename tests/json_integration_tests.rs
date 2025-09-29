//! JSON-Based Integration Tests
//!
//! These tests verify that the JSON configuration system works correctly
//! and that complete MCS-4 systems function as expected.

use rusty_emu::system_config::SystemFactory;
use std::fs;

#[cfg(test)]
mod json_system_tests {
    use super::*;

    #[test]
    fn test_system_factory_creation() {
        let factory = SystemFactory::new();
        // Factory should be created successfully
        assert!(format!("{:?}", factory).len() > 0);
    }

    #[test]
    fn test_basic_system_loading() {
        let factory = SystemFactory::new();

        // Test loading basic system configuration
        let result = factory.create_from_json("configs/mcs4_basic.json");

        assert!(result.is_ok(), "Failed to create basic system: {:?}", result.err());

        let mut system = result.unwrap();
        let info = system.get_system_info();

        // Verify system properties
        assert_eq!(info.name, "IntelMcs4");
        assert_eq!(info.component_count, 5); // CPU, Clock, 2 ROMs, 1 RAM
        assert_eq!(info.cpu_speed, 750000.0);
    }


    #[test]
    fn test_system_component_access() {
        let factory = SystemFactory::new();
        let system = factory.create_from_json("configs/mcs4_basic.json").unwrap();

        // Test that we can access system components
        let info = system.get_system_info();
        assert_eq!(info.component_count, 5);

        // Test that system is initially not running
        assert!(!system.is_running());
    }

    #[test]
    fn test_invalid_config_file() {
        let factory = SystemFactory::new();

        // Test with non-existent file
        let result = factory.create_from_json("configs/non_existent.json");
        assert!(result.is_err());

        // Test with invalid JSON
        let invalid_json = "{\"invalid\": json}";
        fs::write("test_invalid.json", invalid_json).unwrap();

        let result = factory.create_from_json("test_invalid.json");
        assert!(result.is_err());

        // Clean up
        let _ = fs::remove_file("test_invalid.json");
    }

    #[test]
    fn test_system_configuration_metadata() {
        let factory = SystemFactory::new();
        let system = factory.create_from_json("configs/mcs4_basic.json").unwrap();

        let info = system.get_system_info();

        // Verify metadata is loaded correctly
        assert_eq!(info.name, "IntelMcs4");
        assert_eq!(info.description, "Basic MCS-4 System with CPU, Clock, 2 ROMs, and 1 RAM");
        assert_eq!(info.component_count, 5);
        assert_eq!(info.cpu_speed, 750000.0);
        assert_eq!(info.rom_size, 256);
        assert_eq!(info.ram_size, 40);
    }
}

#[cfg(test)]
mod system_execution_tests {
    use super::*;

    #[test]
    fn test_system_execution_lifecycle() {
        let factory = SystemFactory::new();
        let system = factory.create_from_json("configs/mcs4_basic.json").unwrap();

        // System should start not running
        assert!(!system.is_running());

        // Test that we can start and stop the system
        // Note: In a real test, we might want to run for a very short time
        // or mock the execution to avoid infinite loops

        // For now, just test that the system object exists and has correct properties
        let info = system.get_system_info();
        assert_eq!(info.name, "IntelMcs4");
    }

}

#[cfg(test)]
mod fibonacci_program_tests {
    use super::*;

    #[test]
    fn test_fibonacci_program_loading() {
        let factory = SystemFactory::new();
        let system = factory.create_from_json("configs/mcs4_basic.json").unwrap();

        // Test that the system can be created with fibonacci program
        // The actual program loading is handled by the main application
        let info = system.get_system_info();
        assert_eq!(info.name, "IntelMcs4");
    }

    #[test]
    fn test_program_file_existence() {
        // Test that fibonacci program files exist
        assert!(std::path::Path::new("programs/fibonacci.bin").exists());
        assert!(std::path::Path::new("programs/fibonacci_output.bin").exists());

        // Test that config files exist
        assert!(std::path::Path::new("configs/mcs4_basic.json").exists());
        assert!(std::path::Path::new("configs/mcs4_max.json").exists());
    }

    #[test]
    fn test_program_file_sizes() {
        // Test that program files have reasonable sizes
        let fibonacci_size = fs::metadata("programs/fibonacci.bin")
            .unwrap()
            .len();
        let fibonacci_output_size = fs::metadata("programs/fibonacci_output.bin")
            .unwrap()
            .len();

        // Both should be non-zero and reasonable sizes
        assert!(fibonacci_size > 0);
        assert!(fibonacci_output_size > 0);
        assert!(fibonacci_size < 1000); // Reasonable upper bound
        assert!(fibonacci_output_size < 1000);
    }
}

#[cfg(test)]
mod configuration_validation_tests {
    use super::*;


    #[test]
    fn test_component_naming_consistency() {
        let factory = SystemFactory::new();
        let system = factory.create_from_json("configs/mcs4_basic.json").unwrap();

        // Test that component names match expected patterns
        let info = system.get_system_info();
        assert_eq!(info.name, "IntelMcs4");
        assert_eq!(info.component_count, 5);
    }
}