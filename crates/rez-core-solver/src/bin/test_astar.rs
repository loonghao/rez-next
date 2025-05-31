//! Test program for A* search framework
//!
//! This binary tests the A* search implementation without depending
//! on other potentially broken modules.

use rez_core_solver::astar::standalone_test::run_standalone_tests;

fn main() {
    println!("A* Search Framework Standalone Test");
    println!("===================================");

    match run_standalone_tests() {
        Ok(()) => {
            println!("✅ All tests completed successfully!");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("❌ Test failed: {}", e);
            std::process::exit(1);
        }
    }
}
