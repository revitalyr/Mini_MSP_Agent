//! Test runner for Mini MSP Agent
//! 
//! Runs all unit and integration tests with proper setup/teardown

use std::process::{Command, Stdio};
use std::env;
use anyhow::Result;

/// Test configuration
const NATS_DOCKER_IMAGE: &str = "nats:2.10-alpine";
const NATS_CONTAINER_NAME: &str = "mini-msp-nats-test";
const NATS_PORT: u16 = 4223; // Use different port to avoid conflicts

/// Main test runner
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Mini MSP Agent Test Runner");
    println!("================================");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }
    
    match args[1].as_str() {
        "unit" => run_unit_tests().await?,
        "integration" => run_integration_tests().await?,
        "all" => {
            run_unit_tests().await?;
            run_integration_tests().await?;
        }
        "setup" => setup_test_environment().await?,
        "cleanup" => cleanup_test_environment().await?,
        _ => {
            eprintln!("❌ Unknown command: {}", args[1]);
            print_usage();
        }
    }
    
    Ok(())
}

/// Print usage information
fn print_usage() {
    println!("Usage: cargo run --bin test-runner <command>");
    println!("");
    println!("Commands:");
    println!("  unit        - Run unit tests (no NATS required)");
    println!("  integration - Run integration tests (NATS required)");
    println!("  all         - Run all tests");
    println!("  setup       - Setup test environment (start NATS)");
    println!("  cleanup     - Cleanup test environment (stop NATS)");
    println!("");
    println!("Examples:");
    println!("  cargo run --bin test-runner unit");
    println!("  cargo run --bin test-runner integration");
    println!("  cargo run --bin test-runner setup");
    println!("  cargo run --bin test-runner cleanup");
}

/// Run unit tests
async fn run_unit_tests() -> Result<()> {
    println!("📋 Running unit tests...");
    println!("================================");
    
    let output = Command::new("cargo")
        .args(&["test", "--lib", "--test", "broker_unit_tests"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    
    if output.status.success() {
        println!("✅ Unit tests passed!");
    } else {
        eprintln!("❌ Unit tests failed!");
        std::process::exit(1);
    }
    
    Ok(())
}

/// Run integration tests
async fn run_integration_tests() -> Result<()> {
    println!("🔗 Running integration tests...");
    println!("================================");
    
    // Ensure NATS is running
    if !is_nats_running().await {
        println!("🐳 Starting NATS for integration tests...");
        setup_test_environment().await?;
    }
    
    // Wait a bit for NATS to be ready
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Set NATS URL environment variable
    env::set_var("NATS_URL", "nats://localhost:4223");
    
    let output = Command::new("cargo")
        .args(&["test", "--test", "broker_integration_tests"])
        .env("NATS_URL", "nats://localhost:4223")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    
    if output.status.success() {
        println!("✅ Integration tests passed!");
    } else {
        eprintln!("❌ Integration tests failed!");
        std::process::exit(1);
    }
    
    Ok(())
}

/// Setup test environment
async fn setup_test_environment() -> Result<()> {
    println!("🔧 Setting up test environment...");
    
    // Stop existing container if running
    cleanup_test_environment().await?;
    
    // Start NATS container
    let output = Command::new("docker")
        .args(&[
            "run", "-d",
            "--name", NATS_CONTAINER_NAME,
            "-p", &format!("{}:4222", NATS_PORT),
            "-p", "8223:8222", // HTTP monitoring on different port
            NATS_DOCKER_IMAGE,
            "--jetstream"
        ])
        .output()?;
    
    if !output.status.success() {
        eprintln!("❌ Failed to start NATS container");
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err(anyhow::anyhow!("Failed to start NATS"));
    }
    
    println!("✅ NATS container started on port {}", NATS_PORT);
    println!("📊 NATS monitoring: http://localhost:8223");
    
    // Wait for NATS to be ready
    println!("⏳ Waiting for NATS to be ready...");
    for i in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        if is_nats_ready().await {
            println!("✅ NATS is ready!");
            return Ok(());
        }
        
        if i % 5 == 0 {
            println!("   Still waiting... ({}s)", i + 1);
        }
    }
    
    eprintln!("❌ NATS failed to start within 30 seconds");
    cleanup_test_environment().await?;
    Err(anyhow::anyhow!("NATS startup timeout"))
}

/// Cleanup test environment
async fn cleanup_test_environment() -> Result<()> {
    println!("🧹 Cleaning up test environment...");
    
    // Stop and remove container
    let output = Command::new("docker")
        .args(&["rm", "-f", NATS_CONTAINER_NAME])
        .output()?;
    
    // Don't error if container doesn't exist
    if output.status.success() {
        println!("✅ NATS container stopped and removed");
    } else {
        println!("ℹ️  NATS container was not running");
    }
    
    Ok(())
}

/// Check if NATS is running
async fn is_nats_running() -> bool {
    let output = Command::new("docker")
        .args(&["ps", "--filter", &format!("name={}", NATS_CONTAINER_NAME)])
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            stdout.contains(NATS_CONTAINER_NAME)
        }
        Err(_) => false,
    }
}

/// Check if NATS is ready to accept connections
async fn is_nats_ready() -> bool {
    use tokio::net::TcpStream;
    
    match TcpStream::connect(("localhost", NATS_PORT)).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Run module tests
async fn run_module_tests() -> Result<()> {
    println!("🧩 Running module tests...");
    println!("================================");
    
    let output = Command::new("cargo")
        .args(&["test", "--test", "broker_module_tests"])
        .env("NATS_URL", "nats://localhost:4223")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    
    if output.status.success() {
        println!("✅ Module tests passed!");
    } else {
        eprintln!("❌ Module tests failed!");
        std::process::exit(1);
    }
    
    Ok(())
}

/// Run performance tests
async fn run_performance_tests() -> Result<()> {
    println!("⚡ Running performance tests...");
    println!("================================");
    
    let output = Command::new("cargo")
        .args(&["test", "--test", "broker_integration_tests", "--", "performance"])
        .env("NATS_URL", "nats://localhost:4223")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    
    if output.status.success() {
        println!("✅ Performance tests passed!");
    } else {
        eprintln!("❌ Performance tests failed!");
        std::process::exit(1);
    }
    
    Ok(())
}

/// Run all tests including module and performance
async fn run_all_tests() -> Result<()> {
    println!("🎯 Running ALL tests...");
    println!("================================");
    
    // Run unit tests first
    run_unit_tests().await?;
    
    // Setup NATS for integration tests
    if !is_nats_running().await {
        setup_test_environment().await?;
    }
    
    // Wait for NATS
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Run module tests
    run_module_tests().await?;
    
    // Run integration tests
    run_integration_tests().await?;
    
    // Run performance tests
    run_performance_tests().await?;
    
    println!("🎉 ALL TESTS PASSED!");
    
    Ok(())
}
