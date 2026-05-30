// stats.admin.rs - Admin panel for system statistics
// Handles system stats interface including memory info, load average, and hourly stats

use std::fs;
use std::collections::HashMap;

// System memory information structure
#[derive(Debug)]
struct MemoryInfo {
    total: u64,
    free: u64,
    available: u64,
    buffers: u64,
    cached: u64,
    swap_total: u64,
    swap_free: u64,
}

// Load average information structure
#[derive(Debug)]
struct LoadAverage {
    one_minute: f64,
    five_minutes: f64,
    fifteen_minutes: f64,
}

// System uptime information
#[derive(Debug)]
struct UptimeInfo {
    uptime_seconds: f64,
    idle_seconds: f64,
}

// CPU information
#[derive(Debug)]
struct CpuInfo {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

// Disk usage information structure
#[derive(Debug)]
struct DiskUsage {
    filesystem: String,
    total: u64,
    used: u64,
    available: u64,
    usage_percent: f64,
    mount_point: String,
}

// Hourly statistics structure (matches the one in hourly_stats.rs)
#[derive(Debug, Clone)]
struct HourlyStats {
    timestamp: u64,        // Unix timestamp of the hour
    memory_used_mb: f64,   // Memory usage in MB
    cpu_usage_percent: f64, // CPU usage percentage
    request_count: u64,    // Number of requests in this hour
}

// Parse memory information (platform-specific)
fn parse_meminfo() -> Result<MemoryInfo, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::stats_admin::windows_stats::parse_meminfo_windows;
        return parse_meminfo_windows();
    }

    #[cfg(not(target_os = "windows"))]
    {
    let meminfo_content = fs::read_to_string("/proc/meminfo")
        .map_err(|e| format!("Failed to read /proc/meminfo: {}", e))?;

    let mut meminfo = MemoryInfo {
        total: 0,
        free: 0,
        available: 0,
        buffers: 0,
        cached: 0,
        swap_total: 0,
        swap_free: 0,
    };

    for line in meminfo_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(value) = parts[1].parse::<u64>() {
                match parts[0] {
                    "MemTotal:" => meminfo.total = value * 1024, // Convert from KB to bytes
                    "MemFree:" => meminfo.free = value * 1024,
                    "MemAvailable:" => meminfo.available = value * 1024,
                    "Buffers:" => meminfo.buffers = value * 1024,
                    "Cached:" => meminfo.cached = value * 1024,
                    "SwapTotal:" => meminfo.swap_total = value * 1024,
                    "SwapFree:" => meminfo.swap_free = value * 1024,
                    _ => {}
                }
            }
        }
    }

    Ok(meminfo)
    }
}

// Parse load average (platform-specific)
fn parse_loadavg() -> Result<LoadAverage, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::stats_admin::windows_stats::parse_loadavg_windows;
        return parse_loadavg_windows();
    }

    #[cfg(not(target_os = "windows"))]
    {
    let loadavg_content = fs::read_to_string("/proc/loadavg")
        .map_err(|e| format!("Failed to read /proc/loadavg: {}", e))?;

    let parts: Vec<&str> = loadavg_content.split_whitespace().collect();
    if parts.len() >= 3 {
        Ok(LoadAverage {
            one_minute: parts[0].parse().map_err(|e| format!("Failed to parse 1min load: {}", e))?,
            five_minutes: parts[1].parse().map_err(|e| format!("Failed to parse 5min load: {}", e))?,
            fifteen_minutes: parts[2].parse().map_err(|e| format!("Failed to parse 15min load: {}", e))?,
        })
    } else {
    Err("Invalid loadavg format".to_string())
    }
}
}

