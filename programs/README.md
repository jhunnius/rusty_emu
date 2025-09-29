# MCS-4 Programs

This directory contains binary program files for the Intel MCS-4 emulator.

## Available Programs

### fibonacci.bin
- **Size:** 33 bytes (16 instructions × 2 bytes each + 1 extra byte)
- **Description:** Fibonacci sequence generator
- **Algorithm:** Calculates first 8 Fibonacci numbers using registers and RAM
- **Instructions Used:** LDM, LD, WRM, IAC, ADD, JCN, NOP

## Usage

Programs are automatically loaded by the emulator. Use the `--file` option to specify a different program:

```bash
# Use default fibonacci program
cargo run -- --system basic

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