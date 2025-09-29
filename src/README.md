# Source Code Architecture

This directory contains the core implementation of the Rusty Emulator, an Intel 4004/4001 microprocessor simulator written in Rust.

## Architecture Overview

```
src/
├── lib.rs                 # Library root and main exports
├── main.rs               # Binary entry point
├── component.rs          # Core component trait definitions
├── pin.rs               # Pin and signal abstractions
├── connection.rs        # Component interconnection
├── bus.rs              # Data bus implementations
├── types.rs            # Common type definitions
├── components/         # Hardware component implementations
│   ├── mod.rs         # Component module exports
│   ├── common/        # Shared functionality
│   │   ├── mod.rs    # Common exports
│   │   └── intel_400x.rs  # Intel 400x common traits
│   ├── cpu/          # CPU implementations
│   ├── memory/       # Memory component implementations
│   └── clock/       # Clock generation components
└── systems/         # Complete system integrations
    ├── mod.rs      # System module exports
    └── intel_mcs_4.rs    # Intel MCS-4 system
```

## Core Abstractions

### Component System
The emulator is built around a component-based architecture where each hardware element implements the `Component` trait:

```rust
pub trait Component: Send + Sync {
    fn name(&self) -> String;
    fn pins(&self) -> HashMap<String, Arc<Mutex<Pin>>>;
    fn get_pin(&self, name: &str) -> Result<Arc<Mutex<Pin>>, String>;
    fn update(&mut self);
    fn run(&mut self);
    fn stop(&mut self);
    fn is_running(&self) -> bool;
}
```

### Pin System
All components communicate through pins that can be in one of three states:
- `High` (logic 1)
- `Low` (logic 0)
- `HighZ` (high impedance/tri-state)

### Clocking
The system uses a two-phase clock architecture (Φ1, Φ2) that matches the Intel 4004's requirements.

## Component Categories

### CPU Components (`src/components/cpu/`)
- **Intel4004**: Complete Intel 4004 CPU implementation
- **MOS6502**: MOS Technology 6502 CPU (placeholder)
- **WDC65C02**: Western Design Center 65C02 CPU (placeholder)

### Memory Components (`src/components/memory/`)
- **Intel4001**: 256-byte ROM with 4-bit I/O ports
- **Intel4002**: 320-bit RAM with 4-bit output ports
- **Intel4003**: 10-bit shift register
- **GenericRAM**: Generic RAM implementation
- **GenericROM**: Generic ROM implementation

### Common Functionality (`src/components/common/`)
- **Intel400x**: Shared traits and utilities for all Intel 400x series chips
  - Clock handling and edge detection
  - Data bus operations
  - Address handling and latching
  - Control pin management
  - Reset handling
  - Timing state machines

### Clock Components (`src/components/clock/`)
- **GenericClock**: Configurable clock generator

## System Integration (`src/systems/`)

### Intel MCS-4 System
Complete Intel MCS-4 (Micro Computer System) implementation featuring:
- Intel 4004 CPU
- Intel 4001 ROM units
- Intel 4002 RAM units
- Intel 4003 shift registers
- System clock and timing
- Component interconnection

## Key Features

### Hardware-Accurate Simulation
- Cycle-accurate timing where possible
- Two-phase clock operation
- Proper pin drive and tri-state handling
- Bus contention detection and avoidance

### Extensible Architecture
- Trait-based component system
- Easy addition of new CPU types
- Pluggable memory implementations
- Configurable system topologies

### Testing Support
- Comprehensive test suite
- Mock components for isolated testing
- Property-based testing for verification
- Integration tests for system validation

## Usage Examples

### Basic Component Usage
```rust
use rusty_emu::components::memory::intel_4001::Intel4001;

// Create a ROM component
let mut rom = Intel4001::new("ROM1".to_string());

// Load program data
let program = vec![0x12, 0x34, 0x56, 0x78];
rom.load_rom_data(program, 0)?;

// Use in simulation
rom.update(); // Process one clock cycle
```

### System Integration
```rust
use rusty_emu::systems::intel_mcs_4::IntelMCS4;

// Create complete MCS-4 system
let mut system = IntelMCS4::new();

// Load program into ROM
system.load_program(0, &program_data)?;

// Run simulation
system.run();
```

## Development Guidelines

### Adding New Components
1. Implement the `Component` trait
2. Add appropriate pin configuration
3. Implement timing-accurate behavior
4. Add comprehensive unit tests
5. Update module exports

### Testing Requirements
- Unit tests for individual components
- Integration tests for component interactions
- Property-based tests for behavioral verification
- Mock-based tests for external dependencies

### Code Organization
- Keep components focused on single responsibilities
- Use traits for shared behavior
- Maintain clear separation between hardware abstraction and implementation
- Document hardware deviations and limitations

## Performance Considerations

- Components are designed to be thread-safe
- Pin operations use lock-free algorithms where possible
- Memory access patterns optimized for simulation performance
- Clock cycles processed efficiently

## Hardware Accuracy Notes

- Intel 4004 timing is approximated due to Rust's timing limitations
- Pin signal propagation is simplified for simulation purposes
- Some hardware-specific behaviors may be abstracted for testability
- Bus contention handling prioritizes safety over perfect accuracy