// Parse uptime information (platform-specific)
fn parse_uptime() -> Result<UptimeInfo, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::stats_admin::windows_stats::parse_uptime_windows;
        return parse_uptime_windows();
    }

    #[cfg(not(target_os = "windows"))]
    {
    let uptime_content = fs::read_to_string("/proc/uptime")
        .map_err(|e| format!("Failed to read /proc/uptime: {}", e))?;

    let parts: Vec<&str> = uptime_content.split_whitespace().collect();
    if parts.len() >= 2 {
        Ok(UptimeInfo {
            uptime_seconds: parts[0].parse().map_err(|e| format!("Failed to parse uptime: {}", e))?,
            idle_seconds: parts[1].parse().map_err(|e| format!("Failed to parse idle time: {}", e))?,
        })
    } else {
    Err("Invalid uptime format".to_string())
    }
}
}

// Parse CPU information (platform-specific)
fn parse_cpu_stat() -> Result<CpuInfo, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::stats_admin::windows_stats::parse_cpu_stat_windows;
        return parse_cpu_stat_windows();
    }

    #[cfg(not(target_os = "windows"))]
    {
    let stat_content = fs::read_to_string("/proc/stat")
        .map_err(|e| format!("Failed to read /proc/stat: {}", e))?;

    let mut cpu_info = CpuInfo {
        user: 0,
        nice: 0,
        system: 0,
        idle: 0,
        iowait: 0,
        irq: 0,
        softirq: 0,
        steal: 0,
    };

    for line in stat_content.lines() {
        if line.starts_with("cpu ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 8 {
                cpu_info.user = parts[1].parse().unwrap_or(0);
                cpu_info.nice = parts[2].parse().unwrap_or(0);
                cpu_info.system = parts[3].parse().unwrap_or(0);
                cpu_info.idle = parts[4].parse().unwrap_or(0);
                cpu_info.iowait = parts[5].parse().unwrap_or(0);
                cpu_info.irq = parts[6].parse().unwrap_or(0);
                cpu_info.softirq = parts[7].parse().unwrap_or(0);
                if parts.len() > 8 {
                    cpu_info.steal = parts[8].parse().unwrap_or(0);
                }
            }
            break;
        }
    }

    Ok(cpu_info)
    }
}

