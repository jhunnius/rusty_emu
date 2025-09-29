# MCS-4 Programs

This directory contains binary program files for the Intel MCS-4 emulator.

## Available Programs

### fibonacci.bin
- **Size:** 33 bytes (16 instructions × 2 bytes each + 1 extra byte)
- **Description:** Basic Fibonacci sequence generator (stores results in RAM)
- **Algorithm:** Calculates first 8 Fibonacci numbers using registers and RAM
- **Instructions Used:** LDM, LD, WRM, IAC, ADD, JCN, NOP
- **Output:** Results stored in RAM locations 0-7 (not visible to user)

### fibonacci_output.bin ✨ **RECOMMENDED**
- **Size:** 55 bytes (27 instructions × 2 bytes each + 1 extra byte)
- **Description:** Enhanced Fibonacci sequence generator with output port visibility
- **Algorithm:** Calculates first 8 Fibonacci numbers and outputs them to output ports
- **Instructions Used:** LDM, LD, WRM, IAC, ADD, JCN, NOP, SRC
- **Output:** Each Fibonacci number visible on output ports 0-7 during execution
- **Features:** Real-time visibility of calculation progress

### io_demo.bin ✨ **NEW**
- **Size:** 32 bytes (16 instructions × 2 bytes each)
- **Description:** I/O port demonstration program for Intel 4001
- **Algorithm:** Writes values to I/O ports 0-3 and reads them back
- **Instructions Used:** LDM, SRC, WRM, RDM, JCN
- **Output:** Demonstrates I/O port functionality with visible port states
- **Features:** Shows I/O port read/write operations and latching behavior

## Usage

Programs are automatically loaded by the emulator. Use the `--file` option to specify a different program:

```bash
# Use default fibonacci program (RAM-based)
cargo run -- --system basic

# Use enhanced fibonacci program with output ports ✨
cargo run -- --system basic --file programs/fibonacci_output.bin

### io_demo.bin ✨ **NEW**
- **Size:** Assembly source (needs compilation to binary)
- **Description:** I/O port demonstration program for Intel 4001
- **Algorithm:** Writes values 1-4 to I/O ports 0-3 and reads them back
- **Instructions Used:** LDM, SRC, WRM, RDM, JCN
- **Output:** Demonstrates I/O port functionality with visible port states
- **Features:** Shows I/O port read/write operations and latching behavior

# Use I/O demonstration program ✨
cargo run -- --system mcs4_io_demo --file programs/io_demo.bin

# Use specific program
cargo run -- --system basic --file programs/myprogram.bin
```

## Adding New Programs

1. Create your MCS-4 assembly program
2. Assemble it into raw binary format (big-endian byte pairs)
3. Save as `.bin` file in this directory
4. Update this README with program details

## Program Format

MCS-4 programs are stored as raw binary data where each instruction is represented as 8-bit bytes. Multi-byte instructions (like addresses) use big-endian format.

## Program Development Tips

### Making Programs Visible
- **Use Output Ports:** Write results to output ports (0x14-0x17) for visibility
- **SRC Instruction:** Select output port with `SRC` before `WRM`
- **Real-time Output:** Numbers appear on output ports during execution
- **System Monitoring:** Watch RAM contents and output port states in real-time

### Example Output Port Usage
```asm
LDM 5        ; Load value 5
SRC 0        ; Select output port 0
WRM          ; Write to output port 0 (now visible to user)
```