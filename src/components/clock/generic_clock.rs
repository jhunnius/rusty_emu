use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

pub struct GenericClock {
    base: BaseComponent,
    frequency: f64,  // Frequency in Hz
    duty_cycle: f64, // Duty cycle as percentage (0.0 to 1.0)
    current_state: PinValue,
    last_transition: Instant,
    high_time: Duration,
    low_time: Duration,
    enabled: bool,
}

impl GenericClock {
    pub fn new(name: String, frequency: f64) -> Self {
        let pin_names = vec!["CLK", "ENABLE"];
        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        let mut clock = GenericClock {
            base: BaseComponent::new(name, pins),
            frequency,
            duty_cycle: 0.5, // 50% duty cycle by default
            current_state: PinValue::Low,
            last_transition: Instant::now(),
            high_time: Duration::from_secs_f64(0.5 / frequency), // Will be recalculated in set_duty_cycle
            low_time: Duration::from_secs_f64(0.5 / frequency), // Will be recalculated in set_duty_cycle
            enabled: true,
        };

        clock.set_duty_cycle(0.5); // Initialize timing
        clock
    }

    pub fn set_duty_cycle(&mut self, duty_cycle: f64) {
        self.duty_cycle = duty_cycle.clamp(0.1, 0.9); // Keep within reasonable bounds
        self.update_timing();
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        // Start with known state when enabled
        self.current_state = PinValue::Low;
        self.last_transition = Instant::now();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        // Set output to Low when disabled
        self.set_clock_output(PinValue::Low);
    }

    fn frequency_to_duration(frequency: f64) -> Duration {
        if frequency <= 0.0 {
            Duration::from_secs(0)
        } else {
            Duration::from_secs_f64(1.0 / frequency)
        }
    }

    fn update_timing(&mut self) {
        let period = Self::frequency_to_duration(self.frequency);
        self.high_time = Duration::from_secs_f64(period.as_secs_f64() * self.duty_cycle);
        self.low_time = Duration::from_secs_f64(period.as_secs_f64() * (1.0 - self.duty_cycle));
    }

    fn set_clock_output(&self, value: PinValue) {
        if let Ok(clock_pin) = self.base.get_pin("CLK") {
            if let Ok(mut pin_guard) = clock_pin.lock() {
                pin_guard.set_driver(Some(self.base.get_name().to_string()), value);
            }
        }
    }

    fn read_enable_pin(&self) -> bool {
        if let Ok(enable_pin) = self.base.get_pin("ENABLE") {
            if let Ok(pin_guard) = enable_pin.lock() {
                return pin_guard.read() == PinValue::High;
            }
        }
        true // Enabled by default if pin doesn't exist or can't be read
    }

    fn should_transition(&self) -> bool {
        let elapsed = self.last_transition.elapsed();

        match self.current_state {
            PinValue::High => elapsed >= self.high_time,
            PinValue::Low => elapsed >= self.low_time,
            PinValue::HighZ => true, // Always transition from HighZ
        }
    }

    fn perform_transition(&mut self) {
        let new_state = match self.current_state {
            PinValue::High => PinValue::Low,
            PinValue::Low => PinValue::High,
            PinValue::HighZ => PinValue::Low, // Start with Low from HighZ
        };

        self.current_state = new_state;
        self.set_clock_output(new_state);
        self.last_transition = Instant::now();
    }
}

impl Component for GenericClock {
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
        // Read external enable control
        let external_enable = self.read_enable_pin();
        let should_be_enabled = self.enabled && external_enable;

        if !should_be_enabled {
            if self.current_state != PinValue::Low {
                self.set_clock_output(PinValue::Low);
                self.current_state = PinValue::Low;
            }
            return;
        }

        // Check if it's time to transition
        if self.should_transition() {
            self.perform_transition();
        }
    }

    fn run(&mut self) {
        self.base.set_running(true);
        self.enable(); // Ensure clock is enabled when running

        while self.is_running() {
            self.update();

            // Sleep for a short time to prevent busy waiting
            // Use a fraction of the expected transition time for responsiveness
            let sleep_time = if self.frequency > 0.0 {
                Duration::from_secs_f64(1.0 / self.frequency / 100.0).max(Duration::from_micros(1))
            } else {
                Duration::from_micros(100)
            };

            thread::sleep(sleep_time);
        }

        // Clean up when stopping
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


// Advanced clock features
impl GenericClock {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_clock_creation() {
        let clock = GenericClock::new("TEST_CLK".to_string(), 1_000_000.0); // 1MHz
        assert_eq!(clock.name(), "TEST_CLK");
        assert!(!clock.is_running());
    }

    #[test]
    fn test_clock_timing_calculation() {
        let _clock = GenericClock::new("TEST_CLK".to_string(), 1.0); // 1Hz
        let period = GenericClock::frequency_to_duration(1.0);
        assert_eq!(period, Duration::from_secs(1));

        let high_freq = GenericClock::frequency_to_duration(1_000_000.0); // 1MHz
        assert_eq!(high_freq, Duration::from_micros(1));
    }
}
