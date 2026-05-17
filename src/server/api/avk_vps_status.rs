//! AVK VPS filosu sistem durum endpoint — `GET /api/avk/vps-status`.
//!
//! Daemon'ın çalıştığı host'un + `AOE_FLEET` ile tanımlı uzak host'ların
//! kompakt sistem metriklerini liste olarak döner. Uzak host'lara SSH
//! ile bağlanıp `/proc/*` + `df` çıktısı okunur — uzak host'a kurulum
//! gerekmez, sadece SSH key erişimi yeter.
//!
//! `AOE_FLEET` formatı (semicolon separated):
//!   `name@user@host[;name2@user@host]`
//! Örnek:
//!   `runner@root@avk-runners;edge@root@1.2.3.4`
//!
//! Local host her zaman ilk eleman (`role=primary`). SSH komut timeout
//! 5s, hata olursa `ok=false` + `error` doldurulur.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;

use super::AppState;

const REMOTE_SSH_TIMEOUT_SEC: u64 = 5;

#[derive(Serialize, Deserialize)]
pub struct AvkVpsFleetResponse {
    pub hosts: Vec<AvkVpsHostEntry>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct AvkVpsHostEntry {
    pub name: String,
    pub role: String,
    pub ok: bool,
    pub error: Option<String>,
    pub hostname: Option<String>,
    pub kernel: Option<String>,
    pub os: Option<String>,
    pub uptime_sec: Option<u64>,
    pub cpu_count: Option<usize>,
    pub load_avg: Option<[f32; 3]>,
    pub memory: Option<MemoryStat>,
    pub disk: Option<DiskStat>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MemoryStat {
    pub total_kb: u64,
    pub used_kb: u64,
    pub used_pct: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DiskStat {
    pub mount: String,
    pub total_kb: u64,
    pub used_kb: u64,
    pub used_pct: u8,
}

pub async fn get_avk_vps_status(State(_state): State<Arc<AppState>>) -> Response {
    let mut hosts = vec![collect_local()];
    for (name, target) in parse_fleet_env() {
        hosts.push(fetch_remote_ssh(name, target).await);
    }
    Json(AvkVpsFleetResponse { hosts }).into_response()
}

fn collect_local() -> AvkVpsHostEntry {
    let hostname = read_hostname_local();
    let (os, kernel) = read_os_kernel_local();
    AvkVpsHostEntry {
        name: hostname.clone(),
        role: "primary".to_string(),
        ok: true,
        error: None,
        hostname: Some(hostname),
        kernel,
        os,
        uptime_sec: read_uptime_local(),
        cpu_count: std::thread::available_parallelism().ok().map(|n| n.get()),
        load_avg: read_loadavg_local(),
        memory: read_memory_local(),
        disk: read_disk_root_local(),
    }
}

fn parse_fleet_env() -> Vec<(String, String)> {
    let Ok(raw) = std::env::var("AOE_FLEET") else {
        return Vec::new();
    };
    raw.split(';')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let (name, target) = entry.split_once('@')?;
            let name = name.trim();
            let target = target.trim();
            if name.is_empty() || target.is_empty() {
                return None;
            }
            Some((name.to_string(), target.to_string()))
        })
        .collect()
}

const REMOTE_PROBE_SCRIPT: &str = r####"
echo "###HOSTNAME###"
hostname 2>/dev/null
echo "###UNAME###"
uname -r 2>/dev/null
echo "###OS###"
grep ^PRETTY_NAME /etc/os-release 2>/dev/null
echo "###UPTIME###"
cat /proc/uptime 2>/dev/null
echo "###CPU###"
nproc 2>/dev/null
echo "###LOADAVG###"
cat /proc/loadavg 2>/dev/null
echo "###MEMINFO###"
grep -E '^Mem(Total|Available):' /proc/meminfo 2>/dev/null
echo "###DISK###"
df -Pk / 2>/dev/null | tail -1
"####;

async fn fetch_remote_ssh(name: String, target: String) -> AvkVpsHostEntry {
    let target_owned = target.clone();
    let result = tokio::task::spawn_blocking(move || run_ssh_probe(&target_owned)).await;

    match result {
        Ok(Ok(raw)) => parse_remote(name, raw),
        Ok(Err(err)) => AvkVpsHostEntry {
            name,
            role: "runner".to_string(),
            ok: false,
            error: Some(err),
            ..Default::default()
        },
        Err(err) => AvkVpsHostEntry {
            name,
            role: "runner".to_string(),
            ok: false,
            error: Some(format!("join: {err}")),
            ..Default::default()
        },
    }
}

fn run_ssh_probe(target: &str) -> Result<String, String> {
    let out = Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            &format!("ConnectTimeout={REMOTE_SSH_TIMEOUT_SEC}"),
            "-o",
            "StrictHostKeyChecking=accept-new",
            target,
            REMOTE_PROBE_SCRIPT,
        ])
        .output()
        .map_err(|e| format!("ssh spawn: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("ssh exit {}", out.status)
        } else {
            stderr
        });
    }
    String::from_utf8(out.stdout).map_err(|e| format!("utf8: {e}"))
}