// Parse disk usage information (platform-specific)
fn parse_disk_usage() -> Result<Vec<DiskUsage>, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::stats_admin::windows_stats::parse_disk_usage_windows;
        return parse_disk_usage_windows();
    }

    #[cfg(not(target_os = "windows"))]
    {
    use std::process::Command;

    // Run df command to get disk usage information
    let output = Command::new("df")
        .arg("-h")  // Human readable format
        .arg("-P")  // POSIX format (portable)
        .output()
        .map_err(|e| format!("Failed to execute df command: {}", e))?;

    if !output.status.success() {
        return Err(format!("df command failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut disk_usage = Vec::new();

    for line in output_str.lines() {
        // Skip header line
        if line.starts_with("Filesystem") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            let filesystem = parts[0].to_string();
            let total_str = parts[1];
            let used_str = parts[2];
            let available_str = parts[3];
            let usage_percent_str = parts[4].trim_end_matches('%');
            let mount_point = parts[5..].join(" ");

            // Parse sizes (handle K, M, G, T suffixes)
            let total = parse_size_with_suffix(total_str)?;
            let used = parse_size_with_suffix(used_str)?;
            let available = parse_size_with_suffix(available_str)?;
            let usage_percent = usage_percent_str.parse::<f64>()
                .map_err(|e| format!("Failed to parse usage percentage: {}", e))?;

            disk_usage.push(DiskUsage {
                filesystem,
                total,
                used,
                available,
                usage_percent,
                mount_point,
            });
        }
    }

    Ok(disk_usage)
    }
}

// Parse size strings with K, M, G, T suffixes
fn parse_size_with_suffix(size_str: &str) -> Result<u64, String> {
    let size_str = size_str.trim();
    if size_str.is_empty() {
        return Ok(0);
    }

    let (number_part, suffix) = if size_str.ends_with('K') {
        (&size_str[..size_str.len()-1], "K")
    } else if size_str.ends_with('M') {
        (&size_str[..size_str.len()-1], "M")
    } else if size_str.ends_with('G') {
        (&size_str[..size_str.len()-1], "G")
    } else if size_str.ends_with('T') {
        (&size_str[..size_str.len()-1], "T")
    } else {
        (size_str, "")
    };

    let number: f64 = number_part.parse()
        .map_err(|e| format!("Failed to parse size number: {}", e))?;

    let multiplier = match suffix {
        "K" => 1024,
        "M" => 1024 * 1024,
        "G" => 1024 * 1024 * 1024,
        "T" => 1024_u64.pow(4),
        _ => 1,
    };

    Ok((number * multiplier as f64) as u64)
}

// Format bytes in human readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// Format uptime in human readable format
fn format_uptime(seconds: f64) -> String {
    let days = (seconds / 86400.0) as u64;
    let hours = ((seconds % 86400.0) / 3600.0) as u64;
    let minutes = ((seconds % 3600.0) / 60.0) as u64;
    let secs = (seconds % 60.0) as u64;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

// Calculate memory usage percentage
fn calculate_memory_usage(meminfo: &MemoryInfo) -> f64 {
    if meminfo.total > 0 {
        let used = meminfo.total - meminfo.available;
        (used as f64 / meminfo.total as f64) * 100.0
    } else {
        0.0
    }
}

// Calculate swap usage percentage
fn calculate_swap_usage(meminfo: &MemoryInfo) -> f64 {
    if meminfo.swap_total > 0 {
        let used = meminfo.swap_total - meminfo.swap_free;
        (used as f64 / meminfo.swap_total as f64) * 100.0
    } else {
        0.0
    }
}

// Calculate CPU usage percentage
fn calculate_cpu_usage(cpu_info: &CpuInfo) -> f64 {
    let total = cpu_info.user + cpu_info.nice + cpu_info.system + cpu_info.idle +
                cpu_info.iowait + cpu_info.irq + cpu_info.softirq + cpu_info.steal;
    let idle = cpu_info.idle + cpu_info.iowait;

    if total > 0 {
        ((total - idle) as f64 / total as f64) * 100.0
    } else {
        0.0
    }
}

// Generate stats admin panel HTML
fn generate_stats_panel(admin_key: &str) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>System Statistics</title>\n");
    html.push_str("<meta http-equiv=\"refresh\" content=\"30\">\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }\n");
    html.push_str(".container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\n");
    html.push_str("h1 { color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }\n");
    html.push_str(".stats-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin: 20px 0; }\n");
    html.push_str(".stat-card { background-color: #f8f9fa; padding: 20px; border-radius: 8px; border-left: 4px solid #007bff; }\n");
    html.push_str(".stat-card h3 { margin-top: 0; color: #333; }\n");
    html.push_str(".stat-item { display: flex; justify-content: space-between; margin: 10px 0; padding: 5px 0; border-bottom: 1px solid #dee2e6; }\n");
    html.push_str(".stat-label { font-weight: bold; color: #555; }\n");
    html.push_str(".stat-value { color: #333; }\n");
    html.push_str(".progress-bar { width: 100%; height: 20px; background-color: #e9ecef; border-radius: 10px; overflow: hidden; margin: 5px 0; }\n");
    html.push_str(".progress-fill { height: 100%; background-color: #007bff; transition: width 0.3s ease; }\n");
    html.push_str(".progress-fill.high { background-color: #dc3545; }\n");
    html.push_str(".progress-fill.medium { background-color: #ffc107; }\n");
    html.push_str(".refresh-info { text-align: center; color: #666; font-size: 0.9em; margin-top: 20px; }\n");
    html.push_str(".error { background-color: #f8d7da; color: #721c24; padding: 15px; border-radius: 4px; margin: 10px 0; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("<div class=\"container\">\n");

    html.push_str("<h1>&#x1F4CA; System Statistics</h1>\n");

    // Memory information
    html.push_str("<div class=\"stats-grid\">\n");
    html.push_str("<div class=\"stat-card\">\n");
    html.push_str("<h3>&#x1F4BE; Memory Usage</h3>\n");

    match parse_meminfo() {
        Ok(meminfo) => {
            let memory_usage = calculate_memory_usage(&meminfo);
            let swap_usage = calculate_swap_usage(&meminfo);

            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">Total Memory:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{}</span>\n", format_bytes(meminfo.total)));
            html.push_str(&format!("</div>\n"));

            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">Available:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{}</span>\n", format_bytes(meminfo.available)));
            html.push_str(&format!("</div>\n"));

            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">Used:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{}</span>\n", format_bytes(meminfo.total - meminfo.available)));
            html.push_str(&format!("</div>\n"));

            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">Usage:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{:.1}%</span>\n", memory_usage));
            html.push_str(&format!("</div>\n"));

            let progress_class = if memory_usage > 80.0 { "high" } else if memory_usage > 60.0 { "medium" } else { "" };
            html.push_str(&format!("<div class=\"progress-bar\">\n"));
            html.push_str(&format!("<div class=\"progress-fill {} \" style=\"width: {:.1}%\"></div>\n", progress_class, memory_usage));
            html.push_str(&format!("</div>\n"));

            if meminfo.swap_total > 0 {
                html.push_str(&format!("<div class=\"stat-item\">\n"));
                html.push_str(&format!("<span class=\"stat-label\">Swap Total:</span>\n"));
                html.push_str(&format!("<span class=\"stat-value\">{}</span>\n", format_bytes(meminfo.swap_total)));
                html.push_str(&format!("</div>\n"));

                html.push_str(&format!("<div class=\"stat-item\">\n"));
                html.push_str(&format!("<span class=\"stat-label\">Swap Used:</span>\n"));
                html.push_str(&format!("<span class=\"stat-value\">{}</span>\n", format_bytes(meminfo.swap_total - meminfo.swap_free)));
                html.push_str(&format!("</div>\n"));

                html.push_str(&format!("<div class=\"stat-item\">\n"));
                html.push_str(&format!("<span class=\"stat-label\">Swap Usage:</span>\n"));
                html.push_str(&format!("<span class=\"stat-value\">{:.1}%</span>\n", swap_usage));
                html.push_str(&format!("</div>\n"));
            }
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"error\">Error reading memory info: {}</div>\n", html_escape(&e)));
        }
    }

    html.push_str("</div>\n");

    // Load average information
    html.push_str("<div class=\"stat-card\">\n");
    html.push_str("<h3>&#x26A1; Load Average</h3>\n");

    match parse_loadavg() {
        Ok(loadavg) => {
            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">1 minute:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{:.2}</span>\n", loadavg.one_minute));
            html.push_str(&format!("</div>\n"));

            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">5 minutes:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{:.2}</span>\n", loadavg.five_minutes));
            html.push_str(&format!("</div>\n"));

            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">15 minutes:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{:.2}</span>\n", loadavg.fifteen_minutes));
            html.push_str(&format!("</div>\n"));
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"error\">Error reading load average: {}</div>\n", html_escape(&e)));
        }
    }

    html.push_str("</div>\n");

    // System uptime
    html.push_str("<div class=\"stat-card\">\n");
    html.push_str("<h3>&#x23F1;&#xFE0F; System Uptime</h3>\n");

    match parse_uptime() {
        Ok(uptime) => {
            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">Uptime:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{}</span>\n", format_uptime(uptime.uptime_seconds)));
            html.push_str(&format!("</div>\n"));
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"error\">Error reading uptime: {}</div>\n", html_escape(&e)));
        }
    }

    html.push_str("</div>\n");

    // CPU information
    html.push_str("<div class=\"stat-card\">\n");
    html.push_str("<h3>&#x1F5A5;&#xFE0F; CPU Usage</h3>\n");

    match parse_cpu_stat() {
        Ok(cpu_info) => {
            let cpu_usage = calculate_cpu_usage(&cpu_info);

            html.push_str(&format!("<div class=\"stat-item\">\n"));
            html.push_str(&format!("<span class=\"stat-label\">CPU Usage:</span>\n"));
            html.push_str(&format!("<span class=\"stat-value\">{:.1}%</span>\n", cpu_usage));
            html.push_str(&format!("</div>\n"));

            let progress_class = if cpu_usage > 80.0 { "high" } else if cpu_usage > 60.0 { "medium" } else { "" };
            html.push_str(&format!("<div class=\"progress-bar\">\n"));
            html.push_str(&format!("<div class=\"progress-fill {} \" style=\"width: {:.1}%\"></div>\n", progress_class, cpu_usage));
            html.push_str(&format!("</div>\n"));
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"error\">Error reading CPU info: {}</div>\n", html_escape(&e)));
        }
    }

    html.push_str("</div>\n");

    // Disk usage information
    html.push_str("<div class=\"stat-card\">\n");
    html.push_str("<h3>&#x1F4BE; Disk Usage</h3>\n");

    match parse_disk_usage() {
        Ok(disk_usage) => {
            for disk in disk_usage {
                html.push_str(&format!("<div class=\"stat-item\">\n"));
                html.push_str(&format!("<span class=\"stat-label\">{} ({})</span>\n",
                                     html_escape(&disk.filesystem), html_escape(&disk.mount_point)));
                html.push_str(&format!("<span class=\"stat-value\">{:.1}%</span>\n", disk.usage_percent));
                html.push_str(&format!("</div>\n"));

                html.push_str(&format!("<div class=\"stat-item\">\n"));
                html.push_str(&format!("<span class=\"stat-label\">Used:</span>\n"));
                html.push_str(&format!("<span class=\"stat-value\">{} / {}</span>\n",
                                     format_bytes(disk.used), format_bytes(disk.total)));
                html.push_str(&format!("</div>\n"));

                html.push_str(&format!("<div class=\"stat-item\">\n"));
                html.push_str(&format!("<span class=\"stat-label\">Available:</span>\n"));
                html.push_str(&format!("<span class=\"stat-value\">{}</span>\n", format_bytes(disk.available)));
                html.push_str(&format!("</div>\n"));

                let progress_class = if disk.usage_percent > 90.0 { "high" } else if disk.usage_percent > 80.0 { "medium" } else { "" };
                html.push_str(&format!("<div class=\"progress-bar\">\n"));
                html.push_str(&format!("<div class=\"progress-fill {} \" style=\"width: {:.1}%\"></div>\n", progress_class, disk.usage_percent));
                html.push_str(&format!("</div>\n"));

                html.push_str("<br>\n"); // Add some spacing between disks
            }
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"error\">Error reading disk usage: {}</div>\n", html_escape(&e)));
        }
    }

    html.push_str("</div>\n");
    html.push_str("</div>\n");

    // Hourly Statistics Charts
    html.push_str("<div class=\"stats-grid\">\n");
    html.push_str("<div class=\"stat-card\" style=\"grid-column: 1 / -1;\">\n");
    html.push_str("<h3>&#x1F4C8; Hourly Statistics (Last 48 Hours)</h3>\n");

    match load_hourly_stats() {
        Ok(stats) => {
            if stats.is_empty() {
                html.push_str("<p>No hourly statistics available yet. Data will appear after the server has been running for at least one hour.</p>\n");
            } else {
                // Generate JavaScript for charts
                html.push_str("<div style=\"display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 20px; margin: 20px 0;\">\n");

                // Memory Usage Chart
                html.push_str("<div>\n");
                html.push_str("<h4>Memory Usage (MB)</h4>\n");
                html.push_str("<canvas id=\"memoryChart\" width=\"300\" height=\"200\"></canvas>\n");
                html.push_str("</div>\n");

                // CPU Usage Chart
                html.push_str("<div>\n");
                html.push_str("<h4>CPU Usage (%)</h4>\n");
                html.push_str("<canvas id=\"cpuChart\" width=\"300\" height=\"200\"></canvas>\n");
                html.push_str("</div>\n");

                // Request Count Chart
                html.push_str("<div>\n");
                html.push_str("<h4>Request Count</h4>\n");
                html.push_str("<canvas id=\"requestChart\" width=\"300\" height=\"200\"></canvas>\n");
                html.push_str("</div>\n");

                html.push_str("</div>\n");

                // Generate JavaScript data
                let labels: Vec<String> = stats.iter().map(|s| format_timestamp(s.timestamp)).collect();
                let memory_data: Vec<f64> = stats.iter().map(|s| s.memory_used_mb).collect();
                let cpu_data: Vec<f64> = stats.iter().map(|s| s.cpu_usage_percent).collect();
                let request_data: Vec<u64> = stats.iter().map(|s| s.request_count).collect();

                html.push_str("<script>\n");
                html.push_str("const chartData = {\n");
                html.push_str(&format!("  labels: {:?},\n", labels));
                html.push_str(&format!("  memory: {:?},\n", memory_data));
                html.push_str(&format!("  cpu: {:?},\n", cpu_data));
                html.push_str(&format!("  requests: {:?}\n", request_data));
                html.push_str("};\n");

                // Simple chart drawing function
                html.push_str("function drawChart(canvasId, data, label, color) {\n");
                html.push_str("  const canvas = document.getElementById(canvasId);\n");
                html.push_str("  const ctx = canvas.getContext('2d');\n");
                html.push_str("  const width = canvas.width;\n");
                html.push_str("  const height = canvas.height;\n");
                html.push_str("  \n");
                html.push_str("  ctx.clearRect(0, 0, width, height);\n");
                html.push_str("  \n");
                html.push_str("  if (data.length === 0) return;\n");
                html.push_str("  \n");
                html.push_str("  const max = Math.max(...data);\n");
                html.push_str("  const min = Math.min(...data);\n");
                html.push_str("  const range = max - min || 1;\n");
                html.push_str("  \n");
                html.push_str("  ctx.strokeStyle = color;\n");
                html.push_str("  ctx.lineWidth = 2;\n");
                html.push_str("  ctx.beginPath();\n");
                html.push_str("  \n");
                html.push_str("  data.forEach((value, index) => {\n");
                html.push_str("    const x = (index / (data.length - 1)) * (width - 40) + 20;\n");
                html.push_str("    const y = height - 20 - ((value - min) / range) * (height - 40);\n");
                html.push_str("    \n");
                html.push_str("    if (index === 0) {\n");
                html.push_str("      ctx.moveTo(x, y);\n");
                html.push_str("    } else {\n");
                html.push_str("      ctx.lineTo(x, y);\n");
                html.push_str("    }\n");
                html.push_str("  });\n");
                html.push_str("  \n");
                html.push_str("  ctx.stroke();\n");
                html.push_str("  \n");
                html.push_str("  // Draw data points\n");
                html.push_str("  ctx.fillStyle = color;\n");
                html.push_str("  data.forEach((value, index) => {\n");
                html.push_str("    const x = (index / (data.length - 1)) * (width - 40) + 20;\n");
                html.push_str("    const y = height - 20 - ((value - min) / range) * (height - 40);\n");
                html.push_str("    ctx.beginPath();\n");
                html.push_str("    ctx.arc(x, y, 3, 0, 2 * Math.PI);\n");
                html.push_str("    ctx.fill();\n");
                html.push_str("  });\n");
                html.push_str("}\n");
                html.push_str("\n");
                html.push_str("// Draw all charts\n");
                html.push_str("drawChart('memoryChart', chartData.memory, 'Memory (MB)', '#007bff');\n");
                html.push_str("drawChart('cpuChart', chartData.cpu, 'CPU (%)', '#28a745');\n");
                html.push_str("drawChart('requestChart', chartData.requests, 'Requests', '#dc3545');\n");
                html.push_str("</script>\n");

                // Summary statistics
                if let (Some(last_memory), Some(last_cpu), Some(last_requests)) = (
                    memory_data.last(),
                    cpu_data.last(),
                    request_data.last()
                ) {
                    html.push_str("<div style=\"margin-top: 20px; padding: 15px; background-color: #f8f9fa; border-radius: 8px;\">\n");
                    html.push_str("<h4>Latest Hour Summary</h4>\n");
                    html.push_str(&format!("<p><strong>Memory Usage:</strong> {:.1} MB</p>\n", last_memory));
                    html.push_str(&format!("<p><strong>CPU Usage:</strong> {:.1}%</p>\n", last_cpu));
                    html.push_str(&format!("<p><strong>Requests:</strong> {}</p>\n", last_requests));
                    html.push_str("</div>\n");
                }
            }
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"error\">Error loading hourly stats: {}</div>\n", html_escape(&e)));
        }
    }

    html.push_str("</div>\n");
    html.push_str("</div>\n");

    html.push_str("<div class=\"refresh-info\">\n");
    html.push_str("<p>This page refreshes automatically every 30 seconds</p>\n");
    html.push_str(&format!("<p>Last updated: {}</p>\n", get_current_time()));
    html.push_str("</div>\n");

    html.push_str("</div>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

// Get current time in a simple format
fn get_current_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let total_seconds = now.as_secs();
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// Load hourly statistics from file
fn load_hourly_stats() -> Result<Vec<HourlyStats>, String> {
    let stats_file = "/var/lib/easyp/stats/hourly_stats.json";

    if !std::path::Path::new(stats_file).exists() {
        return Ok(Vec::new()); // No data yet
    }

    let content = fs::read_to_string(stats_file)
        .map_err(|e| format!("Failed to read stats file: {}", e))?;

    let mut stats = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() == 4 {
            if let (Ok(timestamp), Ok(memory_used_mb), Ok(cpu_usage_percent), Ok(request_count)) = (
                parts[0].parse::<u64>(),
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
                parts[3].parse::<u64>(),
            ) {
                stats.push(HourlyStats {
                    timestamp,
                    memory_used_mb,
                    cpu_usage_percent,
                    request_count,
                });
            }
        }
    }

    Ok(stats)
}

