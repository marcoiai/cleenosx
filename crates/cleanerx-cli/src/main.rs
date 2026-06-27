use cleanerx_core::{Finding, LogLevel, RiskLevel, ScanLog, UsageNode, VolumeInfo};
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const ORANGE: &str = "\x1b[38;5;208m";
const RED: &str = "\x1b[31m";
const BLUE: &str = "\x1b[34m";
const GRAY: &str = "\x1b[90m";

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--recovery-script") {
        println!("{}", cleanerx_core::generate_recovery_script());
        return;
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    title();
    println!("{YELLOW}Cleanup requires explicit typed confirmation. Scans are read-only until cleanup is chosen.{RESET}");

    loop {
        println!("\n{BOLD}Menu{RESET}");
        println!("1) 📊 Dashboard overview");
        println!("2) 💽 List volumes and APFS hints");
        println!("3) 📁 Scan /System/Volumes/Data");
        println!("4) 🧰 Scan developer tools");
        println!("5) 🧱 Scan AssetsV2");
        println!("6) 🕰️  List local snapshots");
        println!("7) 🛟 Print Recovery script");
        println!("8) 🚪 Exit");

        match prompt("Choose an option") {
            Some(choice) if choice == "1" => dashboard(),
            Some(choice) if choice == "2" => volumes(),
            Some(choice) if choice == "3" => data_usage(),
            Some(choice) if choice == "4" => developer_tools(),
            Some(choice) if choice == "5" => assets_v2(),
            Some(choice) if choice == "6" => snapshots(),
            Some(choice) if choice == "7" => recovery_script(),
            Some(choice) if choice == "8" => break,
            None => break,
            _ => println!("{YELLOW}Choose 1-8.{RESET}"),
        }
    }
}

fn print_help() {
    title();
    println!("Usage: cx");
    println!("       cx --recovery-script > cx.sh");
    println!();
    println!("Starts the guided terminal interface and can print the Recovery cleanup script.");
}

fn title() {
    println!("\n{BOLD}{BLUE}cleenosx CLI{RESET}");
    println!("{GRAY}Guided macOS storage investigation and explicit cleanup workflows.{RESET}");
    println!("{GRAY}--------------------------------------------------------{RESET}");
}

fn dashboard() {
    let started = spinner_start("Scanning overview");
    let result = cleanerx_core::scan_overview();
    spinner_done(started, "Overview scan");
    print_logs(&result.logs);

    println!("\n{BOLD}Storage Summary{RESET}");
    println!(
        "Total: {}",
        format_bytes_opt(result.data.summary.total_bytes)
    );
    println!("Used: {}", format_bytes_opt(result.data.summary.used_bytes));
    println!(
        "Free: {}",
        format_bytes_opt(result.data.summary.available_bytes)
    );
    if let Some(percent) = result.data.summary.percent_used {
        println!("Used %: {percent:.1}%");
    }

    print_findings(&result.data.findings);
}

fn volumes() {
    let started = spinner_start("Scanning volumes");
    let result = cleanerx_core::scan_volumes();
    spinner_done(started, "Volume scan");
    print_logs(&result.logs);
    print_volumes(&result.data);
}

fn data_usage() {
    let started = spinner_start("Scanning Data volume blocks");
    let result = cleanerx_core::scan_data_usage();
    spinner_done(started, "Data usage scan");
    print_logs(&result.logs);
    print_usage(&result.data);
}

fn developer_tools() {
    let started = spinner_start("Scanning developer tool storage");
    let mut findings = cleanerx_core::scan_developer_tools();
    findings
        .data
        .extend(cleanerx_core::scan_rust_artifacts().data);
    findings.data.extend(cleanerx_core::scan_containers().data);
    spinner_done(started, "Developer scan");
    print_logs(&findings.logs);
    print_findings(&findings.data);
}

fn assets_v2() {
    let started = spinner_start("Scanning AssetsV2");
    let result = cleanerx_core::scan_assets_v2();
    spinner_done(started, "AssetsV2 scan");
    print_logs(&result.logs);
    print_findings(&result.data);
}

fn snapshots() {
    let started = spinner_start("Listing local snapshots");
    let result = cleanerx_core::list_snapshots();
    spinner_done(started, "Snapshot scan");
    print_logs(&result.logs);
    print_findings(&result.data);
}

