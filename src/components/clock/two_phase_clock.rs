use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

pub struct TwoPhaseClock {
    base: BaseComponent,
    phi1_state: PinValue,
    phi2_state: PinValue,
    last_transition: Instant,
    phase_time: Duration,
    enabled: bool,
}

impl TwoPhaseClock {
    pub fn new(name: String, frequency: f64) -> Self {
        let pin_names = vec!["CLK", "PHI1", "PHI2", "ENABLE"]; // Keep CLK for compatibility
        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        let phase_time = if frequency > 0.0 {
            Duration::from_secs_f64(1.0 / frequency / 2.0) // Half period for each phase
        } else {
            Duration::from_secs(1)
        };

        TwoPhaseClock {
            base: BaseComponent::new(name, pins),
            phi1_state: PinValue::High, // Start with PHI1 high
            phi2_state: PinValue::Low,
            last_transition: Instant::now(),
            phase_time,
            enabled: true,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.phi1_state = PinValue::High;
        self.phi2_state = PinValue::Low;
        self.last_transition = Instant::now();
        self.update_outputs();

        // Force an immediate update to ensure outputs are driven
        self.update_outputs();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.phi1_state = PinValue::Low;
        self.phi2_state = PinValue::Low;
        self.update_outputs();
    }

    fn update_outputs(&self) {
        // Set CLK output (for compatibility)
        if let Ok(clk_pin) = self.base.get_pin("CLK") {
            if let Ok(mut pin_guard) = clk_pin.lock() {
                // CLK follows PHI1 for compatibility
                pin_guard.set_driver(Some(self.base.get_name().to_string()), self.phi1_state);
            }
        }

        // Set PHI1 output
        if let Ok(phi1_pin) = self.base.get_pin("PHI1") {
            if let Ok(mut pin_guard) = phi1_pin.lock() {
                pin_guard.set_driver(Some(self.base.get_name().to_string()), self.phi1_state);

                // Check if pin has connections and trigger propagation
                let connection_count = pin_guard.get_connected_pins().len();
                if connection_count > 0 {
                    pin_guard.propagate();
                }
            }
        }

        // Set PHI2 output
        if let Ok(phi2_pin) = self.base.get_pin("PHI2") {
            if let Ok(mut pin_guard) = phi2_pin.lock() {
                pin_guard.set_driver(Some(self.base.get_name().to_string()), self.phi2_state);

                // Check if pin has connections and trigger propagation
                let connection_count = pin_guard.get_connected_pins().len();
                if connection_count > 0 {
                    pin_guard.propagate();
                }
            }
        }
    }

    fn should_transition(&self) -> bool {
        self.last_transition.elapsed() >= self.phase_time
    }

    fn perform_transition(&mut self) {
        match (self.phi1_state, self.phi2_state) {
            (PinValue::High, PinValue::Low) => {
                // PHI1 -> Low, PHI2 -> High
                self.phi1_state = PinValue::Low;
                self.phi2_state = PinValue::High;
            }
            (PinValue::Low, PinValue::High) => {
                // PHI2 -> Low, PHI1 -> High
                self.phi1_state = PinValue::High;
                self.phi2_state = PinValue::Low;
            }
            _ => {
                // Reset to known state
                self.phi1_state = PinValue::High;
                self.phi2_state = PinValue::Low;
            }
        }

        self.update_outputs();
        self.last_transition = Instant::now();
    }
}

impl Component for TwoPhaseClock {
    fn name(&self) -> String {
        self.base.name()
    }

    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>> {
        self.base.pins()
    }

    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String> {
        self.base.get_pin(name)
    }

    fn update(&mut self) {
        if !self.enabled {
            return;
        }

        // Always update outputs to ensure they're driven
        self.update_outputs();

        if self.should_transition() {
            self.perform_transition();
        }
    }

    fn run(&mut self) {
        self.base.set_running(true);
        self.enable();

        while self.is_running() {
            self.update();

            // Sleep for a very short time to allow frequent updates
            thread::sleep(Duration::from_micros(100)); // 100µs = 10kHz update rate

            // Check if should stop
            if !self.is_running() {
                break;
            }
        }

        self.disable();
    }

    fn stop(&mut self) {
        self.base.set_running(false);
        self.disable();
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }
}