// Format timestamp for display
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};

    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime = SystemTime::from(datetime);

    // Simple formatting - you could use a proper date library for better formatting
    let total_seconds = timestamp;
    let hours = (total_seconds / 3600) % 24;
    let days = total_seconds / 86400;

    format!("Day {} {:02}:00", days, hours)
}

// HTML escape function
fn html_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

// Main admin handler
pub fn handle_stats_admin_request(
    path: &str,
    method: &str,
    _query_string: &str,
    _body: &str,
    _headers: &HashMap<String, String>,
    admin_keys: &std::collections::HashMap<String, String>,
) -> Result<String, String> {
    // Check if this looks like a stats admin request
    if !path.starts_with("/stats_") {
        return Err("Not a stats admin request".to_string());
    }

    // Get admin key from memory and validate
    let admin_key = admin_keys.get("stats")
        .ok_or("Stats admin key not found".to_string())?;
    let expected_path = format!("/stats_{}", admin_key);

    if path != expected_path {
        return Err("Invalid admin key".to_string());
    }

    // Handle GET requests (display stats panel)
    if method == "GET" {
        let html = generate_stats_panel(admin_key);

        return Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
            html
        ));
    }

    Err("Method not allowed".to_string())
}

// Windows-specific system monitoring functions
#[cfg(target_os = "windows")]
mod windows_stats {
    use super::*;
    use std::process::Command;

