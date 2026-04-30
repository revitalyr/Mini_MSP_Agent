#!/usr/bin/env pwsh

param(
    [Parameter(Mandatory=$false)]
    [string]$TargetDir = "C:\Program Files\MSP Agent",
    
    [Parameter(Mandatory=$false)]
    [string]$ServiceName = "MSPAgent",
    
    [Parameter(Mandatory=$false)]
    [switch]$CreateService,
    
    [Parameter(Mandatory=$false)]
    [switch]$Verbose
)

Write-Host "Deploying Mini MSP Agent..." -ForegroundColor Green

# Check if running as administrator
$currentUser = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($currentUser)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "Error: This script must be run as Administrator" -ForegroundColor Red
    exit 1
}

Write-Host "Deployment configuration:" -ForegroundColor Cyan
Write-Host "  Target directory: $TargetDir"
Write-Host "  Service name: $ServiceName"

# Create target directory
Write-Host "Creating target directory..." -ForegroundColor Yellow
New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
New-Item -ItemType Directory -Path "$TargetDir\logs" -Force | Out-Null
New-Item -ItemType Directory -Path "$TargetDir\configs" -Force | Out-Null
New-Item -ItemType Directory -Path "$TargetDir\plugins" -Force | Out-Null

# Copy binary
Write-Host "Copying agent binary..." -ForegroundColor Yellow
$agentPath = "..\..\target\release\agent.exe"
if (Test-Path $agentPath) {
    Copy-Item $agentPath "$TargetDir\agent.exe" -Force
    Write-Host "Agent binary copied to $TargetDir\agent.exe" -ForegroundColor Green
} else {
    Write-Host "Error: Agent binary not found at $agentPath" -ForegroundColor Red
    Write-Host "Please run 'cargo build --release' first" -ForegroundColor Red
    exit 1
}

# Copy default configuration
Write-Host "Copying configuration files..." -ForegroundColor Yellow
$configPath = "$TargetDir\configs\config.toml"
if (-not (Test-Path $configPath)) {
    $configContent = @"
[agent]
id = `"default-agent`"
platform = `"windows`"
heartbeat_interval = 30
metrics_interval = 10

[broker]
url = `"nats://localhost:4222`"
client_id = `"msp-agent`"
max_reconnect_attempts = 5
reconnect_delay = 5000

[logging]
level = `"info`"
format = `"json`"
file = `"$TargetDir\logs\agent.log`"
max_file_size = 10485760
max_files = 5

[plugins]
enabled_plugins = [`"system_plugin`", `"file_plugin`", `"network_plugin"`]
plugin_dirs = [`"$TargetDir\plugins"`]
auto_reload = false
hot_reload = false

[security]
allowed_commands = [
    `"get_system_info`",
    `"get_processes`", 
    `"get_disk_info`",
    `"get_memory_info`",
    `"get_cpu_info`",
    `"get_network_info`",
    `"list_directory`",
    `"get_file_info`",
    `"read_file`",
    `"get_interfaces`",
    `"get_routes`",
    `"get_connections`"
]
max_file_size = 104857600
sandbox_enabled = false
require_signature = false
"@
    $configContent | Out-File -FilePath $configPath -Encoding UTF8
    Write-Host "Default configuration created at $configPath" -ForegroundColor Green
}

# Create Windows service if requested
if ($CreateService) {
    Write-Host "Creating Windows service..." -ForegroundColor Yellow
    
    $servicePath = "$TargetDir\agent.exe"
    $serviceName = $ServiceName
    $serviceDisplayName = "Mini MSP Agent"
    $serviceDescription = "Modular System Monitoring Agent"
    
    # Remove existing service if it exists
    $existingService = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    if ($existingService) {
        Write-Host "Removing existing service..." -ForegroundColor Yellow
        Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
        Remove-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
    }
    
    # Create new service
    Write-Host "Creating new service..." -ForegroundColor Yellow
    $serviceArgs = @(
        "-Name", $serviceName
        "-DisplayName", $serviceDisplayName
        "-Description", $serviceDescription
        "-BinaryPathName", $servicePath
        "-StartupType", "Automatic"
        "-DependsOn", "Tcpip"
        "-ErrorAction", "Continue"
        "-ErrorAction", "Restart"
        "-RestartDelay", 10000
    )
    
    New-Service @serviceArgs
    
    if ($?) {
        Write-Host "Service created successfully" -ForegroundColor Green
        
        # Start the service
        Write-Host "Starting service..." -ForegroundColor Yellow
        Start-Service -Name $serviceName
        
        Write-Host "Service started" -ForegroundColor Green
    } else {
        Write-Host "Failed to create service" -ForegroundColor Red
    }
}

# Set permissions
Write-Host "Setting permissions..." -ForegroundColor Yellow
# Windows doesn't need explicit permissions for executables

Write-Host ""
Write-Host "Deployment completed successfully!" -ForegroundColor Green
Write-Host ""

if ($CreateService) {
    Write-Host "Service management commands:" -ForegroundColor Cyan
    Write-Host "  Start:   Start-Service -Name $ServiceName"
    Write-Host "  Stop:    Stop-Service -Name $ServiceName"
    Write-Host "  Restart: Restart-Service -Name $ServiceName"
    Write-Host "  Status:  Get-Service -Name $ServiceName"
    Write-Host "  Logs:    Get-EventLog -LogName System -Source `"$ServiceName`" -Newest 100"
} else {
    Write-Host "To run as service:" -ForegroundColor Cyan
    Write-Host "  .\deploy.ps1 -CreateService"
}

Write-Host ""
Write-Host "Configuration file: $configPath"
Write-Host "Log directory: $TargetDir\logs"
Write-Host "Executable: $TargetDir\agent.exe"
