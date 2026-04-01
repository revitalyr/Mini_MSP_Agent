fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create sample data
    let agents = vec![
        ("agent-001", "active", "windows-pc", 86400, "windows"),
        ("agent-002", "inactive", "linux-server", 43200, "linux"),
    ];

    let metrics = vec![
        (1640995200, 25.5, 67.8, 45.2, 156),
        (1640995260, 30.1, 68.2, 45.3, 158),
    ];

    // Generate JSON report
    let mut json_content = String::new();
    json_content.push_str("{\n");
    json_content.push_str("  \"generated_at\": 1640995200,\n");
    json_content.push_str("  \"agent_id\": \"all\",\n");
    json_content.push_str("  \"time_range\": \"2024-01-01/2024-01-02\",\n");
    json_content.push_str("  \"agents\": [\n");
    
    for (i, (id, status, hostname, uptime, platform)) in agents.iter().enumerate() {
        json_content.push_str("    {\n");
        json_content.push_str(&format!("      \"id\": \"{}\",\n", id));
        json_content.push_str(&format!("      \"status\": \"{}\",\n", status));
        json_content.push_str(&format!("      \"hostname\": \"{}\",\n", hostname));
        json_content.push_str(&format!("      \"uptime\": {},\n", uptime));
        json_content.push_str("      \"last_heartbeat\": 1640995200,\n");
        json_content.push_str("      \"version\": \"1.0.0\",\n");
        json_content.push_str(&format!("      \"platform\": \"{}\"\n", platform));
        json_content.push_str("    }");
        if i < agents.len() - 1 {
            json_content.push_str(",");
        }
        json_content.push_str("\n");
    }
    
    json_content.push_str("  ],\n");
    json_content.push_str("  \"metrics\": [\n");
    
    for (i, (timestamp, cpu, ram, disk, processes)) in metrics.iter().enumerate() {
        json_content.push_str("    {\n");
        json_content.push_str(&format!("      \"timestamp\": {},\n", timestamp));
        json_content.push_str(&format!("      \"cpu_usage\": {},\n", cpu));
        json_content.push_str(&format!("      \"ram_usage\": {},\n", ram));
        json_content.push_str(&format!("      \"disk_usage\": {},\n", disk));
        json_content.push_str(&format!("      \"processes\": {}\n", processes));
        json_content.push_str("    }");
        if i < metrics.len() - 1 {
            json_content.push_str(",");
        }
        json_content.push_str("\n");
    }
    
    json_content.push_str("  ],\n");
    json_content.push_str("  \"summary\": {\n");
    json_content.push_str("    \"total_agents\": 2,\n");
    json_content.push_str("    \"active_agents\": 1,\n");
    json_content.push_str("    \"avg_cpu_usage\": 27.8,\n");
    json_content.push_str("    \"avg_ram_usage\": 68.0,\n");
    json_content.push_str("    \"avg_disk_usage\": 45.25,\n");
    json_content.push_str("    \"total_processes\": 314\n");
    json_content.push_str("  }\n");
    json_content.push_str("}\n");

    std::fs::write("report.json", json_content)?;
    println!("✅ JSON report generated: report.json");

    // Generate CSV report
    let mut csv_content = String::new();
    csv_content.push_str("timestamp,cpu_usage,ram_usage,disk_usage,processes\n");
    for (timestamp, cpu, ram, disk, processes) in &metrics {
        csv_content.push_str(&format!(
            "{},{},{},{},{}\n",
            timestamp, cpu, ram, disk, processes
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
        <p>Generated at: 2024-01-01 12:00:00 UTC</p>
        <p>Time Range: 2024-01-01/2024-01-02</p>
        <p>Platform: Windows</p>
    </div>
    
    <div class="summary">
        <h2>Summary</h2>
        <ul>
            <li>Total Agents: 2</li>
            <li>Active Agents: 1</li>
            <li>Avg CPU Usage: 27.8%</li>
            <li>Avg RAM Usage: 68.0%</li>
            <li>Avg Disk Usage: 45.25%</li>
            <li>Total Processes: 314</li>
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
    
    <div class="metrics">
        <h2>Plugin Status</h2>
        <table>
            <tr>
                <th>Name</th>
                <th>Version</th>
                <th>Status</th>
                <th>Capabilities</th>
            </tr>
            <tr>
                <td>windows_system_plugin</td>
                <td>1.0.0</td>
                <td>active</td>
                <td>system_metrics</td>
            </tr>
        </table>
    </div>
</body>
</html>"#,
        agents.iter().map(|(id, status, hostname, uptime, platform)| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}s</td><td>{}</td></tr>",
                id, status, hostname, uptime, platform
            )
        }).collect::<String>(),
        metrics.iter().map(|(timestamp, cpu, ram, disk, processes)| {
            format!(
                "<tr><td>{}</td><td>{}%</td><td>{}%</td><td>{}%</td><td>{}</td></tr>",
                timestamp, cpu, ram, disk, processes
            )
        }).collect::<String>()
    );

    std::fs::write("report.html", html_content)?;
    println!("✅ HTML report generated: report.html");

    println!("\n📊 Report Summary:");
    println!("  - Total Agents: 2");
    println!("  - Active Agents: 1");
    println!("  - Avg CPU Usage: 27.8%");
    println!("  - Avg RAM Usage: 68.0%");
    println!("  - Avg Disk Usage: 45.25%");
    println!("  - Total Processes: 314");
    println!("  - Platform: Windows");
    println!("  - Plugin: windows_system_plugin v1.0.0 (active)");

    Ok(())
}
