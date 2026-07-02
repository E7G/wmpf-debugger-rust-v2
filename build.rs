use prost_build::compile_protos;
use std::env;
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    compile_protos(&["proto/wa_remote_debug.proto"], &["proto/"])?;

    // Only link frida when the frida-link feature is enabled
    if env::var("CARGO_FEATURE_FRIDA_LINK").is_ok() {
        let devkit_path = env::var("FRIDA_DEVKIT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("frida-devkit"));

        if devkit_path.exists() {
            println!("cargo:rustc-link-search=native={}", devkit_path.display());
            println!("cargo:rustc-link-lib=static=frida-core");

            if cfg!(target_os = "windows") {
                for lib in &[
                    "dnsapi", "iphlpapi", "psapi", "winmm", "ws2_32", "advapi32", "crypt32",
                    "gdi32", "kernel32", "ole32", "secur32", "shell32", "shlwapi", "user32",
                    "setupapi",
                ] {
                    println!("cargo:rustc-link-lib=dylib={}", lib);
                }
            }
        } else {
            println!(
                "cargo:warning=frida-devkit not found at {}. Set FRIDA_DEVKIT_PATH.",
                devkit_path.display()
            );
        }
    }

    Ok(())
}
