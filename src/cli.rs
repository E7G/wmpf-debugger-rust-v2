use std::fmt;

pub const DEBUG_PORT: u16 = 9421;
pub const CDP_PORT: u16 = 62666;

#[derive(Debug)]
pub enum CliError {
    InvalidPort { name: String, value: String },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPort { name, value } => write!(f, "invalid {}: {}", name, value),
        }
    }
}

impl std::error::Error for CliError {}

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub debug_port: u16,
    pub cdp_port: u16,
    pub debug_main: bool,
    pub debug_frida: bool,
}

pub fn print_help() {
    println!(
        "Usage: wmpf_debugger [options]

Options:
  --debug-port <port>  Remote debug server port (default: {})
  --cdp-port <port>    CDP proxy server port (default: {})
  --debug-main         Output main process debug messages
  --debug-frida        Output Frida client messages
  -h, --help           Show this help message",
        DEBUG_PORT, CDP_PORT
    );
}

fn parse_port(name: &str, value: Option<&str>, default: u16) -> Result<u16, CliError> {
    match value {
        None => Ok(default),
        Some(v) => {
            let port: u16 = v.parse().map_err(|_| CliError::InvalidPort {
                name: name.to_string(),
                value: v.to_string(),
            })?;
            if port == 0 {
                return Err(CliError::InvalidPort {
                    name: name.to_string(),
                    value: v.to_string(),
                });
            }
            Ok(port)
        }
    }
}

pub fn parse_cli_options() -> Result<CliOptions, CliError> {
    let args: Vec<String> = std::env::args().collect();
    let mut debug_port = DEBUG_PORT;
    let mut cdp_port = CDP_PORT;
    let mut debug_main = false;
    let mut debug_frida = false;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--debug-main" => {
                debug_main = true;
            }
            "--debug-frida" => {
                debug_frida = true;
            }
            "--debug-port" => {
                i += 1;
                debug_port =
                    parse_port("--debug-port", args.get(i).map(|s| s.as_str()), DEBUG_PORT)?;
            }
            "--cdp-port" => {
                i += 1;
                cdp_port = parse_port("--cdp-port", args.get(i).map(|s| s.as_str()), CDP_PORT)?;
            }
            other => {
                eprintln!("Unknown option: {}", other);
                print_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    Ok(CliOptions {
        debug_port,
        cdp_port,
        debug_main,
        debug_frida,
    })
}
