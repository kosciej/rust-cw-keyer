# Rust CW Keyer

A simple, low-latency CW (Continuous Wave) keyer written in Rust. It captures keyboard input and toggles modem control lines (RTS/DTR) on a serial port to key a radio.

## Features

- **Low Latency:** High polling rate (~500Hz) ensures minimal delay between keypress and signal.
- **Cross-Platform:** 
  - **Unix (Linux/macOS):** Creates a virtual serial port (PTY) that you can connect your radio software (like fldigi or Thetis) to.
  - **Windows:** Connects to a physical COM port (or a virtual one like com0com).
- **Simple Controls:**
  - `Z` key: DIT (keys RTS)
  - `X` key: DAH (keys DTR)
  - `Esc`: Exit the application

## Prerequisites

- **Rust:** You'll need the Rust toolchain installed (`cargo`, `rustc`).
- **Permissions:** 
  - On Linux/macOS, ensure your user has permissions to access serial devices (usually by being in the `dialout` or `uucp` group).

## Installation & Building

```bash
git clone git@github.com:kosciej/rust-cw-keyer.git
cd rust-cw-keyer
cargo build
```

## Usage

### 1. Run the Keyer

Start the keyer using:

```bash
cargo run
```

On Unix, it will output the path to the virtual serial port (e.g., `/dev/ttys005`). You should configure your radio application to use this port for CW keying.

### 2. Verify the Signal

To verify that the signals are being sent correctly, you can use the built-in verification tool:

```bash
cargo run --bin verify-keyer <serial-port-path>
```

Replace `<serial-port-path>` with the path provided by the main application.

## Project Structure

- `src/main.rs`: The main application logic and port abstraction.
- `src/bin/verify.rs`: A utility to monitor modem control lines on a serial port.
- `Cargo.toml`: Project configuration and dependencies.

## License

This project is licensed under the MIT License.
