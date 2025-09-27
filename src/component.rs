use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::pin::Pin;

/// Core trait for all hardware components in the emulator
/// Provides the fundamental interface that all components must implement
/// Components are inherently thread-safe and can be used across thread boundaries
pub trait Component: Send + Sync {
    /// Get the name/identifier of this component
    /// Returns: Component name as a String
    fn name(&self) -> String;

    /// Get all pins associated with this component
    /// Returns: HashMap mapping pin names to pin objects
    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>>;

    /// Get a specific pin by name
    /// Parameters: name - The name of the pin to retrieve
    /// Returns: Ok(pin) if found, Err(String) if pin not found
    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String>;

    /// Update the component state for one simulation cycle
    /// This method is called repeatedly during component execution
    fn update(&mut self);

    /// Run the component in a continuous loop until stopped
    /// This method blocks until the component is stopped
    fn run(&mut self);

    /// Stop the component and clean up resources
    /// This method should tri-state all outputs and prepare for shutdown
    fn stop(&mut self);

    /// Check if the component is currently running
    /// Returns: true if component is running, false otherwise
    fn is_running(&self) -> bool;
}
/// Extended trait for components that can be run in their own threads
/// Provides automatic thread spawning functionality for components
pub trait RunnableComponent: Component + Send + 'static {
    /// Spawn the component in its own thread
    /// Parameters: self - The component to spawn (consumed by the thread)
    /// Returns: JoinHandle for the spawned thread
    fn spawn_in_thread(mut self) -> thread::JoinHandle<()>
    where
        Self: Sized,
    {
        thread::spawn(move || {
            self.run();
        })
    }
}
/// Base implementation of the Component trait providing common functionality
/// Handles thread-safe state management and basic pin operations
/// Most hardware components should embed this struct to inherit common behavior
pub struct BaseComponent {
    name: String,
    pins: HashMap<String, Arc<Mutex<Pin>>>,
    running: AtomicBool, // Thread-safe running state
}

impl BaseComponent {
    /// Create a new BaseComponent with the specified name and pins
    /// Parameters: name - Component identifier, pins - Pin mapping for the component
    /// Returns: New BaseComponent instance
    pub fn new(name: String, pins: HashMap<String, Arc<Mutex<Pin>>>) -> Self {
        BaseComponent {
            name,
            pins,
            running: AtomicBool::new(false),
        }
    }

    /// Get the name of this component
    /// Returns: Component name as string slice
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Check if the component is currently running
    /// Returns: true if running, false otherwise
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Set the running state of the component
    /// Parameters: running - New running state
    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }

    /// Create a pin mapping from a list of pin names
    /// Parameters: pin_names - List of pin name strings, component_name - Name of the component
    /// Returns: HashMap mapping pin names to Pin objects with proper naming
    pub fn create_pin_map(
        pin_names: &[&str],
        component_name: &str,
    ) -> HashMap<String, Arc<Mutex<Pin>>> {
        let mut pins = HashMap::new();
        for pin_name in pin_names {
            pins.insert(
                pin_name.to_string(),
                Arc::new(Mutex::new(Pin::new(format!(
                    "{}_{}",
                    component_name, pin_name
                )))),
            );
        }
        pins
    }
}

impl Component for BaseComponent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        self.pins.clone()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.pins
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Pin {} not found", name))
    }

    /// Update the component state for one simulation cycle
    /// Base implementation does nothing - should be overridden by specific components
    fn update(&mut self) {
        // Base implementation does nothing
    }

    /// Run the component in a continuous loop until stopped
    /// Provides a default time-sliced execution model with 10 microsecond delays
    fn run(&mut self) {
        self.set_running(true);
        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10));
        }
    }

    /// Stop the component and clean up resources
    /// Sets running state to false to exit the run loop
    fn stop(&mut self) {
        self.set_running(false);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl RunnableComponent for BaseComponent {}
