use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

#[derive(Clone)]
pub struct GenericClock {
    base: BaseComponent,
    half_period: Duration,
    last_state: bool,
}
impl GenericClock {
    pub fn new(name: String, frequency_hz: u64) -> Self {
        let mut base = BaseComponent::new(name);
        let _out_pin = base.add_pin("out".to_string(), PinValue::Low, true);

        let half_period_ms = 500 / frequency_hz; // Convert to milliseconds

        Self {
            base,
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