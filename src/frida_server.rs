use std::sync::Arc;

use crate::cli::CliOptions;
use crate::logger::Logger;

#[cfg(feature = "frida-link")]
mod inner {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    pub async fn run(_options: CliOptions, logger: Arc<Logger>) {
        logger.info("[frida] attempting to attach to WeChatAppEx.exe...");

        let dm = match crate::frida_ffi::DeviceManager::new() {
            Ok(dm) => dm,
            Err(e) => {
                logger.error(&format!("[frida] failed to create device manager: {}", e));
                return;
            }
        };

        let device = match dm.get_local_device() {
            Ok(d) => d,
            Err(e) => {
                logger.error(&format!("[frida] failed to get local device: {}", e));
                return;
            }
        };

        let processes = match device.enumerate_processes() {
            Ok(p) => p,
            Err(e) => {
                logger.error(&format!("[frida] failed to enumerate processes: {}", e));
                return;
            }
        };

        let wmpf: Vec<_> = processes
            .iter()
            .filter(|p| p.name == "WeChatAppEx.exe")
            .collect();

        if wmpf.is_empty() {
            logger.error("[frida] WeChatAppEx.exe process not found");
            return;
        }

        logger.info(&format!(
            "[frida] found {} WeChatAppEx.exe processes",
            wmpf.len()
        ));

        let parent_pid = find_most_common_pid(&wmpf);
        let pid = match parent_pid {
            Some(p) => p,
            None => {
                logger.error("[frida] could not determine parent pid");
                return;
            }
        };

        let version = extract_version(&wmpf, pid);
        let version = match version {
            Some(v) => v,
            None => {
                logger.error("[frida] error finding WMPF version");
                return;
            }
        };

        let session = match device.attach(pid) {
            Ok(s) => s,
            Err(e) => {
                logger.error(&format!("[frida] failed to attach to pid {}: {}", pid, e));
                return;
            }
        };

        let script_content = match load_hook_script() {
            Ok(s) => s,
            Err(e) => {
                logger.error(&format!("[frida] {}", e));
                return;
            }
        };

        let config_content = match load_version_config(version) {
            Ok(s) => s,
            Err(e) => {
                logger.error(&format!("[frida] {}", e));
                return;
            }
        };

        let script_source = script_content.replace("@@CONFIG@@", &config_content);

        let script = match session.create_script(&script_source) {
            Ok(s) => s,
            Err(e) => {
                logger.error(&format!("[frida] failed to create script: {}", e));
                return;
            }
        };

        if let Err(e) = script.load() {
            logger.error(&format!("[frida] failed to load script: {}", e));
            return;
        }

        logger.info(&format!(
            "[frida] script loaded, WMPF version: {}, pid: {}",
            version, pid
        ));
        logger.info("[frida] you can now open any miniapps");
    }

    fn find_most_common_pid(processes: &[&crate::frida_ffi::Process]) -> Option<u32> {
        use std::collections::HashMap;
        let mut counts: HashMap<u32, usize> = HashMap::new();
        for p in processes {
            let ppid = if p.ppid > 0 { p.ppid } else { p.pid };
            *counts.entry(ppid).or_insert(0) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|&(_, c)| c)
            .map(|(pid, _)| pid)
    }

    fn extract_version(processes: &[&crate::frida_ffi::Process], parent_pid: u32) -> Option<u32> {
        let process = processes.iter().find(|p| {
            let ppid = if p.ppid > 0 { p.ppid } else { p.pid };
            ppid == parent_pid
        })?;

        if !process.path.is_empty() {
            let numbers: Vec<&str> = process
                .path
                .split(|c: char| !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
                .collect();
            for num_str in numbers.iter().rev() {
                if let Ok(v) = num_str.parse::<u32>() {
                    if v > 1000 {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    fn load_hook_script() -> Result<String, String> {
        let candidates: Vec<Option<PathBuf>> = vec![
            Some(PathBuf::from("frida/hook.js")),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("frida/hook.js"))),
            std::env::current_dir()
                .ok()
                .map(|d| d.join("frida/hook.js")),
        ];

        for path in candidates.into_iter().flatten() {
            if path.exists() {
                return fs::read_to_string(&path)
                    .map_err(|e| format!("failed to read {}: {}", path.display(), e));
            }
        }
        Err("hook script not found (frida/hook.js)".to_string())
    }

    fn load_version_config(version: u32) -> Result<String, String> {
        let config_name = format!("frida/config/addresses.{}.json", version);
        let candidates: Vec<Option<PathBuf>> = vec![
            Some(PathBuf::from(&config_name)),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join(&config_name))),
            std::env::current_dir().ok().map(|d| d.join(&config_name)),
        ];

        for path in candidates.into_iter().flatten() {
            if path.exists() {
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
                let _: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| format!("invalid JSON in {}: {}", path.display(), e))?;
                return Ok(content);
            }
        }
        Err(format!(
            "version config not found: addresses.{}.json",
            version
        ))
    }
}

#[cfg(not(feature = "frida-link"))]
mod inner {
    use super::*;
    pub async fn run(_options: CliOptions, logger: Arc<Logger>) {
        logger.error("[frida] frida integration not available (build with --features frida-link)");
        logger.info(
            "[frida] download frida-core devkit and build with: cargo build --features frida-link",
        );
    }
}

pub async fn run_frida_server(options: CliOptions, logger: Arc<Logger>) {
    inner::run(options, logger).await;
}
