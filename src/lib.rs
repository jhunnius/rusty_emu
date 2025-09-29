//! # Rusty Emulator Library
//!
//! A comprehensive Intel MCS-4 microprocessor simulator written in Rust.
//!
//! This library provides:
//! - JSON-configurable system architecture for flexible system definition
//! - Cycle-accurate hardware simulation of Intel 4004/4001/4002/4003 components
//! - Comprehensive testing framework with multiple testing strategies
//! - Extensible component system with trait-based architecture
//! - Professional project organization with clean separation of concerns

pub mod component;
pub mod components;
pub mod connection;
pub mod console;
pub mod pin;
pub mod system_config;
pub mod types;

// Re-export commonly used items for easier importing
pub use component::{BaseComponent, Component};
pub use connection::connect_pins;
pub use pin::{Pin, PinValue};
