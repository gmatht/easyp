// hourly_stats.rs - Hourly statistics collection and storage
// Tracks memory usage, CPU usage, and request counts for the last 48 hours

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::fs;
use std::path::Path;

/// Single hour's statistics
#[derive(Debug, Clone)]
pub struct HourlyStats {
    pub timestamp: u64,        // Unix timestamp of the hour
    pub memory_used_mb: f64,   // Memory usage in MB
    pub cpu_usage_percent: f64, // CPU usage percentage
    pub request_count: u64,    // Number of requests in this hour
}

impl HourlyStats {
    /// Serialize to TSV format (tab-separated values)
    fn to_tsv(&self) -> String {
        format!("{}\t{}\t{}\t{}",
            self.timestamp,
            self.memory_used_mb,
            self.cpu_usage_percent,
            self.request_count
        )
    }

    /// Deserialize from TSV format
    fn from_tsv_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() == 4 {
            Some(HourlyStats {
                timestamp: parts[0].parse().ok()?,
                memory_used_mb: parts[1].parse().ok()?,
                cpu_usage_percent: parts[2].parse().ok()?,
                request_count: parts[3].parse().ok()?,
            })
        } else {
            None
        }
    }
}

/// Statistics collector that maintains 48 hours of data
#[derive(Debug)]
pub struct HourlyStatsCollector {
    stats: Arc<Mutex<VecDeque<HourlyStats>>>,
    current_hour_requests: Arc<Mutex<u64>>,
    pub data_file: String,
}

impl HourlyStatsCollector {
    /// Create a new stats collector
    pub fn new(data_file: String) -> Self {
        let collector = Self {
            stats: Arc::new(Mutex::new(VecDeque::new())),
            current_hour_requests: Arc::new(Mutex::new(0)),
            data_file,
        };

        // Load existing data
        if let Err(e) = collector.load_stats() {
            eprintln!("Warning: Failed to load existing stats: {}", e);
        }

        collector
    }

    /// Record a new request
    pub fn record_request(&self) {
        if let Ok(mut count) = self.current_hour_requests.lock() {
            *count += 1;
        }
    }

    /// Collect and store current hour's statistics
    pub fn collect_current_stats(&self) -> Result<(), String> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to get current time: {}", e))?
            .as_secs();

        // Round down to the current hour
        let current_hour = (current_time / 3600) * 3600;

        // Get current hour's request count
        let request_count = {
            let mut count = self.current_hour_requests.lock()
                .map_err(|e| format!("Failed to lock request counter: {}", e))?;
            let count_value = *count;
            *count = 0; // Reset for next hour
            count_value
        };

        // Get system stats
        let memory_used_mb = self.get_memory_usage()?;
        let cpu_usage_percent = self.get_cpu_usage()?;

        // Create new stats entry
        let new_stats = HourlyStats {
            timestamp: current_hour,
            memory_used_mb,
            cpu_usage_percent,
            request_count,
        };

        // Add to collection and maintain 48-hour window
        {
            let mut stats = self.stats.lock()
                .map_err(|e| format!("Failed to lock stats: {}", e))?;

            // Remove old entries (older than 48 hours)
            let cutoff_time = current_hour - (48 * 3600);
            while let Some(front) = stats.front() {
                if front.timestamp < cutoff_time {
                    stats.pop_front();
                } else {
                    break;
                }
            }

            // Add new entry if it's a new hour, or update existing entry
            if let Some(back) = stats.back_mut() {
                if back.timestamp == current_hour {
                    // Update existing entry
                    *back = new_stats;
                } else {
                    // Add new entry
                    stats.push_back(new_stats);
                }
            } else {
                // First entry
                stats.push_back(new_stats);
            }
        }

        // Save to disk
        self.save_stats()?;

