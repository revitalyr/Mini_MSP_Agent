mod os_reporter;

use os_reporter::{OSReporter, OSReport};
use std::env;
use serde_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥️  Mini MSP Agent - OS Information Reporter");
    println!("============================================");
    println!();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    let output_format = if args.len() > 1 {
        match args[1].as_str() {
            "json" => "json",
            "table" => "table",
            "csv" => "csv",
            _ => {
                eprintln!("Usage: {} [json|table|csv]", args[0]);
                eprintln!("  json - Output as JSON");
                eprintln!("  table - Output as formatted table (default)");
                eprintln!("  csv - Output as CSV");
                std::process::exit(1);
            }
        }
    } else {
        "table"
    };

    println!("📊 Generating OS information report...");
    println!();

    // Generate the OS report
    let report = match OSReporter::generate_report() {
        Ok(report) => {
            println!("✅ Report generated successfully!");
            println!();
            report
        }
        Err(e) => {
            eprintln!("❌ Error generating report: {}", e);
            std::process::exit(1);
        }
    };

    // Output the report in the requested format
    match output_format {
        "json" => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{}", json);
        }
        "table" => {
            let formatted = OSReporter::format_report(&report);
            println!("{}", formatted);
        }
        "csv" => {
            output_csv(&report);
        }
        _ => unreachable!(),
    }

    println!();
    println!("🎉 Report generation complete!");
    
    Ok(())
}

fn output_csv(report: &OSReport) {
    println!("OS Information Report (CSV Format)");
    println!("==================================");
    
    // System Information
    println!();
    println!("[System Information]");
    println!("Hostname,OS Type,OS Version,Architecture,Uptime,Current User");
    println!("{},{},{},{},{},{}", 
        report.system_info.hostname,
        report.system_info.os_type,
        report.system_info.os_version,
        report.system_info.architecture,
        report.system_info.uptime,
        report.system_info.current_user
    );

    // Hardware Information
    println!();
    println!("[Hardware Information]");
    println!("CPU Model,Cores,Logical Processors,Total Memory (MB),Available Memory (MB)");
    println!("{},{},{},{},{}", 
        report.hardware_info.cpu_info.model,
        report.hardware_info.cpu_info.cores,
        report.hardware_info.cpu_info.logical_processors,
        report.hardware_info.memory_total,
        report.hardware_info.memory_available
    );

    // Network Information
    println!();
    println!("[Network Interfaces]");
    println!("Interface Name,IP Address,MAC Address,Is Up,Speed");
    for interface in &report.network_info.interfaces {
        println!("{},{},{},{},{}", 
            interface.name,
            interface.ip_address,
            interface.mac_address,
            interface.is_up,
            interface.speed.unwrap_or(0)
        );
    }

    // Process Information
    println!();
    println!("[Process Information]");
    println!("PID,Name,CPU %,Memory (MB),Status,Start Time");
    for process in &report.process_info {
        println!("{},{},{},{},{},{}", 
            process.pid,
            process.name,
            process.cpu_percent,
            process.memory_mb,
            process.status,
            process.start_time
        );
    }

    // Disk Information
    println!();
    println!("[Disk Information]");
    println!("Mount Point,Total Space (GB),Free Space (GB),Used Space (GB),Filesystem Type");
    for disk in &report.disk_info {
        println!("{},{},{},{},{}", 
            disk.mount_point,
            disk.total_space / 1073741824,
            disk.free_space / 1073741824,
            disk.used_space / 1073741824,
            disk.filesystem_type
        );
    }

    // Memory Information
    println!();
    println!("[Memory Information]");
    println!("Total (MB),Used (MB),Available (MB),Swap Total (MB),Swap Free (MB)");
    println!("{},{},{},{},{}", 
        report.memory_info.total_mb,
        report.memory_info.used_mb,
        report.memory_info.available_mb,
        report.memory_info.swap_total_mb,
        report.memory_info.swap_free_mb
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_function() {
        // Test that main function doesn't panic on basic usage
        // In a real test, we would need to mock the system calls
    }
}
