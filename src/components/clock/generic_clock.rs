use std::collections::HashMap;
use crate::{PinValue, BaseComponent, Component, Pin};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
#[derive(Clone)]
pub struct GenericClock {
    base: BaseComponent,
    frequency: u64,
    half_period: Duration,
    last_state: bool,
}
impl GenericClock {
    pub fn new(name: String, frequency_hz: u64) -> Self {
        let mut base = BaseComponent::new(name);
        let out_pin = base.add_pin("out".to_string(), PinValue::Low, true);

        let half_period_ms = 500 / frequency_hz; // Convert to milliseconds

        Self {
            base,
            frequency: frequency_hz,
            half_period: Duration::from_millis(half_period_ms),
            last_state: false,
        }
    }

    pub fn out(&self) -> Option<Arc<Pin>> {
        self.base.get_pin("out")
    }
}

impl Component for GenericClock {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn pins(&self) -> &HashMap<String, Arc<Pin>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Option<Arc<Pin>> {
        self.base.get_pin(name)
    }

    fn connect_pin(&mut self, pin_name: &str, other_pin: Arc<Pin>) -> Result<(), String> {
        self.base.connect_pin(pin_name, other_pin)
    }

    fn update(&mut self) -> Result<(), String> {
        let out_pin = self.base.pins.get("out").unwrap();

        // Toggle clock output
        self.last_state = !self.last_state;
        let new_value = if self.last_state { PinValue::High } else { PinValue::Low };

        out_pin.write(new_value, true);
        thread::sleep(self.half_period);

        Ok(())
    }

    fn run(&mut self) {
        self.base.run();
    }

    fn stop(&mut self) {
        self.base.stop();
    }
}