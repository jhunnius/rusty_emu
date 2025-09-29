# System Integration

This directory contains complete system implementations that integrate multiple components into functional microprocessor systems.

## System Architecture

Systems represent complete working computers built from individual components. They handle:

- **Component Interconnection**: Connecting pins between components
- **Clock Distribution**: Providing synchronized clocks to all components
- **Bus Management**: Coordinating access to shared buses
- **System Lifecycle**: Initialization, execution, and shutdown
- **Program Loading**: Loading and executing software

## Current Systems

### Intel MCS-4 System

#### Overview
The Intel MCS-4 (Micro Computer System) was Intel's first microprocessor system, consisting of:

- **Intel 4004 CPU**: 4-bit central processing unit
- **Intel 4001 ROM**: Program storage with I/O ports
- **Intel 4002 RAM**: Data storage with output ports
- **Intel 4003 Shift Register**: Serial I/O interface

#### Architecture

```
┌─────────────────┐    ┌─────────────────┐
│   Intel 4004    │    │   Intel 4001    │
│       CPU       │◄──►│      ROM        │
└─────────────────┘    └─────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│   Intel 4002    │    │   Intel 4003    │
│      RAM        │    │  Shift Register │
└─────────────────┘    └─────────────────┘
```

#### System Components

**CPU Subsystem**
- Intel 4004 processor with full instruction set
- Program counter and stack management
- Register file (16 × 4-bit registers)
- ALU with carry/borrow logic

**Memory Subsystem**
- ROM units for program storage
- RAM units for data storage
- Memory-mapped I/O ports
- Banked memory organization

**I/O Subsystem**
- Shift registers for serial communication
- Output ports for status display
- Input ports for external data

#### Clock System

The MCS-4 uses a two-phase clock system:

- **Φ1 (Phase 1)**: CPU drives address bus, memory latches addresses
- **Φ2 (Phase 2)**: Memory drives data bus, CPU reads data

Clock timing is critical for proper system operation:

```
┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐
│     │     │     │     │     │     │     │
│  Φ1 │     │  Φ2 │     │  Φ1 │     │  Φ2 │
│     │     │     │     │     │     │     │
└─────┘     └─────┘     └─────┘     └─────┘
  T1          T2          T3          T4
```

#### Memory Organization

**Address Space**
- **12-bit address bus** (4096 possible locations)
- **4-bit data bus** (nibbles)
- **8-bit instructions** (2 nibbles each)
- **16-bit addresses** (4 nibbles each)

**Memory Map**
- ROM and RAM share the same address space
- Chip select signals determine which memory responds
- I/O ports are memory-mapped

#### Implementation Status

**Currently Implemented**
- ✅ Basic system structure and component interconnection
- ✅ Clock generation and distribution
- ✅ Memory subsystem with ROM and RAM
- ✅ CPU integration with memory
- ✅ Basic I/O port functionality

**Partially Implemented**
- ⚠️ Complete instruction execution pipeline
- ⚠️ Interrupt handling system
- ⚠️ DMA (Direct Memory Access) support
- ⚠️ Advanced I/O device support

**Not Yet Implemented**
- ❌ Complex peripheral devices
- ❌ Operating system integration
- ❌ Development tools integration
- ❌ Performance optimization

## System Integration Patterns

### Component Connection

Components connect through shared pin references:

```rust
// Create system components
let cpu = Intel4004::new("CPU".to_string());
let rom = Intel4001::new("ROM".to_string());

// Connect data bus pins
let cpu_data_pin = cpu.get_pin("D0")?;
let rom_data_pin = rom.get_pin("D0")?;
connect_pins(cpu_data_pin, rom_data_pin)?;

// Connect clock signals
let cpu_clock_pin = cpu.get_pin("PHI1")?;
let rom_clock_pin = rom.get_pin("PHI1")?;
connect_pins(cpu_clock_pin, rom_clock_pin)?;
```

### Clock Synchronization

All components receive synchronized clock signals:

```rust
// Create clock generator
let clock = GenericClock::new("SystemClock".to_string());

// Distribute clock to all components
let clock_output = clock.get_pin("OUT")?;
cpu.connect_clock(clock_output.clone())?;
rom.connect_clock(clock_output.clone())?;
ram.connect_clock(clock_output)?;
```

### Bus Arbitration

Multiple components may drive the same bus:

```rust
// CPU drives bus during Φ1
cpu.set_bus_driver(true)?;

// Memory drives bus during Φ2
rom.set_bus_driver(true)?;
ram.set_bus_driver(true)?;

// System manages arbitration
system.arbitrate_bus_access()?;
```

## Usage Examples

### Basic System Setup

