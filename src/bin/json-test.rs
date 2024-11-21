use std::process::ExitCode;

use json_data::Value;

// Run with https://github.com/nst/JSONTestSuite
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 2);

    let input = std::fs::read(&args[1]).unwrap();

    match Value::from_json(&input) {
        Ok(_) => 0.into(),
        Err(_) => 1.into(),
    }
}
