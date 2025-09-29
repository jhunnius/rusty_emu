# Intel MCS-4 Emulator Console Interface

## Overview

The console interface provides an interactive terminal-based user interface for the Intel MCS-4 emulator, replacing the need to use Ctrl+C to interrupt execution. It offers real-time system monitoring, debugging capabilities, and user controls through an ncurses-like interface.

## Features

- **Real-time System Monitoring**: View system status, RAM contents, and CPU registers
- **Interactive Commands**: Control system execution with simple commands
- **Multiple Display Panes**: Switch between different views of system state
- **Configurable Display**: Customize refresh rates and display options
- **Graceful Exit**: Clean shutdown without needing Ctrl+C

## Usage

### Basic Usage

```bash
# Enable console interface with basic system
cargo run -- --console --system basic

# Enable console with max system
cargo run -- --console --system max

# Enable console with custom configuration
cargo run -- --console --system custom_config.json
```

### Command Line Options

- `-c, --console`: Enable the interactive console interface
- `-s, --system <SYSTEM>`: System type (basic, max, fig1, or JSON file)
- `-f, --file <FILE>`: Program binary file to load
- `-h, --help`: Show help message

## Console Commands

### Basic Commands

- `quit`, `exit`, `q` - Exit the emulator
- `run`, `r` - Start system execution
- `stop`, `s` - Stop system execution
- `help`, `h` - Show/hide help screen
- `reset` - Reset the system

### Navigation

- `Tab` - Switch between display panes
- `Enter` - Execute command
- `Backspace` - Delete character from command line

## Display Layout

The console interface is divided into three main areas:

### Title Bar (Top)
- Shows current system status
- Displays available commands
- Indicates which pane is currently selected

### Main Content (Middle)
Two-pane layout showing:

**Left Pane - System Information:**
- System name and description
- Component count and CPU speed
- ROM/RAM sizes
- Current execution status

**Right Pane - RAM Contents:**
- Real-time RAM bank contents
- Memory layout visualization
- Bank-by-bank display

### Command Bar (Bottom)
- Command input area
- Shows current command being typed
- Displays help text when active

## Configuration

### XML Configuration File

The console can be configured using an XML file (`configs/console_config.xml`):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<console_config>
    <interface>
        <enabled>true</enabled>
        <refresh_rate_ms>100</refresh_rate_ms>
        <show_system_info>true</show_system_info>
        <show_registers>true</show_registers>
        <show_ram>true</show_ram>
        <ram_banks_per_row>4</ram_banks_per_row>
        <max_ram_rows>5</max_ram_rows>
    </interface>

    <display>
        <colors>
            <title_bar>cyan</title_bar>
            <command_bar>white</command_bar>
            <system_info>white</system_info>
            <ram_contents>white</ram_contents>
        </colors>
    </display>
</console_config>
```

### Configuration Options

- `enabled`: Enable/disable console interface
- `refresh_rate_ms`: Screen refresh rate in milliseconds
- `show_system_info`: Show system information pane
- `show_registers`: Show CPU registers pane
- `show_ram`: Show RAM contents pane
- `ram_banks_per_row`: Number of RAM banks to display per row
- `max_ram_rows`: Maximum number of RAM rows to display

## Display Features

### System Information
- Real-time system status
- Component counts and specifications
- Execution state monitoring

### RAM Display
- Bank-by-bank memory visualization
- Hexadecimal data representation
- Configurable layout options

### Register Display
- CPU register contents
- Program counter and accumulator
- Index register status

## Integration with Existing Systems

The console interface integrates seamlessly with the existing system architecture:

1. **System Factory**: Uses the same JSON configuration system
2. **Component Access**: Reads data from existing components
3. **Thread Safety**: Uses Arc<Mutex<>> for safe concurrent access
4. **Backward Compatibility**: Original command-line interface still available

## Dependencies

The console interface requires additional dependencies:

```toml
[dependencies]
crossterm = "0.27"  # Terminal handling
ratatui = "0.24"    # TUI framework
```

## Future Enhancements

Planned features for future versions:

- **Memory Editing**: Modify RAM contents during execution
- **Breakpoint Support**: Set breakpoints at specific addresses
- **Step Execution**: Single-step through instructions
- **Register Editing**: Modify CPU registers
- **Disassembly View**: Show disassembled instructions
- **Performance Metrics**: Display execution statistics
- **Log Viewer**: View system logs and debug information

## Troubleshooting

### Common Issues

1. **Terminal Not Supported**: Ensure your terminal supports the required features
2. **Display Issues**: Try adjusting the refresh rate in configuration
3. **Input Not Working**: Check that the terminal is in raw mode

### Debug Mode

For debugging console issues, you can:

1. Check terminal capabilities
2. Verify configuration file syntax
3. Test with minimal configuration

## Examples

### Basic Console Usage

```bash
# Start console with basic system
cargo run -- --console --system basic

# The console will show:
# - System information in left pane
# - RAM contents in right pane
# - Command bar at bottom
```

### Custom Configuration

```bash
# Use custom console configuration
cargo run -- --console --config custom_console.xml --system max
```

### Development Testing

```bash
# Test console interface during development
cargo run -- --console --system basic --file programs/test.bin
```

## Architecture

The console interface follows a modular architecture:

- **ConsoleApp**: Main application state and event handling
- **Display Modules**: Separate rendering for each UI component
- **Input Handler**: Keyboard and mouse input processing
- **System Interface**: Safe access to emulator state
- **Configuration**: XML-based configuration management

This design allows for easy extension and customization of the interface.