    pub fn parse_meminfo_windows() -> Result<MemoryInfo, String> {
        // Use PowerShell to get memory information
        let ps_command = r#"
        $os = Get-CimInstance -ClassName Win32_OperatingSystem
        $cs = Get-CimInstance -ClassName Win32_ComputerSystem
        $total = $cs.TotalPhysicalMemory
        $free = $os.FreePhysicalMemory * 1024
        $available = $free
        $used = $total - $free
        $swap_total = $os.TotalVirtualMemorySize - $total
        $swap_free = $os.FreeVirtualMemory * 1024
        $swap_used = $swap_total - $swap_free
        Write-Output "$total,$free,$available,0,0,$swap_total,$swap_used"
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = output_str.split(',').collect();

        if parts.len() != 7 {
            return Err("Invalid PowerShell output format".to_string());
        }

        Ok(MemoryInfo {
            total: parts[0].parse().unwrap_or(0),
            free: parts[1].parse().unwrap_or(0),
            available: parts[2].parse().unwrap_or(0),
            buffers: parts[3].parse().unwrap_or(0),
            cached: parts[4].parse().unwrap_or(0),
            swap_total: parts[5].parse().unwrap_or(0),
            swap_free: parts[6].parse().unwrap_or(0),
        })
    }

    pub fn parse_loadavg_windows() -> Result<LoadAverage, String> {
        // Windows doesn't have load average in the same way as Unix
        // We'll use CPU usage as a proxy
        let ps_command = r#"
        $cpu = Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 1
        $load = $cpu.CounterSamples[0].CookedValue / 100.0
        Write-Output "$load,$load,$load"
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = output_str.split(',').collect();

        if parts.len() != 3 {
            return Err("Invalid PowerShell output format".to_string());
        }

        Ok(LoadAverage {
            one_minute: parts[0].parse().unwrap_or(0.0),
            five_minutes: parts[1].parse().unwrap_or(0.0),
            fifteen_minutes: parts[2].parse().unwrap_or(0.0),
        })
    }