        Ok(())
    }

    /// Get the last 48 hours of statistics
    pub fn get_stats(&self) -> Result<Vec<HourlyStats>, String> {
        let stats = self.stats.lock()
            .map_err(|e| format!("Failed to lock stats: {}", e))?;
        Ok(stats.iter().cloned().collect())
    }

    /// Get memory usage in MB
    fn get_memory_usage(&self) -> Result<f64, String> {
        #[cfg(target_os = "windows")]
        {
            self.get_memory_usage_windows()
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.get_memory_usage_unix()
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_memory_usage_unix(&self) -> Result<f64, String> {
        let meminfo_content = fs::read_to_string("/proc/meminfo")
            .map_err(|e| format!("Failed to read /proc/meminfo: {}", e))?;

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;

        for line in meminfo_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(value) = parts[1].parse::<u64>() {
                    match parts[0] {
                        "MemTotal:" => total_kb = value,
                        "MemAvailable:" => available_kb = value,
                        _ => {}
                    }
                }
            }
        }

        if total_kb == 0 {
            return Err("Could not determine total memory".to_string());
        }

        let used_kb = total_kb - available_kb;
        let used_mb = (used_kb as f64) / 1024.0;

        Ok(used_mb)
    }

    #[cfg(target_os = "windows")]
    fn get_memory_usage_windows(&self) -> Result<f64, String> {
        use std::process::Command;

        let ps_command = r#"
        $os = Get-CimInstance -ClassName Win32_OperatingSystem
        $cs = Get-CimInstance -ClassName Win32_ComputerSystem
        $total = $cs.TotalPhysicalMemory
        $free = $os.FreePhysicalMemory * 1024
        $used = $total - $free
        Write-Output $used
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let used_bytes: u64 = output_str.trim().parse()
            .map_err(|e| format!("Failed to parse memory usage: {}", e))?;

        let used_mb = (used_bytes as f64) / (1024.0 * 1024.0);
        Ok(used_mb)
    }

    /// Get CPU usage percentage
    fn get_cpu_usage(&self) -> Result<f64, String> {
        #[cfg(target_os = "windows")]
        {
            self.get_cpu_usage_windows()
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.get_cpu_usage_unix()
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_cpu_usage_unix(&self) -> Result<f64, String> {
        // Read /proc/stat twice with a small delay to calculate CPU usage
        let stat1 = fs::read_to_string("/proc/stat")
            .map_err(|e| format!("Failed to read /proc/stat: {}", e))?;

        std::thread::sleep(Duration::from_millis(100));

        let stat2 = fs::read_to_string("/proc/stat")
            .map_err(|e| format!("Failed to read /proc/stat: {}", e))?;

        let parse_cpu_line = |line: &str| -> Result<(u64, u64), String> {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                return Err("Invalid CPU line format".to_string());
            }

            let user: u64 = parts[1].parse().unwrap_or(0);
            let nice: u64 = parts[2].parse().unwrap_or(0);
            let system: u64 = parts[3].parse().unwrap_or(0);
            let idle: u64 = parts[4].parse().unwrap_or(0);
            let iowait: u64 = parts[5].parse().unwrap_or(0);
            let irq: u64 = parts[6].parse().unwrap_or(0);
            let softirq: u64 = parts[7].parse().unwrap_or(0);
            let steal: u64 = if parts.len() > 8 { parts[8].parse().unwrap_or(0) } else { 0 };

            let total = user + nice + system + idle + iowait + irq + softirq + steal;
            let idle_total = idle + iowait;

            Ok((total, idle_total))
        };

        let (total1, idle1) = parse_cpu_line(stat1.lines().next().unwrap_or(""))?;
        let (total2, idle2) = parse_cpu_line(stat2.lines().next().unwrap_or(""))?;

        let total_diff = total2 - total1;
        let idle_diff = idle2 - idle1;

        if total_diff == 0 {
            return Ok(0.0);
        }

        let cpu_usage = ((total_diff - idle_diff) as f64 / total_diff as f64) * 100.0;
        Ok(cpu_usage)
    }

    #[cfg(target_os = "windows")]
    fn get_cpu_usage_windows(&self) -> Result<f64, String> {
        use std::process::Command;

        let ps_command = r#"
        $cpu = Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 1
        $usage = $cpu.CounterSamples[0].CookedValue
        Write-Output $usage
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", ps_command])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let usage: f64 = output_str.trim().parse()
            .map_err(|e| format!("Failed to parse CPU usage: {}", e))?;

        Ok(usage)
    }

    /// Save statistics to disk in TSV format
    fn save_stats(&self) -> Result<(), String> {
        let stats = self.stats.lock()
            .map_err(|e| format!("Failed to lock stats: {}", e))?;

        // Convert to TSV format (one entry per line)
        let mut tsv = String::new();
        for stat in stats.iter() {
            tsv.push_str(&stat.to_tsv());
            tsv.push('\n');
        }

        // Ensure directory exists
        if let Some(parent) = Path::new(&self.data_file).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| {
                    format!("Failed to create stats directory '{}': {} (os error {})",
                           parent.display(), e, e.raw_os_error().unwrap_or(0))
                })?;
        }

        fs::write(&self.data_file, tsv)
            .map_err(|e| {
                format!("Failed to write stats file '{}': {} (os error {})",
                       self.data_file, e, e.raw_os_error().unwrap_or(0))
            })?;

        Ok(())
    }

    /// Load statistics from disk (TSV format)
    fn load_stats(&self) -> Result<(), String> {
        if !Path::new(&self.data_file).exists() {
            return Ok(()); // No existing data
        }

        let content = fs::read_to_string(&self.data_file)
            .map_err(|e| format!("Failed to read stats file: {}", e))?;

        let mut stats = self.stats.lock()
            .map_err(|e| format!("Failed to lock stats: {}", e))?;

        // Parse TSV format
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(stat) = HourlyStats::from_tsv_line(line) {
                stats.push_back(stat);
            } else {
                eprintln!("Warning: Failed to parse stats line: {}", line);
            }
        }

        Ok(())
    }
}

/// Background task that collects stats every hour
pub async fn start_stats_collection_task(collector: Arc<HourlyStatsCollector>) {
    let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour

    loop {
        interval.tick().await;

        if let Err(e) = collector.collect_current_stats() {
            eprintln!("⚠️  Failed to collect hourly stats: {}", e);
            eprintln!("ℹ️  Stats file: {}", collector.data_file);
            eprintln!("ℹ️  This is not critical - the server will continue running normally");
        }
    }
}
