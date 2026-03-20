use anyhow::{Context, Result};
use sentinel_core::types::*;
use std::collections::HashMap;
use std::process::Command;

/// Get system status by parsing /proc and standard commands.
pub fn get_status(hostname: &str) -> Result<SystemStatus> {
    let uptime = std::fs::read_to_string("/proc/uptime")
        .context("reading /proc/uptime")?;
    let uptime_seconds = uptime
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;

    let loadavg = std::fs::read_to_string("/proc/loadavg")
        .context("reading /proc/loadavg")?;
    let loads: Vec<f64> = loadavg
        .split_whitespace()
        .take(3)
        .filter_map(|s| s.parse().ok())
        .collect();
    let load_average = [
        loads.first().copied().unwrap_or(0.0),
        loads.get(1).copied().unwrap_or(0.0),
        loads.get(2).copied().unwrap_or(0.0),
    ];

    let meminfo = std::fs::read_to_string("/proc/meminfo")
        .context("reading /proc/meminfo")?;
    let mem = parse_meminfo(&meminfo);

    let root_usage = get_root_disk_usage().unwrap_or(0.0);

    Ok(SystemStatus {
        hostname: hostname.to_string(),
        uptime_seconds,
        load_average,
        memory: MemoryInfo {
            total_bytes: mem.get("MemTotal").copied().unwrap_or(0) * 1024,
            used_bytes: (mem.get("MemTotal").copied().unwrap_or(0)
                - mem.get("MemAvailable").copied().unwrap_or(0))
                * 1024,
            available_bytes: mem.get("MemAvailable").copied().unwrap_or(0) * 1024,
        },
        swap: SwapInfo {
            total_bytes: mem.get("SwapTotal").copied().unwrap_or(0) * 1024,
            used_bytes: (mem.get("SwapTotal").copied().unwrap_or(0)
                - mem.get("SwapFree").copied().unwrap_or(0))
                * 1024,
        },
        root_disk_usage_percent: root_usage,
    })
}

fn parse_meminfo(content: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for line in content.lines() {
        if let Some((key, rest)) = line.split_once(':') {
            if let Some(val) = rest.trim().strip_suffix("kB") {
                if let Ok(v) = val.trim().parse::<u64>() {
                    map.insert(key.to_string(), v);
                }
            }
        }
    }
    map
}

fn get_root_disk_usage() -> Result<f64> {
    let stat = nix_compat_statvfs("/")?;
    let total = stat.total_blocks * stat.block_size;
    let avail = stat.avail_blocks * stat.block_size;
    if total == 0 {
        return Ok(0.0);
    }
    Ok(((total - avail) as f64 / total as f64) * 100.0)
}

struct StatVfs {
    block_size: u64,
    total_blocks: u64,
    avail_blocks: u64,
}

fn nix_compat_statvfs(path: &str) -> Result<StatVfs> {
    let output = Command::new("stat")
        .args(["-f", "-c", "%S %b %a", path])
        .output()
        .context("running stat -f")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.trim().split_whitespace().collect();
    if parts.len() < 3 {
        anyhow::bail!("unexpected stat output: {text}");
    }
    Ok(StatVfs {
        block_size: parts[0].parse()?,
        total_blocks: parts[1].parse()?,
        avail_blocks: parts[2].parse()?,
    })
}

/// Get systemd services.
pub fn get_services() -> Result<Vec<ServiceInfo>> {
    let output = Command::new("systemctl")
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--no-legend",
            "--plain",
        ])
        .output()
        .context("running systemctl list-units")?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(5, char::is_whitespace).collect();
        if parts.len() >= 4 {
            // Filter out empty strings from multiple whitespace
            let fields: Vec<&str> = parts.into_iter().filter(|s| !s.is_empty()).collect();
            if fields.len() >= 4 {
                services.push(ServiceInfo {
                    unit: fields[0].to_string(),
                    load_state: fields[1].to_string(),
                    active_state: fields[2].to_string(),
                    sub_state: fields[3].to_string(),
                    description: fields.get(4).unwrap_or(&"").to_string(),
                });
            }
        }
    }
    Ok(services)
}

/// Get failed systemd services.
pub fn get_failed() -> Result<Vec<ServiceInfo>> {
    let all = get_services()?;
    Ok(all
        .into_iter()
        .filter(|s| s.active_state == "failed")
        .collect())
}

