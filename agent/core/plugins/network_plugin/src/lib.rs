use anyhow::Result;
use chrono::Utc;
use core_shared::{Plugin, PluginInfo, PluginStatus, NetworkInfo, NetworkInterface, RouteInfo, ConnectionInfo};
use nix::sys::net::{if_nameindex, if_nametoindex};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;
use tracing::{info, warn, error};
use uuid::Uuid;

pub struct NetworkPlugin {
    info: PluginInfo,
}

impl NetworkPlugin {
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "network_plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "Network interface and connection monitoring plugin".to_string(),
                author: "MSP Agent Team".to_string(),
                status: PluginStatus::Unloaded,
                loaded_at: None,
                last_error: None,
            },
        }
    }

    fn get_network_interfaces(&self) -> Result<Vec<NetworkInterface>> {
        let mut interfaces = Vec::new();
        
        // Get network interfaces from /proc/net/dev (Linux)
        if cfg!(target_os = "linux") {
            if let Ok(content) = fs::read_to_string("/proc/net/dev") {
                for line in content.lines().skip(2) {
                    if let Some(interface) = self.parse_proc_net_dev_line(line) {
                        interfaces.push(interface);
                    }
                }
            }
        }
        
        // Get additional interface information
        if let Ok(if_index_map) = if_nameindex() {
            for (index, name) in if_index_map {
                if let Some(existing) = interfaces.iter_mut().find(|i| i.name == name.to_string_lossy()) {
                    existing.index = index as u32;
                }
            }
        }
        
        Ok(interfaces)
    }

    fn parse_proc_net_dev_line(&self, line: &str) -> Option<NetworkInterface> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 17 {
            return None;
        }
        
        let interface_name = parts[0].trim_end_matches(':');
        
        let rx_bytes = parts[1].parse::<u64>().ok()?;
        let rx_packets = parts[2].parse::<u64>().ok()?;
        let rx_errors = parts[3].parse::<u64>().ok()?;
        
        let tx_bytes = parts[9].parse::<u64>().ok()?;
        let tx_packets = parts[10].parse::<u64>().ok()?;
        let tx_errors = parts[11].parse::<u64>().ok()?;
        
        Some(NetworkInterface {
            name: interface_name.to_string(),
            index: 0,
            mtu: 1500, // Default MTU
            is_up: true,
            is_loopback: interface_name == "lo",
            mac_address: None,
            ipv4_addresses: Vec::new(),
            ipv6_addresses: Vec::new(),
            bytes_received: rx_bytes,
            bytes_sent: tx_bytes,
            packets_received: rx_packets,
            packets_sent: tx_packets,
            errors_in: rx_errors,
            errors_out: tx_errors,
        })
    }

    fn get_network_routes(&self) -> Result<Vec<RouteInfo>> {
        let mut routes = Vec::new();
        
        if cfg!(target_os = "linux") {
            if let Ok(content) = fs::read_to_string("/proc/net/route") {
                for line in content.lines() {
                    if let Some(route) = self.parse_proc_route_line(line) {
                        routes.push(route);
                    }
                }
            }
        }
        
        Ok(routes)
    }

    fn parse_proc_route_line(&self, line: &str) -> Option<RouteInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            return None;
        }
        
        let destination = self.parse_hex_ip(parts[0])?;
        let gateway = if parts[1] != "00000000" {
            Some(self.parse_hex_ip(parts[1])?)
        } else {
            None
        };
        
        let interface = parts[6].to_string();
        let is_default = destination == "0.0.0.0";
        
        Some(RouteInfo {
            destination,
            gateway,
            interface,
            metric: Some(parts[2].parse::<u32>().ok()?),
            is_default,
        })
    }

    fn parse_hex_ip(&self, hex: &str) -> Option<String> {
        if hex.len() != 8 {
            return None;
        }
        
        let mut ip_bytes = [0u8; 4];
        for (i, chunk) in hex.chunks(2).enumerate() {
            ip_bytes[i] = u8::from_str_radix(chunk, 16).ok()?;
        }
        
        Some(format!("{}.{}.{}.{}", ip_bytes[3], ip_bytes[2], ip_bytes[1], ip_bytes[0]))
    }

    fn get_network_connections(&self) -> Result<Vec<ConnectionInfo>> {
        let mut connections = Vec::new();
        
        if cfg!(target_os = "linux") {
            // TCP connections
            if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
                for line in content.lines().skip(1) {
                    if let Some(conn) = self.parse_proc_net_line(line, "tcp") {
                        connections.push(conn);
                    }
                }
            }
            
            // UDP connections
            if let Ok(content) = fs::read_to_string("/proc/net/udp") {
                for line in content.lines().skip(1) {
                    if let Some(conn) = self.parse_proc_net_line(line, "udp") {
                        connections.push(conn);
                    }
                }
            }
        }
        
        Ok(connections)
    }

    fn parse_proc_net_line(&self, line: &str, protocol: &str) -> Option<ConnectionInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }
        
        let local_addr = self.parse_proc_net_addr(parts[1])?;
        let remote_addr = parts[2];
        let state = parts[3];
        
        let remote = if remote_addr != "00000000:0000" {
            Some(self.parse_proc_net_addr(remote_addr)?)
        } else {
            None
        };
        
        Some(ConnectionInfo {
            protocol: protocol.to_string(),
            local_address: local_addr,
            remote_address: remote,
            state: self.parse_tcp_state(state)?,
            pid: None,
            process_name: None,
        })
    }

    fn parse_proc_net_addr(&self, addr: &str) -> Option<String> {
        let parts: Vec<&str> = addr.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        
        let ip_hex = parts[0];
        let port_hex = parts[1];
        
        if ip_hex.len() != 8 || port_hex.len() != 4 {
            return None;
        }
        
        let mut ip_bytes = [0u8; 4];
        for (i, chunk) in ip_hex.chunks(2).enumerate() {
            ip_bytes[i] = u8::from_str_radix(chunk, 16).ok()?;
        }
        
        let port = u16::from_str_radix(port_hex, 16).ok()?;
        
        Some(format!("{}.{}.{}.{}:{}", ip_bytes[3], ip_bytes[2], ip_bytes[1], ip_bytes[0], port))
    }

    fn parse_tcp_state(&self, state: &str) -> Option<String> {
        let state_num = state.parse::<u8>().ok()?;
        
        let state_str = match state_num {
            0x01 => "ESTABLISHED",
            0x02 => "SYN_SENT",
            0x03 => "SYN_RECV",
            0x04 => "FIN_WAIT1",
            0x05 => "FIN_WAIT2",
            0x06 => "TIME_WAIT",
            0x07 => "CLOSE",
            0x08 => "CLOSE_WAIT",
            0x09 => "LAST_ACK",
            0x0A => "LISTEN",
            0x0B => "CLOSING",
            _ => "UNKNOWN",
        };
        
        Some(state_str.to_string())
    }

    async fn ping_host(&self, host: &str, count: u32) -> Result<Value> {
        use tokio::process::Command;
        
        let output = Command::new("ping")
            .args(&["-c", &count.to_string(), host])
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Ping failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse ping results
        let mut transmitted = 0;
        let mut received = 0;
        let mut min_time = f64::MAX;
        let mut max_time = 0.0;
        let mut total_time = 0.0;
        
        for line in stdout.lines() {
            if line.contains("packets transmitted") {
                if let Some(stats) = line.split(',').nth(2) {
                    if let Some(received_str) = stats.split_whitespace().nth(0) {
                        received = received_str.parse::<u32>().unwrap_or(0);
                    }
                }
                if let Some(stats) = line.split(',').nth(0) {
                    if let Some(transmitted_str) = stats.split_whitespace().nth(0) {
                        transmitted = transmitted_str.parse::<u32>().unwrap_or(0);
                    }
                }
            }
            
            if line.contains("min/avg/max") {
                if let Some(times) = line.split('=').nth(1) {
                    let time_parts: Vec<&str> = times.trim().split('/').collect();
                    if time_parts.len() >= 3 {
                        min_time = time_parts[0].parse::<f64>().unwrap_or(0.0);
                        total_time = time_parts[1].parse::<f64>().unwrap_or(0.0);
                        max_time = time_parts[2].parse::<f64>().unwrap_or(0.0);
                    }
                }
            }
        }
        
        let avg_time = if received > 0 { total_time / received as f64 } else { 0.0 };
        let loss_percent = if transmitted > 0 {
            ((transmitted - received) as f64 / transmitted as f64) * 100.0
        } else {
            0.0
        };
        
        Ok(json!({
            "host": host,
            "packets_transmitted": transmitted,
            "packets_received": received,
            "packet_loss_percent": loss_percent,
            "min_time_ms": min_time,
            "max_time_ms": max_time,
            "avg_time_ms": avg_time,
            "success": received > 0,
        }))
    }
}