    pub fn parse_uptime_windows() -> Result<UptimeInfo, String> {
        let ps_command = r#"
        $os = Get-CimInstance -ClassName Win32_OperatingSystem
        $uptime = (Get-Date) - $os.LastBootUpTime
        $uptime_seconds = $uptime.TotalSeconds
        Write-Output "$uptime_seconds,0"
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = output_str.split(',').collect();

        if parts.len() != 2 {
            return Err("Invalid PowerShell output format".to_string());
        }

        Ok(UptimeInfo {
            uptime_seconds: parts[0].parse().unwrap_or(0.0),
            idle_seconds: parts[1].parse().unwrap_or(0.0),
        })
    }

    pub fn parse_cpu_stat_windows() -> Result<CpuInfo, String> {
        let ps_command = r#"
        $cpu = Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 2
        $current = $cpu.CounterSamples[0].CookedValue
        $previous = $cpu.CounterSamples[1].CookedValue
        $usage = ($current + $previous) / 2
        Write-Output "$usage,0,0,0,0,0,0,0,0,0,0"
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = output_str.split(',').collect();

        if parts.len() != 11 {
            return Err("Invalid PowerShell output format".to_string());
        }

        Ok(CpuInfo {
            user: parts[0].parse().unwrap_or(0),
            nice: parts[1].parse().unwrap_or(0),
            system: parts[2].parse().unwrap_or(0),
            idle: parts[3].parse().unwrap_or(0),
            iowait: parts[4].parse().unwrap_or(0),
            irq: parts[5].parse().unwrap_or(0),
            softirq: parts[6].parse().unwrap_or(0),
            steal: parts[7].parse().unwrap_or(0),
        })
    }

    pub fn parse_disk_usage_windows() -> Result<Vec<DiskUsage>, String> {
        let ps_command = r#"
        Get-WmiObject -Class Win32_LogicalDisk | Where-Object {$_.DriveType -eq 3} | ForEach-Object {
            $size = $_.Size
            $free = $_.FreeSpace
            $used = $size - $free
            $percent = if ($size -gt 0) { ($used / $size) * 100 } else { 0 }
            Write-Output "$($_.DeviceID),$size,$used,$free,$percent,$($_.VolumeName)"
        }
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut disk_usage = Vec::new();

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 6 {
                if let (Ok(total), Ok(used), Ok(available), Ok(usage_percent)) = (
                    parts[1].parse::<u64>(),
                    parts[2].parse::<u64>(),
                    parts[3].parse::<u64>(),
                    parts[4].parse::<f64>(),
                ) {
                    disk_usage.push(DiskUsage {
                        filesystem: parts[0].to_string(),
                        total,
                        used,
                        available,
                        usage_percent,
                        mount_point: parts[5].to_string(),
                    });
                }
            }
        }

        Ok(disk_usage)
    }
}

// Get admin paths
pub fn get_stats_admin_paths() -> Vec<String> {
    vec!["/stats_".to_string()]
}
