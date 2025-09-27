use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::pin::Pin;

// Component is now inherently thread-safe
pub trait Component: Send + Sync {
    fn name(&self) -> String;
    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>>;
    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String>;
    fn update(&mut self);
    fn run(&mut self);
    fn stop(&mut self);
    fn is_running(&self) -> bool;
}
pub trait RunnableComponent: Component + Send + 'static {
    fn spawn_in_thread(mut self) -> thread::JoinHandle<()>
    where
        Self: Sized,
    {
        thread::spawn(move || {
            self.run();
        })
    }
}
// BaseComponent uses AtomicBool for thread-safe state
pub struct BaseComponent {
    name: String,
    pins: HashMap<String, Arc<Mutex<Pin>>>,
    running: AtomicBool, // Thread-safe running state
}

impl BaseComponent {
    pub fn new(name: String, pins: HashMap<String, Arc<Mutex<Pin>>>) -> Self {
        BaseComponent {
            name,
            pins,
            running: AtomicBool::new(false),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }

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

    fn update(&mut self) {
        // Base implementation does nothing
    }

    fn run(&mut self) {
        self.set_running(true);
        while self.is_running() {
            self.update();
            thread::sleep(Duration::from_micros(10));
        }
    }

    fn stop(&mut self) {
        self.set_running(false);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl RunnableComponent for BaseComponent {}