impl Plugin for NetworkPlugin {
    fn name(&self) -> &str {
        &self.info.name
    }

    fn version(&self) -> &str {
        &self.info.version
    }

    fn description(&self) -> &str {
        &self.info.description
    }

    async fn initialize(&mut self) -> Result<()> {
        info!("Initializing network plugin");
        
        self.info.status = PluginStatus::Loaded;
        self.info.loaded_at = Some(Utc::now());
        
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down network plugin");
        self.info.status = PluginStatus::Unloaded;
        self.info.loaded_at = None;
        Ok(())
    }

    async fn handle_command(&self, command: &str, params: HashMap<String, Value>) -> Result<Value> {
        match command {
            "get_interfaces" => self.handle_get_interfaces(params).await,
            "get_routes" => self.handle_get_routes(params).await,
            "get_connections" => self.handle_get_connections(params).await,
            "ping" => self.handle_ping(params).await,
            "traceroute" => self.handle_traceroute(params).await,
            "nslookup" => self.handle_nslookup(params).await,
            "get_dns_servers" => self.handle_get_dns_servers(params).await,
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }

    async fn get_metrics(&self) -> Result<core_shared::SystemMetrics> {
        // This plugin doesn't provide system metrics
        Err(anyhow::anyhow!("Network plugin doesn't provide system metrics"))
    }
}

impl NetworkPlugin {
    async fn handle_get_interfaces(&self, _params: HashMap<String, Value>) -> Result<Value> {
        let interfaces = self.get_network_interfaces()?;
        
        Ok(json!({
            "interfaces": interfaces,
            "count": interfaces.len(),
        }))
    }

