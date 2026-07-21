#![no_main]

use libfuzzer_sys::fuzz_target;
use dew_core::Expr;

fuzz_target!(|data: &str| {
    // Parser should never panic on any input
    let _ = Expr::parse(data);
});
