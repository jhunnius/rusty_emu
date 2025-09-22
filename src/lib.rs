pub mod component;
pub mod pin;
pub mod connection;
pub mod components;
pub mod systems;
mod bus;

// Re-export commonly used items for easier importing
pub use component::{Component, BaseComponent};
pub use pin::{Pin, PinValue};
pub use connection::connect_pins;