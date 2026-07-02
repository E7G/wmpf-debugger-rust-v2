# WMPF Debugger (Rust)

A WeChat Mini Program (WMPF) remote debugger written in pure Rust.

This tool intercepts the communication between WeChatAppEx.exe and the Chrome DevTools Protocol (CDP), allowing you to debug WeChat Mini Programs using standard browser developer tools.

## Architecture

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   WeChatAppEx    │────▶│  WMPF Debugger   │────▶│  Chrome DevTools │
│   (Mini Program) │     │  (Rust)          │     │  Inspector       │
│                  │◀────│                  │◀────│                  │
└──────────────────┘     └──────────────────┘     └──────────────────┘
      ws://localhost:9421        ↕           ws://localhost:62666
                              Frida
                           (script inject)
```

1. **Frida** injects a hook script into WeChatAppEx.exe to enable the remote debug protocol
2. **Debug Server** (port 9421) receives protobuf-encoded debug messages from the mini program
3. **CDP Proxy** (port 62666) translates between the internal protocol and Chrome DevTools Protocol

## Installation

### Download Pre-built Binaries (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/E7G/wmpf-debugger-rust/releases):

| Platform | File |
|----------|------|
| Windows x64 | `wmpf-debugger-x86_64-pc-windows-msvc.exe` |
| Linux x64 | `wmpf-debugger-x86_64-unknown-linux-gnu` |
| macOS x64 | `wmpf-debugger-x86_64-apple-darwin` |

```bash
# Windows (PowerShell)
Invoke-WebRequest -Uri "https://github.com/E7G/wmpf-debugger-rust/releases/latest/download/wmpf-debugger-x86_64-pc-windows-msvc.exe" -OutFile "wmpf-debugger.exe"

# Linux
curl -LO https://github.com/E7G/wmpf-debugger-rust/releases/latest/download/wmpf-debugger-x86_64-unknown-linux-gnu
chmod +x wmpf-debugger-x86_64-unknown-linux-gnu

# macOS
curl -LO https://github.com/E7G/wmpf-debugger-rust/releases/latest/download/wmpf-debugger-x86_64-apple-darwin
chmod +x wmpf-debugger-x86_64-apple-darwin
```

### Build from Source

**Prerequisites:**
- [Rust](https://rustup.rs/) (1.70+)
- [protobuf compiler](https://github.com/protocolbuffers/protobuf/releases) (`protoc`)

```bash
# Clone the repository
git clone https://github.com/E7G/wmpf-debugger-rust.git
cd wmpf-debugger-rust

# Build without Frida (WebSocket servers only)
cargo build --release

# Build with Frida integration (requires frida-core devkit)
# 1. Download frida-core-devkit from https://github.com/frida/frida/releases
# 2. Extract to frida-devkit/ in the project root
# 3. Build with feature flag
cargo build --release --features frida-link
```

The binary will be at `target/release/wmpf-debugger.exe` (Windows) or `target/release/wmpf-debugger` (Linux/macOS).

## Usage

### 1. Start the Debugger

```bash
# Windows
wmpf-debugger.exe

# Linux/macOS
./wmpf-debugger-x86_64-unknown-linux-gnu
```

Make sure WeChat DevTools is running (WeChatAppEx.exe process must be active).

### 2. Open Chrome DevTools

Navigate to:
```
devtools://devtools/bundled/inspector.html?ws=127.0.0.1:62666
```

### 3. Debug Your Mini Program

Open any mini program in WeChat DevTools - the debugger will automatically attach and you can use Chrome DevTools to inspect it.

### Command-line Options

| Flag | Description | Default |
|------|-------------|---------|
| `--debug-port <port>` | Debug server WebSocket port | 9421 |
| `--cdp-port <port>` | CDP proxy WebSocket port | 62666 |
| `--debug-main` | Output main process debug messages | false |
| `--debug-frida` | Output Frida client messages | false |
| `-h, --help` | Show help message | |

```bash
# Example: custom ports with debug output
wmpf-debugger.exe --debug-port 9421 --cdp-port 62666 --debug-main --debug-frida
```

## How It Works

1. Attaches to WeChatAppEx.exe via Frida
2. Extracts WMPF version from the process path (e.g., `...\14185\...`)
3. Loads version-specific hook script from `frida/hook.js`
4. Patches the CDP filter to allow remote debugging
5. Modifies the mini program scene to enable debug mode
6. Proxies debug messages between the mini program and Chrome DevTools

## Project Structure

```
src/
├── main.rs           # Entry point, spawns all servers
├── cli.rs            # Command-line argument parsing
├── logger.rs         # Colored logging
├── constants.rs      # Protocol constants and enums
├── proto.rs          # Protobuf generated types (prost)
├── codex.rs          # Protobuf encode/decode + Zlib compression
├── frida_ffi.rs      # Raw FFI bindings to Frida C API (behind frida-link feature)
├── frida_server.rs   # Frida integration (process attach, script inject)
├── debug_server.rs   # WebSocket server for mini program connections
└── proxy_server.rs   # WebSocket server for CDP connections

proto/
└── wa_remote_debug.proto  # WeChat remote debug protocol definition

frida/
├── hook.js           # Frida hook script
└── config/           # Version-specific memory addresses (45 versions)
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "WeChatAppEx.exe process not found" | Make sure WeChat DevTools is running |
| "error finding WMPF version" | Your WMPF version may not be supported (check `frida/config/`) |
| "hook script not found" | Ensure `frida/hook.js` is in the working directory |
| Port already in use | Change ports with `--debug-port` and `--cdp-port` |
| Frida not available | Build with `--features frida-link` and provide frida-core devkit |

## Acknowledgments

- Original TypeScript implementation: [WMPFDebugger](https://github.com/nicennnnnnnlee/WMPFDebugger) by evi0s
- [Frida](https://frida.re/) - Dynamic instrumentation toolkit

## License

GPL-2.0-only
