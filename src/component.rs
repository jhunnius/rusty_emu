use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use crate::pin::{Pin, PinValue};
// Component trait
pub trait Component: Send {
    fn name(&self) -> &str;
    fn pins(&self) -> &HashMap<String, Arc<Pin>>;
    fn get_pin(&self, name: &str) -> Option<Arc<Pin>>;
    fn update(&mut self) -> Result<(), String>;
    fn run(&mut self);
    fn stop(&mut self);
}
#[derive(Clone)]
pub struct BaseComponent {
    pub name: String,
    pub pins: HashMap<String, Arc<Pin>>,
    pub running: Arc<Mutex<bool>>,
}
impl BaseComponent {
    pub fn new(name: String) -> Self {
        Self {
            name,
            pins: HashMap::new(),
            running: Arc::new(Mutex::new(false)),
        }
    }
    pub fn add_pin(&mut self, name: String, initial_value: PinValue, initial_output_enable: bool) -> Arc<Pin> {
        let pin = Arc::new(Pin {
            name: name.clone(),
            value: Arc::new(RwLock::new(initial_value)),
            output_enable: Arc::new(Mutex::new(initial_output_enable)),
            connections: Arc::new(Mutex::new(Vec::new())),
        });

        self.pins.insert(name, pin.clone());
        pin
    }
}
impl Component for BaseComponent {
    fn name(&self) -> &str {
        &self.name
    }
    fn pins(&self) -> &HashMap<String, Arc<Pin>> {
        &self.pins
    }
    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.pins.get(name).cloned()
    }
    fn update(&mut self) -> Result<(), String> {
        // Base implementation does nothing
        Ok(())
    }
    fn run(&mut self) {
        *self.running.lock().unwrap() = true;
        let running = self.running.clone();
        let mut self_clone = self.clone();

        thread::spawn(move || {
            while *running.lock().unwrap() {
                if let Err(e) = self_clone.update() {
                    eprintln!("Error in {}: {}", self_clone.name, e);
                }
                thread::sleep(Duration::from_micros(10));
            }
        });
    }
    fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
    }
}