```rust
use rusty_emu::systems::intel_mcs_4::IntelMCS4;

// Create complete MCS-4 system
let mut system = IntelMCS4::new();

// Load program into ROM
let program = vec![
    0x12, 0x34,  // Sample instructions
    0x56, 0x78,
    // ... more program data
];
system.load_program(0, &program)?;

// Run the system
system.run();
```

### System Configuration

```rust
// Configure system parameters
system.set_clock_frequency(750_000.0)?; // 750 kHz
system.set_memory_size(1024)?;          // 1KB total memory
system.enable_debug_mode(true)?;        // Enable detailed logging

// Add peripheral devices
system.add_peripheral("Terminal".to_string(), terminal_device)?;
system.add_peripheral("Storage".to_string(), storage_device)?;
```

### Program Development

```rust
// Load and debug programs
system.load_program_from_file("program.bin")?;

// Set breakpoints
system.set_breakpoint(0x100)?;

// Run with debugging
system.run_with_debug()?;

// Inspect system state
println!("PC: 0x{:04X}", system.get_program_counter()?);
println!("ACC: 0x{:02X}", system.get_accumulator()?);
```

## Testing Strategy

### System-Level Tests

System integration requires comprehensive testing:

1. **Component Interaction Tests**
   - Verify CPU can read from ROM
   - Verify CPU can write to RAM
   - Verify I/O operations work correctly

2. **Timing Verification Tests**
   - Verify clock synchronization
   - Verify bus timing constraints
   - Verify memory access timing

3. **Program Execution Tests**
   - Test simple programs execute correctly
   - Test I/O operations
   - Test error conditions

### Debugging Support

The system provides debugging capabilities:

```rust
// Enable debug tracing
system.enable_tracing(true)?;

// Set up debug output
system.set_debug_output_file("debug.log")?;

// Monitor specific signals
system.monitor_pin("CPU", "D0")?;
system.monitor_pin("ROM", "DATA_READY")?;
```

## Performance Considerations

### Simulation Performance

System simulation performance depends on:

- **Component Count**: More components = slower simulation
- **Clock Frequency**: Higher frequency = more updates per second
- **Debug Output**: Logging reduces performance
- **Pin Monitoring**: Signal monitoring adds overhead

### Optimization Strategies

1. **Selective Updates**: Only update components that need attention
2. **Event-Driven Simulation**: Respond to events rather than polling
3. **Lazy Evaluation**: Delay expensive operations until needed
4. **Caching**: Cache frequently accessed state

## Hardware Accuracy

### MCS-4 Specifics

The implementation aims for hardware accuracy:

- **Cycle-Accurate Timing**: Where possible given Rust limitations
- **Proper Signal Levels**: Correct High/Low/HighZ states
- **Bus Contention Handling**: Proper arbitration and error detection
- **Clock Phase Relationships**: Correct Φ1/Φ2 timing

### Known Limitations

- **Timing Precision**: Limited by OS scheduling and Rust timing
- **Parallel Execution**: Hardware runs components in parallel, simulation is sequential
- **Signal Propagation**: Simplified delay modeling
- **Power Simulation**: No power consumption modeling

## Future Enhancements

### Planned Features

1. **Enhanced I/O Support**
   - Serial communication interfaces
   - Parallel I/O expansion
   - Analog-to-digital conversion
   - Real-time clock support

2. **Development Tools Integration**
   - Assembler integration
   - Debugger interface
   - Program loading and saving
   - Memory inspection tools

3. **Performance Improvements**
   - Multi-threaded component execution
   - Optimized timing simulation
   - Memory layout optimization
   - Caching for frequently accessed data

4. **Extended System Support**
   - MCS-40 system support
   - Enhanced peripheral set
   - Networking capabilities
   - Storage device emulation

### Extension Points

The system architecture is designed for extensibility:

- **New Component Types**: Easy addition of new hardware
- **System Variants**: Support for different MCS-4 configurations
- **Custom Peripherals**: Integration of specialized devices
- **Alternative Architectures**: Framework for other processor families

## Troubleshooting

### Common Issues

1. **Clock Synchronization Problems**
   - Verify all components receive the same clock signal
   - Check clock phase relationships
   - Ensure proper setup/hold time compliance

2. **Bus Contention Issues**
   - Verify only one component drives the bus at a time
   - Check tri-state timing
   - Monitor for conflicting drivers

3. **Memory Access Problems**
   - Verify address decoding is correct
   - Check chip select signal timing
   - Ensure proper memory bank selection

### Debug Tools

The system provides debugging utilities:

```rust
// Enable verbose logging
system.set_log_level(LogLevel::Debug)?;

// Monitor all pin states
system.monitor_all_pins()?;

// Generate timing diagrams
system.generate_timing_diagram("timing.txt")?;
```

This system integration provides a solid foundation for building complete microprocessor systems with proper component interaction, timing, and debugging support.