fn recovery_script() {
    println!("\n{BOLD}Recovery Script{RESET}");
    println!("{DIM}Review before running. Destructive cleanup requires typed confirmations in Recovery.{RESET}\n");
    println!("{}", cleanerx_core::generate_recovery_script());
}

fn prompt(label: &str) -> Option<String> {
    print!("{label}: ");
    io::stdout().flush().ok()?;
    let mut input = String::new();
    let bytes_read = io::stdin().read_line(&mut input).ok()?;
    if bytes_read == 0 {
        return None;
    }
    Some(input.trim().to_string())
}

fn spinner_start(label: &'static str) -> Instant {
    let start = Instant::now();
    print!("{BLUE}⏳ {label}...{RESET}");
    io::stdout().flush().ok();
    thread::sleep(Duration::from_millis(180));
    start
}

fn spinner_done(start: Instant, label: &str) {
    println!(
        "\r{GREEN}✓ {label} finished in {:.1}s{RESET}",
        start.elapsed().as_secs_f32()
    );
}

fn print_logs(logs: &[ScanLog]) {
    if logs.is_empty() {
        return;
    }

    println!("\n{BOLD}Logs{RESET}");
    for log in logs {
        let prefix = match log.level {
            LogLevel::Info => format!("{BLUE}info{RESET}"),
            LogLevel::Warning => format!("{YELLOW}warn{RESET}"),
            LogLevel::Error => format!("{RED}error{RESET}"),
        };
        println!("{prefix} {}", log.message);
    }
}

fn print_volumes(volumes: &[VolumeInfo]) {
    println!("\n{BOLD}Volumes{RESET}");
    if volumes.is_empty() {
        println!("{YELLOW}No volumes parsed.{RESET}");
        return;
    }

    for volume in volumes {
        println!(
            "{} {} {} {}",
            risk_label(&volume.risk),
            volume.identifier,
            volume.role.clone().unwrap_or_else(|| "Unknown".to_string()),
            volume
                .mount_point
                .clone()
                .unwrap_or_else(|| "Not mounted".to_string())
        );
        println!(
            "   name: {} | used: {} | free: {}",
            volume.name,
            format_bytes_opt(volume.used_bytes),
            format_bytes_opt(volume.available_bytes)
        );
        for note in &volume.notes {
            println!("   {YELLOW}{note}{RESET}");
        }
    }
}

fn print_usage(nodes: &[UsageNode]) {
    println!("\n{BOLD}Large Blocks{RESET}");
    if nodes.is_empty() {
        println!("{YELLOW}No usage nodes parsed. Permission or mount state may be limiting the scan.{RESET}");
        return;
    }

    for node in nodes.iter().take(24) {
        println!(
            "{} {:>10} {}",
            risk_label(&node.risk),
            format_bytes(node.size_bytes),
            node.path
        );
        if !node.flags.is_empty() {
            println!("   flags: {}", node.flags.join(", "));
        }
    }
}

fn print_findings(findings: &[Finding]) {
    println!("\n{BOLD}Findings{RESET}");
    if findings.is_empty() {
        println!("{GREEN}No findings for this scan.{RESET}");
        return;
    }

    for finding in findings {
        println!(
            "{} {} {}",
            risk_label(&finding.risk),
            finding.title,
            finding
                .size_bytes
                .map(format_bytes)
                .unwrap_or_else(|| "unknown size".to_string())
        );
        if let Some(path) = &finding.path {
            println!("   path: {path}");
        }
        println!("   reason: {}", finding.reason);
        println!("   action: {}", finding.recommended_action);
    }
}

fn risk_label(risk: &RiskLevel) -> String {
    match risk {
        RiskLevel::SafeToAnalyze => format!("{GREEN}🟢 safe{RESET}"),
        RiskLevel::Attention => format!("{YELLOW}🟡 attention{RESET}"),
        RiskLevel::ReviewRequired => format!("{ORANGE}🟠 review{RESET}"),
        RiskLevel::Dangerous => format!("{RED}🔴 dangerous{RESET}"),
        RiskLevel::ReadOnlySystem => format!("{GRAY}⚪ read-only/system{RESET}"),
    }
}

fn format_bytes_opt(bytes: Option<u64>) -> String {
    bytes
        .map(format_bytes)
        .unwrap_or_else(|| "unknown".to_string())
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}
