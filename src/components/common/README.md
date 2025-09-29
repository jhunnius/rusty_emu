# Intel 400x Common Functionality

This module provides shared functionality for all Intel 400x series chips (4001, 4002, 4003, 4004). It contains common traits, utilities, and behaviors that are used across multiple components.

## Purpose

The Intel 400x series shares many common characteristics:
- **Two-phase clock operation** (Φ1, Φ2)
- **4-bit data bus** with multiplexed address/data
- **12-bit address space** (8-bit addresses in two 4-bit cycles)
- **Common timing constraints** and access patterns
- **Similar pin configurations** and control signals
- **Shared state machine patterns** for memory operations

Rather than duplicating this code in each component, it's centralized here for maintainability and consistency.

## Architecture

### Trait-Based Design

The common functionality is organized around several key traits:

```
Intel400xClockHandling    - Clock edge detection and timing
Intel400xDataBus         - Data bus read/write operations
Intel400xAddressHandling - Address latching and assembly
Intel400xControlPins     - Control signal reading
Intel400xResetHandling   - Reset signal handling
Intel400xTimingState     - Memory operation state machines
```

### State Machine

All Intel 400x memory operations follow a common pattern:

```
Idle → AddressPhase → WaitLatency → DriveData → Idle
```

Each phase has specific timing requirements and bus usage patterns.

## Core Traits

### Intel400xClockHandling

Handles the two-phase clock system used by all 400x components:

```rust
pub trait Intel400xClockHandling {
    fn get_base(&self) -> &BaseComponent;

    // Clock edge detection
    fn is_phi1_rising_edge(&self, prev_phi1: PinValue) -> bool;
    fn is_phi1_falling_edge(&self, prev_phi1: PinValue) -> bool;
    fn is_phi2_rising_edge(&self, prev_phi2: PinValue) -> bool;
    fn is_phi2_falling_edge(&self, prev_phi2: PinValue) -> bool;

    // Clock state management
    fn update_clock_states(&self, prev_phi1: &mut PinValue, prev_phi2: &mut PinValue);
}
```

**Key Features:**
- Edge detection for both clock phases
- Previous state tracking for transition detection
- Thread-safe pin access

### Intel400xDataBus

Manages the 4-bit multiplexed address/data bus:

```rust
pub trait Intel400xDataBus {
    fn get_base(&self) -> &BaseComponent;

    // Bus operations
    fn read_data_bus(&self) -> u8;           // Read 4-bit value from D0-D3
    fn write_data_bus(&self, data: u8);      // Drive 4-bit value on D0-D3
    fn tri_state_data_bus(&self);            // Set bus to high impedance
}
```

**Key Features:**
- Safe pin access with error handling
- Driver name tracking for debugging
- Bus contention avoidance

### Intel400xAddressHandling

Handles the two-phase addressing used by 400x components:

```rust
pub trait Intel400xAddressHandling {
    fn get_base(&self) -> &BaseComponent;

    // Address operations
    fn assemble_full_address(&self, high_nibble: Option<u8>, low_nibble: Option<u8>) -> Option<u16>;
    fn handle_address_latching(&self, nibble: u8, ...);  // Complex latching logic
    fn handle_latency_wait(&self, address_latch_time: &Option<Instant>, access_time: Duration) -> bool;
}
```

**Key Features:**
- Two-phase address latching (high nibble, then low nibble)
- Timing-aware address assembly
- Access time validation

### Intel400xControlPins

Reads control signals common to all 400x components:

```rust
pub trait Intel400xControlPins {
    fn get_base(&self) -> &BaseComponent;

    // Control signal reading
    fn read_sync_pin(&self) -> bool;    // SYNC signal state
    fn read_cm_rom_pin(&self) -> bool;   // CM-ROM chip select
    fn read_reset_pin(&self) -> bool;    // RESET signal state
}
```

**Key Features:**
- Safe pin reading with fallback values
- Consistent pin naming across components

### Intel400xResetHandling

Provides reset functionality for all 400x components:

```rust
pub trait Intel400xResetHandling {
    fn get_base(&self) -> &BaseComponent;

    // Reset operations
    fn handle_reset(&self, reset_pin_name: &str) -> bool;
    fn perform_reset(&self);  // Component-specific reset logic
}
```

**Key Features:**
- System-wide reset signal handling
- Component-specific reset behaviors

### Intel400xTimingState

Manages memory operation timing and state machines:

```rust
pub trait Intel400xTimingState {
    // State management
    fn get_timing_state(&self) -> TimingState;
    fn set_timing_state(&mut self, state: TimingState);

    // Address tracking
    fn get_address_latch_time(&self) -> Option<Instant>;
    fn set_address_latch_time(&mut self, time: Option<Instant>);

    // Address components
    fn get_full_address_ready(&self) -> bool;
    fn get_address_high_nibble(&self) -> Option<u8>;
    fn get_address_low_nibble(&self) -> Option<u8>;

    // Timing configuration
    fn get_access_time(&self) -> Duration;
}
```

**Key Features:**
- Unified state machine interface
- Timing constraint management
- Address tracking and validation

## Timing Constants

### Standard Timing Values

```rust
impl TimingConstants {
    pub const DEFAULT_ACCESS_TIME: Duration = Duration::from_nanos(500); // 500ns default
    pub const FAST_ACCESS_TIME: Duration = Duration::from_nanos(200);    // 200ns for shift registers
    pub const ADDRESS_SETUP: Duration = Duration::from_nanos(100);       // Address setup time
    pub const DATA_VALID: Duration = Duration::from_nanos(200);          // Data valid delay
}
```

### Usage in Components

Each component type uses appropriate timing constants:

