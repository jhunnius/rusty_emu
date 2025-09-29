mod bus;
pub mod component;
pub mod components;
pub mod connection;
pub mod pin;
pub mod system_config;
pub mod types;

// Re-export commonly used items for easier importing
pub use component::{BaseComponent, Component};
pub use connection::connect_pins;
pub use pin::{Pin, PinValue};