/// Get temperature readings by parsing `sensors -j`.
pub fn get_temperatures() -> Result<Vec<TemperatureReading>> {
    let output = Command::new("sensors")
        .arg("-j")
        .output()
        .context("running sensors -j")?;

    if !output.status.success() {
        anyhow::bail!("sensors command failed");
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing sensors JSON")?;

    let mut readings = Vec::new();

    if let Some(obj) = json.as_object() {
        for (device, sensors) in obj {
            if let Some(sensors_obj) = sensors.as_object() {
                for (sensor_name, sensor_data) in sensors_obj {
                    if let Some(data_obj) = sensor_data.as_object() {
                        // Only process temperature sensors (temp*), skip voltages (in*) and fans (fan*)
                        let has_temp_field = data_obj.keys().any(|k| k.starts_with("temp"));
                        if !has_temp_field {
                            continue;
                        }

                        let mut current = None;
                        let mut high = None;
                        let mut critical = None;

                        for (key, val) in data_obj {
                            if key.starts_with("temp") && key.ends_with("_input") {
                                current = val.as_f64();
                            } else if key.starts_with("temp") && key.ends_with("_max") {
                                high = val.as_f64();
                            } else if key.starts_with("temp") && key.ends_with("_crit") {
                                critical = val.as_f64();
                            }
                        }

                        if let Some(temp) = current {
                            readings.push(TemperatureReading {
                                device: device.clone(),
                                sensor: sensor_name.clone(),
                                current_c: temp,
                                high_c: high,
                                critical_c: critical,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(readings)
}

/// Get disk usage.
pub fn get_disk_usage() -> Result<Vec<DiskUsage>> {
    let output = Command::new("df")
        .args(["--output=source,target,size,used,avail,pcent", "-B1", "--exclude-type=tmpfs", "--exclude-type=devtmpfs", "--exclude-type=efivarfs"])
        .output()
        .context("running df")?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::new();

    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 6 {
            disks.push(DiskUsage {
                filesystem: fields[0].to_string(),
                mount_point: fields[1].to_string(),
                total_bytes: fields[2].parse().unwrap_or(0),
                used_bytes: fields[3].parse().unwrap_or(0),
                available_bytes: fields[4].parse().unwrap_or(0),
                use_percent: fields[5]
                    .trim_end_matches('%')
                    .parse()
                    .unwrap_or(0.0),
            });
        }
    }
    Ok(disks)
}

/// Get GPU status from sysfs (AMD).
pub fn get_gpu_status() -> Result<GpuStatus> {
    // Find the first amdgpu card
    let drm = std::path::Path::new("/sys/class/drm");
    let mut card_path = None;

    if drm.exists() {
        for entry in std::fs::read_dir(drm)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && !name.contains('-') {
                let device_path = entry.path().join("device");
                if device_path.join("gpu_busy_percent").exists() {
                    card_path = Some((name, device_path));
                    break;
                }
            }
        }
    }

    let (card, device) = card_path
        .ok_or_else(|| anyhow::anyhow!("no AMD GPU found"))?;

    let vram_total = read_sysfs_u64(&device.join("mem_info_vram_total")).ok();
    let vram_used = read_sysfs_u64(&device.join("mem_info_vram_used")).ok();
    let temp = read_hwmon_temp(&device);
    let power = read_hwmon_power(&device);

    Ok(GpuStatus {
        driver: "amdgpu".to_string(),
        card,
        vram_total_mb: vram_total.map(|v| v / 1024 / 1024),
        vram_used_mb: vram_used.map(|v| v / 1024 / 1024),
        temperature_c: temp,
        power_watts: power,
    })
}

fn read_sysfs_u64(path: &std::path::Path) -> Result<u64> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.trim().parse()?)
}

fn read_hwmon_temp(device: &std::path::Path) -> Option<f64> {
    let hwmon_dir = device.join("hwmon");
    if let Ok(entries) = std::fs::read_dir(&hwmon_dir) {
        for entry in entries.flatten() {
            let temp_file = entry.path().join("temp1_input");
            if let Ok(content) = std::fs::read_to_string(&temp_file) {
                if let Ok(millideg) = content.trim().parse::<f64>() {
                    return Some(millideg / 1000.0);
                }
            }
        }
    }
    None
}

fn read_hwmon_power(device: &std::path::Path) -> Option<f64> {
    let hwmon_dir = device.join("hwmon");
    if let Ok(entries) = std::fs::read_dir(&hwmon_dir) {
        for entry in entries.flatten() {
            let power_file = entry.path().join("power1_average");
            if let Ok(content) = std::fs::read_to_string(&power_file) {
                if let Ok(microwatts) = content.trim().parse::<f64>() {
                    return Some(microwatts / 1_000_000.0);
                }
            }
        }
    }
    None
}

/// Get journal log lines for a unit.
pub fn get_logs(unit: &str, lines: u32) -> Result<LogLines> {
    let output = Command::new("journalctl")
        .args([
            "-u",
            unit,
            "-n",
            &lines.to_string(),
            "--no-pager",
            "--output=short-iso",
        ])
        .output()
        .context("running journalctl")?;

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(LogLines {
        unit: unit.to_string(),
        lines: text.lines().map(|l| l.to_string()).collect(),
    })
}

/// Perform a health check.
pub fn get_health(hostname: &str) -> Result<HealthCheck> {
    let mut checks = Vec::new();
    let mut worst = HealthStatus::Pass;

    // Check disk space
    if let Ok(disks) = get_disk_usage() {
        for d in &disks {
            if d.use_percent > 95.0 {
                checks.push(HealthCheckItem {
                    name: format!("disk:{}", d.mount_point),
                    status: HealthStatus::Fail,
                    message: format!("{}% full", d.use_percent),
                });
                worst = HealthStatus::Fail;
            } else if d.use_percent > 90.0 {
                checks.push(HealthCheckItem {
                    name: format!("disk:{}", d.mount_point),
                    status: HealthStatus::Warn,
                    message: format!("{}% full", d.use_percent),
                });
                if worst == HealthStatus::Pass {
                    worst = HealthStatus::Warn;
                }
            }
        }
    }

    // Check failed services
    if let Ok(failed) = get_failed() {
        if !failed.is_empty() {
            let names: Vec<_> = failed.iter().map(|s| s.unit.as_str()).collect();
            checks.push(HealthCheckItem {
                name: "failed-services".to_string(),
                status: HealthStatus::Warn,
                message: format!("{} failed: {}", failed.len(), names.join(", ")),
            });
            if worst == HealthStatus::Pass {
                worst = HealthStatus::Warn;
            }
        }
    }

    // Check temperatures
    if let Ok(temps) = get_temperatures() {
        for t in &temps {
            if let Some(crit) = t.critical_c {
                if t.current_c >= crit {
                    checks.push(HealthCheckItem {
                        name: format!("temp:{}:{}", t.device, t.sensor),
                        status: HealthStatus::Fail,
                        message: format!("{}°C (critical: {}°C)", t.current_c, crit),
                    });
                    worst = HealthStatus::Fail;
                }
            }
            if let Some(high) = t.high_c {
                if t.current_c >= high {
                    checks.push(HealthCheckItem {
                        name: format!("temp:{}:{}", t.device, t.sensor),
                        status: HealthStatus::Warn,
                        message: format!("{}°C (high: {}°C)", t.current_c, high),
                    });
                    if worst == HealthStatus::Pass {
                        worst = HealthStatus::Warn;
                    }
                }
            }
        }
    }

    // Check memory pressure
    if let Ok(status) = get_status(hostname) {
        let mem_pct = if status.memory.total_bytes > 0 {
            (status.memory.used_bytes as f64 / status.memory.total_bytes as f64) * 100.0
        } else {
            0.0
        };
        if mem_pct > 95.0 {
            checks.push(HealthCheckItem {
                name: "memory".to_string(),
                status: HealthStatus::Fail,
                message: format!("{:.0}% used", mem_pct),
            });
            worst = HealthStatus::Fail;
        } else if mem_pct > 90.0 {
            checks.push(HealthCheckItem {
                name: "memory".to_string(),
                status: HealthStatus::Warn,
                message: format!("{:.0}% used", mem_pct),
            });
            if worst == HealthStatus::Pass {
                worst = HealthStatus::Warn;
            }
        }
    }

    if checks.is_empty() {
        checks.push(HealthCheckItem {
            name: "overall".to_string(),
            status: HealthStatus::Pass,
            message: "all checks passed".to_string(),
        });
    }

    Ok(HealthCheck {
        status: worst,
        checks,
    })
}

/// Restart a systemd service.
pub fn restart_service(unit: &str) -> Result<ActionResult> {
    let output = Command::new("systemctl")
        .args(["restart", unit])
        .output()
        .context("running systemctl restart")?;

    Ok(ActionResult {
        action: format!("restart-service {unit}"),
        success: output.status.success(),
        message: if output.status.success() {
            None
        } else {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        },
    })
}

/// Trigger GPU reset via sysfs.
pub fn gpu_reset() -> Result<ActionResult> {
    // Try the amdgpu debugfs reset path
    let reset_path = "/sys/kernel/debug/dri/0/amdgpu_gpu_recover";
    match std::fs::write(reset_path, "1") {
        Ok(_) => Ok(ActionResult {
            action: "gpu-reset".to_string(),
            success: true,
            message: Some("GPU recovery triggered".to_string()),
        }),
        Err(e) => Ok(ActionResult {
            action: "gpu-reset".to_string(),
            success: false,
            message: Some(format!("failed to trigger GPU recovery: {e}")),
        }),
    }
}

/// Vacuum journal.
pub fn journal_vacuum(max_size: &str) -> Result<ActionResult> {
    let output = Command::new("journalctl")
        .args(["--vacuum-size", max_size])
        .output()
        .context("running journalctl --vacuum-size")?;

    Ok(ActionResult {
        action: format!("journal-vacuum {max_size}"),
        success: output.status.success(),
        message: Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
    })
}

/// Reboot the system.
pub fn reboot() -> Result<ActionResult> {
    let output = Command::new("systemctl")
        .arg("reboot")
        .output()
        .context("running systemctl reboot")?;

    Ok(ActionResult {
        action: "reboot".to_string(),
        success: output.status.success(),
        message: if output.status.success() {
            Some("reboot initiated".to_string())
        } else {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        },
    })
}

/// Kill a process by PID.
pub fn kill_process(pid: u32) -> Result<ActionResult> {
    let output = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output()
        .context("running kill")?;

    Ok(ActionResult {
        action: format!("kill-process {pid}"),
        success: output.status.success(),
        message: if output.status.success() {
            Some(format!("SIGTERM sent to PID {pid}"))
        } else {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        },
    })
}