- **ROM (4001)**: 500ns typical access time
- **RAM (4002)**: 500ns access time with refresh overhead
- **Shift Register (4003)**: 200ns fast access time
- **CPU (4004)**: Internal operation timing

## State Machine

### Memory Operation States

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimingState {
    Idle,         // No memory operation in progress
    AddressPhase, // Currently latching address nibbles
    WaitLatency,  // Address latched, waiting for access time
    DriveData,    // Latency elapsed, driving data on bus
}
```

### State Transition Logic

1. **Idle → AddressPhase**: Triggered by SYNC high + Φ1 rising edge
2. **AddressPhase → WaitLatency**: After both address nibbles latched
3. **WaitLatency → DriveData**: After access time has elapsed
4. **DriveData → Idle**: On Φ2 falling edge (bus tri-stated)

### Hardware Timing

Each state has specific timing requirements:

- **Address Setup**: 100ns minimum before latching
- **Data Valid**: 200ns maximum from address valid to data available
- **Bus Hold**: Data held until Φ2 falling edge
- **Tri-state Delay**: 150ns maximum to high impedance after disable

## Utility Functions

### Pin Operations

```rust
pub mod utils {
    // Safe pin operations with error handling
    pub fn is_pin_high(pin: &Arc<Mutex<Pin>>) -> bool;
    pub fn is_pin_high_z(pin: &Arc<Mutex<Pin>>) -> bool;
    pub fn read_pin_safe(pin: &Arc<Mutex<Pin>>, default: PinValue) -> PinValue;
    pub fn set_pin_driver_safe(pin: &Arc<Mutex<Pin>>, driver_name: String, value: PinValue);

    // Component naming and identification
    pub fn create_driver_name(component_name: &str, suffix: &str) -> String;
}
```

### Error Handling

All pin operations include proper error handling:
- **Lock failures**: Return safe default values
- **Pin not found**: Return appropriate error messages
- **Invalid states**: Graceful degradation

## Testing

### Comprehensive Test Suite

The common functionality is thoroughly tested with:

1. **Unit Tests**: Test individual functions and algorithms
2. **Mock Tests**: Test with controlled hardware inputs
3. **Property Tests**: Verify behavioral properties
4. **Integration Tests**: Test with real component implementations

### Testable Areas

- **Timing constants validation**
- **Address assembly algorithms**
- **State machine transitions**
- **Clock edge detection accuracy**
- **Data bus bit operations**
- **Pin operation safety**

### Mock System

A comprehensive mock system allows testing without real hardware:

```rust
// Create mock component for testing
let mut mock_component = MockIntel400xComponent::new("TestComponent");

// Set up test scenario
mock_component.set_clock_values(PinValue::High, PinValue::Low);

// Test behavior
assert!(mock_component.is_phi1_rising_edge(PinValue::Low));
```

## Usage in Components

### Implementation Pattern

Components implement the relevant traits:

```rust
impl Intel400xClockHandling for MyComponent {
    fn get_base(&self) -> &BaseComponent {
        &self.base
    }
}

impl Intel400xDataBus for MyComponent {
    fn get_base(&self) -> &BaseComponent {
        &self.base
    }

    fn read_data_bus(&self) -> u8 {
        // Use common data bus reading logic
        let mut data = 0;
        for i in 0..4 {
            if let Ok(pin) = self.get_base().get_pin(&format!("D{}", i)) {
                if let Ok(pin_guard) = pin.lock() {
                    if pin_guard.read() == PinValue::High {
                        data |= 1 << i;
                    }
                }
            }
        }
        data & 0x0F
    }
}
```

### Benefits

1. **Code Reuse**: Common functionality shared across components
2. **Consistency**: Uniform behavior across all 400x components
3. **Maintainability**: Changes to common logic update all components
4. **Testability**: Common code can be tested once and reused
5. **Type Safety**: Traits ensure proper implementation

## Hardware Accuracy

### Timing Considerations

The common code implements hardware-accurate timing where possible:

- **Clock Edge Detection**: Proper Φ1/Φ2 edge detection
- **Address Latching**: Two-phase address capture
- **Access Time Simulation**: Configurable access latencies
- **Bus State Management**: Proper drive/tri-state timing

### Limitations

- **Rust Timing**: Limited by Rust's timing resolution
- **Threading**: Simulation runs in single thread vs. hardware parallelism
- **Signal Propagation**: Simplified signal timing for simulation

## Future Enhancements

### Planned Improvements

1. **Enhanced Timing Simulation**
   - More accurate sub-nanosecond timing
   - Better clock phase relationship modeling
   - Improved signal propagation delays

2. **Extended Component Support**
   - Support for additional Intel 400x variants
   - Enhanced I/O device support
   - Better peripheral modeling

3. **Testing Improvements**
   - More sophisticated mock scenarios
   - Better timing verification tools
   - Enhanced debugging capabilities

### Extension Points

The trait-based design makes it easy to:

- **Add New Components**: Implement the required traits
- **Enhance Timing**: Modify timing constants and logic
- **Improve Testing**: Add new mock scenarios and test utilities
- **Extend Functionality**: Add new common behaviors as needed

## Dependencies

The common functionality depends on:

- **Base Component System**: For pin management and lifecycle
- **Pin Abstraction**: For signal state management
- **Timing Utilities**: For duration and instant handling
- **Error Handling**: For robust operation

## Performance

### Optimization Features

- **Efficient Pin Access**: Minimized lock contention
- **Fast State Transitions**: Optimized state machine
- **Minimal Allocations**: Reuse of common data structures
- **Lock-Free Algorithms**: Where possible for performance

### Memory Usage

- **Small Footprint**: Minimal memory overhead per component
- **Shared Code**: No duplication across components
- **Efficient State Storage**: Compact state representation

This common functionality module provides a solid foundation for all Intel 400x components while maintaining high performance and hardware accuracy.