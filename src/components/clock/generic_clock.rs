use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::component::{BaseComponent, Component};
use crate::pin::{Pin, PinValue};

pub struct GenericClock {
    base: BaseComponent,
    frequency: f64,          // Frequency in Hz
    duty_cycle: f64,         // Duty cycle as percentage (0.0 to 1.0)
    current_state: PinValue,
    last_transition: Instant,
    half_period: Duration,
    high_time: Duration,
    low_time: Duration,
    enabled: bool,
}

impl GenericClock {
    pub fn new(name: String, frequency: f64) -> Self {
        let pin_names = vec!["CLK", "ENABLE"];
        let pins = BaseComponent::create_pin_map(&pin_names, &name);

        let half_period = Self::frequency_to_duration(frequency) / 2;

        let mut clock = GenericClock {
            base: BaseComponent::new(name, pins),
            frequency,
            duty_cycle: 0.5, // 50% duty cycle by default
            current_state: PinValue::Low,
            last_transition: Instant::now(),
            half_period,
            high_time: half_period, // Will be recalculated in set_duty_cycle
            low_time: half_period,  // Will be recalculated in set_duty_cycle
            enabled: true,
        };

        clock.set_duty_cycle(0.5); // Initialize timing
        clock
    }

    pub fn with_duty_cycle(mut self, duty_cycle: f64) -> Self {
        self.set_duty_cycle(duty_cycle);
        self
    }

    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
        self.half_period = Self::frequency_to_duration(frequency) / 2;
        self.update_timing();
    }

    pub fn get_frequency(&self) -> f64 {
        self.frequency
    }

    pub fn set_duty_cycle(&mut self, duty_cycle: f64) {
        self.duty_cycle = duty_cycle.clamp(0.1, 0.9); // Keep within reasonable bounds
        self.update_timing();
    }

    pub fn get_duty_cycle(&self) -> f64 {
        self.duty_cycle
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

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn reset(&mut self) {
        self.current_state = PinValue::Low;
        self.last_transition = Instant::now();
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

    fn pins(&self) -> &HashMap<String, Arc<Mutex<Pin>>> {
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
impl GenericClock {
    pub fn generate_single_pulse(&mut self, width: Duration) {
        self.disable(); // Stop regular clock
        self.set_clock_output(PinValue::High);

        // Spawn a thread to end the pulse after specified width
        let clock_name = self.base.get_name().to_string();
        let clock_pin = self.base.get_pin("CLK").unwrap().clone();

        thread::spawn(move || {
            thread::sleep(width);
            if let Ok(mut pin_guard) = clock_pin.lock() {
                pin_guard.set_driver(Some(clock_name), PinValue::Low);
            }
        });
    }

    pub fn generate_clock_burst(&mut self, count: usize) {
        self.disable(); // Stop regular clock

        let clock_name = self.base.get_name().to_string();
        let clock_pin = self.base.get_pin("CLK").unwrap().clone();
        let half_period = self.half_period;

        thread::spawn(move || {
            for i in 0..count * 2 { // Each cycle has two transitions
                let state = if i % 2 == 0 { PinValue::High } else { PinValue::Low };

                if let Ok(mut pin_guard) = clock_pin.lock() {
                    pin_guard.set_driver(Some(clock_name.clone()), state);
                }

                thread::sleep(half_period);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_clock_creation() {
        let clock = GenericClock::new("TEST_CLK".to_string(), 1_000_000.0); // 1MHz
        assert_eq!(clock.name(), "TEST_CLK");
        assert_eq!(clock.get_frequency(), 1_000_000.0);
        assert_eq!(clock.get_duty_cycle(), 0.5);
        assert!(!clock.is_running());
    }

    #[test]
    fn test_clock_frequency_change() {
        let mut clock = GenericClock::new("TEST_CLK".to_string(), 1_000_000.0);
        clock.set_frequency(2_000_000.0);
        assert_eq!(clock.get_frequency(), 2_000_000.0);
    }

    #[test]
    fn test_clock_duty_cycle() {
        let mut clock = GenericClock::new("TEST_CLK".to_string(), 1_000_000.0);
        clock.set_duty_cycle(0.25);
        assert_eq!(clock.get_duty_cycle(), 0.25);

        // Test clamping
        clock.set_duty_cycle(1.5);
        assert_eq!(clock.get_duty_cycle(), 0.9);

        clock.set_duty_cycle(-0.5);
        assert_eq!(clock.get_duty_cycle(), 0.1);
    }

    #[test]
    fn test_clock_enable_disable() {
        let mut clock = GenericClock::new("TEST_CLK".to_string(), 1_000_000.0);

        assert!(clock.is_enabled());
        clock.disable();
        assert!(!clock.is_enabled());

        clock.enable();
        assert!(clock.is_enabled());
    }

    #[test]
    fn test_clock_timing_calculation() {
        let clock = GenericClock::new("TEST_CLK".to_string(), 1.0); // 1Hz
        let period = GenericClock::frequency_to_duration(1.0);
        assert_eq!(period, Duration::from_secs(1));

        let high_freq = GenericClock::frequency_to_duration(1_000_000.0); // 1MHz
        assert_eq!(high_freq, Duration::from_micros(1));
    }

    #[test]
    fn test_single_pulse() {
        let mut clock = GenericClock::new("PULSE_CLK".to_string(), 1_000_000.0);
        clock.generate_single_pulse(Duration::from_millis(10));
        // Note: In a real test, you'd want to verify the pin state
    }

    #[test]
    fn test_clock_burst() {
        let mut clock = GenericClock::new("BURST_CLK".to_string(), 1_000_000.0);
        clock.generate_clock_burst(5); // 5 clock cycles
        // Note: In a real test, you'd want to verify the pin states
    }
}