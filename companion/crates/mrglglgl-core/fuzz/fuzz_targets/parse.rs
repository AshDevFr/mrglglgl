//! Any input must parse or fail with an error: never panic, overflow the stack or hang.
//! Run with: cargo +nightly fuzz run parse -- -max_total_time=600
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = mrglglgl_core::sv::parse(&text);
});
