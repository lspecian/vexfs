//! Standalone vector test runner for VexFS
//! 
//! This binary tests the vector search functionality without requiring kernel compilation.

use std::env;
use std::process;

mod vector_test {
    include!("../vector_test.rs");
}

use vector_test::{run_performance_test, run_functional_test};

fn main() {
    println!("VexFS Vector Operations Test Runner");
    println!("===================================");
    
    let args: Vec<String> = env::args().collect();
    
    let test_type = if args.len() > 1 {
        &args[1]
    } else {
        "all"
    };
    
    match test_type {
        "functional" => {
            if let Err(e) = run_functional_test() {
                eprintln!("Functional test failed: {}", e);
                process::exit(1);
            }
        }
        "performance" => {
            if let Err(e) = run_performance_test() {
                eprintln!("Performance test failed: {}", e);
                process::exit(1);
            }
        }
        "all" => {
            println!("Running all tests...\n");
            
            if let Err(e) = run_functional_test() {
                eprintln!("Functional test failed: {}", e);
                process::exit(1);
            }
            
            println!("\n{}\n", "=".repeat(50));
            
            if let Err(e) = run_performance_test() {
                eprintln!("Performance test failed: {}", e);
                process::exit(1);
            }
        }
        _ => {
            println!("Usage: {} [functional|performance|all]", args[0]);
            println!("  functional  - Run functional tests only");
            println!("  performance - Run performance tests only");
            println!("  all         - Run all tests (default)");
            process::exit(1);
        }
    }
    
    println!("\nAll tests completed successfully! ✅");
}