fn parse_remote(name: String, raw: String) -> AvkVpsHostEntry {
    let mut sections: std::collections::HashMap<&str, Vec<&str>> = Default::default();
    let mut current: Option<&str> = None;
    for line in raw.lines() {
        if let Some(tag) = line.strip_prefix("###").and_then(|s| s.strip_suffix("###")) {
            current = Some(tag);
            sections.entry(tag).or_default();
        } else if let Some(tag) = current {
            sections.entry(tag).or_default().push(line);
        }
    }
    let take_first = |tag: &str| {
        sections
            .get(tag)
            .and_then(|v| v.iter().find(|l| !l.trim().is_empty()))
            .map(|l| l.trim().to_string())
    };

    let hostname = take_first("HOSTNAME");
    let kernel = take_first("UNAME");
    let os = take_first("OS").map(|s| {
        s.strip_prefix("PRETTY_NAME=")
            .unwrap_or(&s)
            .trim_matches('"')
            .to_string()
    });
    let uptime_sec = take_first("UPTIME").and_then(|s| {
        s.split_whitespace()
            .next()
            .and_then(|n| n.parse::<f64>().ok())
            .map(|f| f as u64)
    });
    let cpu_count = take_first("CPU").and_then(|s| s.parse::<usize>().ok());
    let load_avg = take_first("LOADAVG").and_then(|s| {
        let mut parts = s.split_whitespace();
        let a: f32 = parts.next()?.parse().ok()?;
        let b: f32 = parts.next()?.parse().ok()?;
        let c: f32 = parts.next()?.parse().ok()?;
        Some([a, b, c])
    });
    let memory = parse_remote_memory(sections.get("MEMINFO").map(|v| v.as_slice()));
    let disk = parse_remote_disk(take_first("DISK"));

    let resolved_hostname = hostname.clone().unwrap_or_else(|| name.clone());
    AvkVpsHostEntry {
        name,
        role: "runner".to_string(),
        ok: true,
        error: None,
        hostname: Some(resolved_hostname),
        kernel,
        os,
        uptime_sec,
        cpu_count,
        load_avg,
        memory,
        disk,
    }
}

fn parse_remote_memory(lines: Option<&[&str]>) -> Option<MemoryStat> {
    let lines = lines?;
    let mut total_kb: Option<u64> = None;
    let mut avail_kb: Option<u64> = None;
    for line in lines {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(v);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail_kb = parse_kb(v);
        }
    }
    let total = total_kb?;
    let avail = avail_kb?;
    let used = total.saturating_sub(avail);
    let pct = if total > 0 {
        ((used as f64 / total as f64) * 100.0).round() as u8
    } else {
        0
    };
    Some(MemoryStat {
        total_kb: total,
        used_kb: used,
        used_pct: pct,
    })
}

fn parse_remote_disk(row: Option<String>) -> Option<DiskStat> {
    let row = row?;
    let cols: Vec<&str> = row.split_whitespace().collect();
    if cols.len() < 6 {
        return None;
    }
    let total_kb: u64 = cols[1].parse().ok()?;
    let used_kb: u64 = cols[2].parse().ok()?;
    let pct = if total_kb > 0 {
        ((used_kb as f64 / total_kb as f64) * 100.0).round() as u8
    } else {
        0
    };
    Some(DiskStat {
        mount: cols[5].to_string(),
        total_kb,
        used_kb,
        used_pct: pct,
    })
}

// ---------- local readers ----------

fn read_hostname_local() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "bilinmiyor".to_string())
}

fn read_os_kernel_local() -> (Option<String>, Option<String>) {
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|raw| {
            raw.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim_matches('"').to_string())
        });
    let kernel = Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    (os, kernel)
}

fn read_uptime_local() -> Option<u64> {
    let raw = std::fs::read_to_string("/proc/uptime").ok()?;
    let secs: f64 = raw.split_whitespace().next()?.parse().ok()?;
    Some(secs as u64)
}

fn read_loadavg_local() -> Option<[f32; 3]> {
    let raw = std::fs::read_to_string("/proc/loadavg").ok()?;
    let mut parts = raw.split_whitespace();
    let a: f32 = parts.next()?.parse().ok()?;
    let b: f32 = parts.next()?.parse().ok()?;
    let c: f32 = parts.next()?.parse().ok()?;
    Some([a, b, c])
}

fn read_memory_local() -> Option<MemoryStat> {
    let raw = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb: Option<u64> = None;
    let mut avail_kb: Option<u64> = None;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(v);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail_kb = parse_kb(v);
        }
        if total_kb.is_some() && avail_kb.is_some() {
            break;
        }
    }
    let total = total_kb?;
    let avail = avail_kb?;
    let used = total.saturating_sub(avail);
    let pct = if total > 0 {
        ((used as f64 / total as f64) * 100.0).round() as u8
    } else {
        0
    };
    Some(MemoryStat {
        total_kb: total,
        used_kb: used,
        used_pct: pct,
    })
}

fn parse_kb(v: &str) -> Option<u64> {
    v.trim()
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
}

fn read_disk_root_local() -> Option<DiskStat> {
    let out = Command::new("df").args(["-Pk", "/"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    let row = stdout.lines().nth(1)?;
    let cols: Vec<&str> = row.split_whitespace().collect();
    if cols.len() < 6 {
        return None;
    }
    let total_kb: u64 = cols[1].parse().ok()?;
    let used_kb: u64 = cols[2].parse().ok()?;
    let pct = if total_kb > 0 {
        ((used_kb as f64 / total_kb as f64) * 100.0).round() as u8
    } else {
        0
    };
    Some(DiskStat {
        mount: cols[5].to_string(),
        total_kb,
        used_kb,
        used_pct: pct,
    })
}
