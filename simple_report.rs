use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct Agent {
    id: String,
    status: String,
    hostname: String,
    uptime: u64,
    last_heartbeat: u64,
    version: String,
    platform: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Metric {
    timestamp: u64,
    cpu_usage: f32,
    ram_usage: f32,
    disk_usage: f32,
    processes: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Report {
    generated_at: u64,
    agent_id: String,
    time_range: String,
    agents: Vec<Agent>,
    metrics: Vec<Metric>,
    summary: HashMap<String, serde_json::Value>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create sample data
    let agents = vec![
        Agent {
            id: "agent-001".to_string(),
            status: "active".to_string(),
            hostname: "windows-pc".to_string(),
            uptime: 86400,
            last_heartbeat: 1640995200,
            version: "1.0.0".to_string(),
            platform: "windows".to_string(),
        },
        Agent {
            id: "agent-002".to_string(),
            status: "inactive".to_string(),
            hostname: "linux-server".to_string(),
            uptime: 43200,
            last_heartbeat: 1640995000,
            version: "1.0.0".to_string(),
            platform: "linux".to_string(),
        },
    ];

    let metrics = vec![
        Metric {
            timestamp: 1640995200,
            cpu_usage: 25.5,
            ram_usage: 67.8,
            disk_usage: 45.2,
            processes: 156,
        },
        Metric {
            timestamp: 1640995260,
            cpu_usage: 30.1,
            ram_usage: 68.2,
            disk_usage: 45.3,
            processes: 158,
        },
    ];

    let mut summary = HashMap::new();
    summary.insert("total_agents".to_string(), serde_json::Value::Number(2.into()));
    summary.insert("active_agents".to_string(), serde_json::Value::Number(1.into()));
    summary.insert("avg_cpu_usage".to_string(), serde_json::Value::Number(27.8.into()));
    summary.insert("avg_ram_usage".to_string(), serde_json::Value::Number(68.0.into()));
    summary.insert("avg_disk_usage".to_string(), serde_json::Value::Number(45.25.into()));
    summary.insert("total_processes".to_string(), serde_json::Value::Number(314.into()));

    let report = Report {
        generated_at: chrono::Utc::now().timestamp() as u64,
        agent_id: "all".to_string(),
        time_range: "2024-01-01/2024-01-02".to_string(),
        agents,
        metrics,
        summary,
    };

    // Generate JSON report
    let json_report = serde_json::to_string_pretty(&report)?;
    std::fs::write("report.json", json_report)?;
    println!("✅ JSON report generated: report.json");

    // Generate CSV report
    let mut csv_content = String::new();
    csv_content.push_str("timestamp,cpu_usage,ram_usage,disk_usage,processes\n");
    for metric in &report.metrics {
        csv_content.push_str(&format!(
            "{},{},{},{},{}\n",
            metric.timestamp, metric.cpu_usage, metric.ram_usage, metric.disk_usage, metric.processes
        ));
    }
    std::fs::write("report.csv", csv_content)?;
    println!("✅ CSV report generated: report.csv");

    // Generate HTML report
    let html_content = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Mini MSP Agent Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f4f4f4; padding: 20px; border-radius: 5px; }}
        .metrics {{ margin: 20px 0; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
        .summary {{ background: #e8f5e8; padding: 15px; border-radius: 5px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Mini MSP Agent Report</h1>
        <p>Generated at: {}</p>
        <p>Time Range: {}</p>
    </div>
    
    <div class="summary">
        <h2>Summary</h2>
        <ul>
            <li>Total Agents: {}</li>
            <li>Active Agents: {}</li>
            <li>Avg CPU Usage: {}%</li>
            <li>Avg RAM Usage: {}%</li>
            <li>Avg Disk Usage: {}%</li>
            <li>Total Processes: {}</li>
        </ul>
    </div>
    
    <div class="metrics">
        <h2>Agents</h2>
        <table>
            <tr>
                <th>ID</th>
                <th>Status</th>
                <th>Hostname</th>
                <th>Uptime</th>
                <th>Platform</th>
            </tr>
            {}
        </table>
    </div>
    
    <div class="metrics">
        <h2>Metrics</h2>
        <table>
            <tr>
                <th>Timestamp</th>
                <th>CPU Usage</th>
                <th>RAM Usage</th>
                <th>Disk Usage</th>
                <th>Processes</th>
            </tr>
            {}
        </table>
    </div>
</body>
</html>"#,
        chrono::DateTime::from_timestamp(report.generated_at as i64, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S UTC"),
        report.time_range,
        report.summary.get("total_agents").unwrap(),
        report.summary.get("active_agents").unwrap(),
        report.summary.get("avg_cpu_usage").unwrap(),
        report.summary.get("avg_ram_usage").unwrap(),
        report.summary.get("avg_disk_usage").unwrap(),
        report.summary.get("total_processes").unwrap(),
        report.agents.iter().map(|a| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}s</td><td>{}</td></tr>",
                a.id, a.status, a.hostname, a.uptime, a.platform
            )
        }).collect::<String>(),
        report.metrics.iter().map(|m| {
            format!(
                "<tr><td>{}</td><td>{}%</td><td>{}%</td><td>{}%</td><td>{}</td></tr>",
                m.timestamp, m.cpu_usage, m.ram_usage, m.disk_usage, m.processes
            )
        }).collect::<String>()
    );

    std::fs::write("report.html", html_content)?;
    println!("✅ HTML report generated: report.html");

    println!("\n📊 Report Summary:");
    println!("  - Total Agents: {}", report.summary.get("total_agents").unwrap());
    println!("  - Active Agents: {}", report.summary.get("active_agents").unwrap());
    println!("  - Avg CPU Usage: {}%", report.summary.get("avg_cpu_usage").unwrap());
    println!("  - Avg RAM Usage: {}%", report.summary.get("avg_ram_usage").unwrap());
    println!("  - Avg Disk Usage: {}%", report.summary.get("avg_disk_usage").unwrap());
    println!("  - Total Processes: {}", report.summary.get("total_processes").unwrap());

    Ok(())
}
