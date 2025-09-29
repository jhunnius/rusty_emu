# Hardware Components

This directory contains implementations of various hardware components used in the Intel 4004 microprocessor system and
other architectures.

## Component Architecture

All components implement the base `Component` trait defined in `src/component.rs`, which provides:

- **Pin Management**: Components expose named pins for interconnection
- **Lifecycle Management**: Components can be started, stopped, and updated
- **Thread Safety**: All components are `Send + Sync` for concurrent access
- **State Management**: Components maintain internal state and respond to clock cycles

## Component Categories

### CPU Components (`cpu/`)

#### Intel 4004 CPU

- **File**: `intel_4004.rs`
- **Features**:
    - Complete Intel 4004 instruction set implementation
    - 4-bit data processing
    - 12-bit address space
    - Two-phase clock operation (Φ1, Φ2)
    - 16 general-purpose registers (8 index, 8 main)
    - Stack-based subroutine calls
- **Status**: Fully implemented with comprehensive testing

#### MOS 6502 CPU (Placeholder)

- **File**: `mos_6502.rs`
- **Features**: Placeholder for MOS Technology 6502 implementation
- **Status**: Not yet implemented

#### WDC 65C02 CPU (Placeholder)

- **File**: `wdc_65c02.rs`
- **Features**: Placeholder for Western Design Center 65C02 implementation
- **Status**: Not yet implemented

### Memory Components (`memory/`)

#### Intel 4001 ROM

- **File**: `intel_4001.rs`
- **Features**:
    - 256 bytes of mask-programmable ROM
    - 4-bit I/O ports for peripheral interface
    - Two-phase addressing (8-bit address in two 4-bit cycles)
    - 500ns typical access time
    - SYNC signal detection for instruction fetch
- **Status**: Fully implemented with comprehensive testing

#### Intel 4002 RAM

- **File**: `intel_4002.rs`
- **Features**:
    - 320 bits of read/write memory (80 × 4-bit characters)
    - 4-bit output port for status/display
    - Banked memory organization (4 banks × 20 characters)
    - Refresh circuitry for dynamic memory cells
    - Status character instructions for control
- **Status**: Implemented with known limitations

#### Intel 4003 Shift Register

- **File**: `intel_4003.rs`
- **Features**:
    - 10-bit static shift register
    - Serial input/output capability
    - Parallel input from data bus
    - 200ns typical access time
    - Bidirectional shifting
- **Status**: Basic implementation

#### Generic Memory Components

- **GenericRAM**: `generic_ram.rs` - Configurable RAM implementation
- **GenericROM**: `generic_rom.rs` - Configurable ROM implementation

### Common Functionality (`common/`)

#### Intel 400x Series Common Code

- **File**: `intel_400x.rs`
- **Purpose**: Shared functionality for all Intel 400x series chips
- **Features**:
    - Common timing constants and state machines
    - Clock edge detection and handling
    - Data bus operations and bit manipulation
    - Address latching and assembly logic
    - Control pin reading and management
    - Reset handling and initialization
    - Memory operation state machines

### Clock Components (`clock/`)

#### Generic Clock Generator

- **File**: `generic_clock.rs`
- **Features**:
    - Configurable frequency and duty cycle
    - Multiple output waveforms
    - Synchronization capabilities
    - Test pattern generation
- **Status**: Basic implementation

## Component Interface

### Required Methods

All components must implement:

```rust
fn name(&self) -> String;                    // Component identifier
fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>>;  // Pin configuration
fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String>; // Pin access
fn update(&mut self);                        // Process one clock cycle
fn run(&mut self);                          // Continuous execution
fn stop(&mut self);                         // Graceful shutdown
fn is_running(&self) -> bool;               // Execution status
```

### Optional Traits

Components can implement additional traits for specialized functionality:

- **RunnableComponent**: Automatic thread spawning
- **CPU**: Processor-specific operations
- **Memory**: Storage-specific operations

## Pin Configuration

### Standard Pin Types

- **Data Pins**: `D0-D3` for 4-bit data/address bus
- **Clock Pins**: `PHI1`, `PHI2` for two-phase clock
- **Control Pins**: `SYNC`, `RESET`, `CM`, `CI` for system control
- **I/O Pins**: `IO0-IO3` for peripheral interface

### Pin Signal States

- **High**: Logic level 1
- **Low**: Logic level 0
- **HighZ**: High impedance (tri-state, not driving)

## Usage Examples

### Creating and Using a Component

```rust
use rusty_emu::components::memory::intel_4001::Intel4001;

// Create a ROM component
let mut rom = Intel4001::new("ROM1".to_string());

// Load program data
let program = vec![0x12, 0x34, 0x56, 0x78];
rom.load_rom_data(program, 0) ?;

// Use in simulation loop
while simulation_running() {
rom.update(); // Process one clock cycle
}
```

### Component Interconnection

```rust
// Components connect through shared pin references
let cpu_pin = cpu.get_pin("D0") ?;
let rom_pin = rom.get_pin("D0") ?;
let ram_pin = ram.get_pin("D0") ?;

// Connect pins together
connect_pins(vec![cpu_pin, rom_pin, ram_pin]) ?;
```

## Testing Strategy

### Unit Tests

- Located in each component's implementation file
- Test individual component behavior in isolation
- Focus on state management and pin operations

### Integration Tests

- Located in `tests/` directory
- Test component interactions and system behavior
- Verify proper timing and signal propagation

### Mock-Based Tests

- Use mock components for controlled testing
- Test error conditions and edge cases
- Verify behavior with various input patterns

## Development Guidelines

### Adding New Components

1. **Implement Core Traits**: Start with `Component` trait implementation
2. **Define Pin Configuration**: Specify required pins and their purposes
3. **Implement Timing**: Add clock cycle processing in `update()` method
4. **Add Specialized Traits**: Implement CPU, Memory, or other specialized traits as needed
5. **Write Tests**: Add comprehensive unit tests for the new component
6. **Update Exports**: Add the component to the appropriate module exports

### Component Design Principles

- **Single Responsibility**: Each component should have one primary function
- **Clear Interfaces**: Well-defined pin configurations and behaviors
- **Timing Awareness**: Proper handling of clock cycles and timing constraints
- **Error Handling**: Graceful handling of invalid inputs and states
- **Testability**: Design for easy testing and verification

### Performance Considerations

- **Pin Access**: Minimize lock contention on shared pins
- **State Updates**: Efficient state machine transitions
- **Memory Layout**: Optimize memory access patterns
- **Clock Processing**: Minimize overhead in update cycles

## Hardware Accuracy

### Intel 4004 Series Specifics

- **Two-Phase Clock**: All operations synchronized to Φ1 and Φ2
- **4-Bit Architecture**: Data processed in 4-bit nibbles
- **12-Bit Addressing**: 8-bit addresses transferred in two 4-bit cycles
- **Timing Constraints**: Specific setup and hold times for signals
- **Bus Sharing**: Multiple components share the same data bus

### Implementation Notes

- Some timing specifications are approximated due to Rust's timing limitations
- Pin signal propagation is simplified for simulation performance
- Bus contention handling prioritizes correctness over perfect hardware accuracy
- Memory refresh cycles may be abstracted for simplicity

## Future Enhancements

### Planned Components

- Enhanced Intel 4002 RAM with proper refresh cycles
- Intel 4008/4009 I/O expanders
- Intel 4269 programmable ROM
- More accurate timing simulation
- Cycle-accurate instruction execution

### Architecture Improvements

- Better pin signal propagation modeling
- More sophisticated bus arbitration
- Enhanced timing constraint checking
- Improved debugging and introspection capabilities