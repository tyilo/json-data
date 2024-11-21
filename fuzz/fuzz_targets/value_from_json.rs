#![no_main]

use json_data::Value;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    _ = Value::from_json(data);
});
