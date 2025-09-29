# Intel 400x Common Functionality Tests

This directory contains comprehensive tests for the `intel_400x.rs` common functionality module, which provides shared
timing, clock handling, and bus operations for Intel 400x series chips.

## Test Organization

### Unit Tests (in implementation files)

- **Intel4001 unit tests**: Located in `src/components/memory/intel_4001.rs`
    - Component creation and configuration
    - Memory operations and data loading
    - State management and transitions
    - Error handling and edge cases

### Integration Tests (in tests/ directory)

1. **`intel_400x_tests.rs`** - Tests for the common intel_400x functionality
2. **`mocks.rs`** - Mock implementations for testing hardware interactions
3. **`mock_based_tests.rs`** - Tests using mocks for data bus and pin operations
4. **`property_based_tests.rs`** - Property-based tests for state machine verification
5. **`integration_tests.rs`** - Integration tests with concrete chip implementations
6. **`working_test.rs`** - Working demonstration of core functionality
7. **`lib.rs`** - Test library and common utilities

### Test Categories

#### 1. Common Functionality Tests (`intel_400x_tests.rs`)

- **Timing Constants**: Verify access times and setup requirements
- **Address Assembly**: Test nibble-to-address conversion logic
- **State Machine**: Test state transition logic and queries
- **Clock Edge Detection**: Test edge detection algorithms
- **Data Bus Operations**: Test bit manipulation for 4-bit bus
- **Utility Functions**: Test helper functions and pin operations

#### 2. Mock-Based Tests (`mock_based_tests.rs`)

- **Data Bus Operations**: Test read/write operations with controlled inputs
- **Clock Handling**: Test clock pin interactions and edge scenarios
- **Control Pins**: Test SYNC, CM, and RESET pin behavior
- **Timing State**: Test state machine with mock timing
- **Error Handling**: Test behavior with invalid inputs

#### 3. Property-Based Tests (`property_based_tests.rs`)

- **State Machine Invariants**: Verify state consistency properties
- **Type Conversions**: Test enum conversion roundtrips
- **Address Properties**: Test address assembly properties
- **Timing Properties**: Test duration and timing constraints
- **Concurrency Safety**: Test thread-safe behavior

#### 4. Integration Tests (`integration_tests.rs`)

- **Trait Implementation**: Verify concrete types implement traits correctly
- **Cross-Component Compatibility**: Test multiple components work together
- **Real Component Testing**: Test with actual Intel4001 implementation
- **Lifecycle Testing**: Test component initialization and cleanup

#### 5. Working Tests (`working_test.rs`)

- **Core Functionality**: Demonstrates that the intel_400x module is testable
- **Basic Operations**: Tests fundamental features that work correctly
- **Regression Prevention**: Ensures core functionality remains stable

## Running the Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Categories

```bash
# Unit tests only
cargo test intel_400x_tests

# Mock-based tests only
cargo test mock_based_tests

# Property-based tests only
cargo test property_based_tests

# Integration tests only
cargo test integration_tests

# Demo tests only
cargo test demo
```

### Run with Detailed Output

```bash
cargo test -- --nocapture
```

### Run Specific Test

```bash
cargo test test_name
```

## Test Architecture

### Mock System

The test suite includes a comprehensive mock system that provides:

- **MockPin**: Simulates pin behavior with operation counting
- **MockIntel400xComponent**: Full component mock implementing all traits
- **MockScenario**: Test scenario builder with common setups
- **MockTimeProvider**: Deterministic time source for timing tests

### Property-Based Testing

Uses `proptest` to verify:

- State machine invariants hold for all inputs
- Type conversions are consistent
- Address assembly works for all valid inputs
- Timing constraints are maintained

### Integration Testing

Tests verify that:

- Concrete implementations (like Intel4001) correctly use the common traits
- Multiple components can interact through the common interface
- The common functionality works correctly in real usage scenarios

## Key Testing Areas

### 1. Timing and Clock Logic

- Clock edge detection accuracy
- Two-phase clock handling
- Access time simulation
- State machine timing

### 2. Address Handling

- Nibble assembly into full addresses
- Address latching during clock phases
- Address validation and bounds checking

### 3. Data Bus Operations

- 4-bit data bus read/write operations
- Bus tri-stating for contention avoidance
- Bit-level manipulation and verification

### 4. State Machine Verification

- State transition correctness
- State invariant preservation
- Reset behavior verification
- Error state handling

### 5. Hardware Abstraction

- Pin operation safety
- Driver management
- Hardware timing simulation
- Bus contention avoidance

## Adding New Tests

When adding new functionality to `intel_400x.rs`, add corresponding tests:

1. **Unit tests** for pure functions in `intel_400x_tests.rs`
2. **Mock tests** for hardware interactions in `mock_based_tests.rs`
3. **Property tests** for behavioral properties in `property_based_tests.rs`
4. **Integration tests** for concrete usage in `integration_tests.rs`

## Test Data

The tests use realistic data patterns that reflect actual Intel 4004/4001 usage:

- Address ranges: 0x0000 to 0x0FFF (12-bit addressing)
- Data values: 0x00 to 0x0F (4-bit data)
- Timing values: nanosecond precision for hardware simulation
- Pin states: High, Low, and High-Z (tri-state)

## Dependencies

The test suite uses these testing libraries:

- `proptest`: Property-based testing
- `mockall`: Mock generation (if needed)
- `pretty_assertions`: Enhanced assertion messages
- `std::time`: Timing simulation

## Continuous Integration

These tests are designed to run in CI environments and provide:

- Deterministic results (no race conditions)
- Reasonable execution time
- Clear failure messages
- Comprehensive coverage of edge cases

## Troubleshooting

If tests fail:

1. Check that timing-related tests account for system timing variations
2. Verify that mock setups match the expected hardware behavior
3. Ensure that property-based tests have appropriate generation strategies
4. Check that integration tests use the correct trait implementations

The test suite is designed to be robust and provide clear feedback about what functionality is broken and why.