    async fn handle_get_routes(&self, _params: HashMap<String, Value>) -> Result<Value> {
        let routes = self.get_network_routes()?;
        
        Ok(json!({
            "routes": routes,
            "count": routes.len(),
        }))
    }

    async fn handle_get_connections(&self, _params: HashMap<String, Value>) -> Result<Value> {
        let connections = self.get_network_connections()?;
        
        Ok(json!({
            "connections": connections,
            "count": connections.len(),
        }))
    }

    async fn handle_ping(&self, params: HashMap<String, Value>) -> Result<Value> {
        let host = params.get("host")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing host parameter"))?;
        
        let count = params.get("count")
            .and_then(|c| c.as_u64())
            .unwrap_or(4) as u32;
        
        self.ping_host(host, count).await
    }

    async fn handle_traceroute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let host = params.get("host")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing host parameter"))?;
        
        use tokio::process::Command;
        
        let output = Command::new("traceroute")
            .arg(host)
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Traceroute failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let hops: Vec<String> = stdout.lines().map(|line| line.to_string()).collect();
        
        Ok(json!({
            "host": host,
            "hops": hops,
            "success": true,
        }))
    }

    async fn handle_nslookup(&self, params: HashMap<String, Value>) -> Result<Value> {
        let hostname = params.get("hostname")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing hostname parameter"))?;
        
        use tokio::process::Command;
        
        let output = Command::new("nslookup")
            .arg(hostname)
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("NSLookup failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        Ok(json!({
            "hostname": hostname,
            "result": stdout,
            "success": true,
        }))
    }

    async fn handle_get_dns_servers(&self, _params: HashMap<String, Value>) -> Result<Value> {
        let mut dns_servers = Vec::new();
        
        if cfg!(target_os = "linux") {
            if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
                for line in content.lines() {
                    if line.starts_with("nameserver") {
                        if let Some(server) = line.split_whitespace().nth(1) {
                            dns_servers.push(server.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(json!({
            "dns_servers": dns_servers,
            "count": dns_servers.len(),
        }))
    }
}

// Factory function for plugin loading
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut Box<dyn Plugin> {
    let plugin = Box::new(NetworkPlugin::new());
    Box::into_raw(Box::new(plugin))
}

// Required for dynamic loading
#[no_mangle]
pub extern "C" fn get_plugin_info() -> PluginInfo {
    PluginInfo {
        name: "network_plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Network interface and connection monitoring plugin".to_string(),
        author: "MSP Agent Team".to_string(),
        status: PluginStatus::Unloaded,
        loaded_at: None,
        last_error: None,
    }
}
