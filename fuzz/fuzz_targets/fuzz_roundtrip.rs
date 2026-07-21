#![no_main]

use libfuzzer_sys::fuzz_target;
use dew_core::Expr;

fuzz_target!(|data: &str| {
    // Only test expressions that parse successfully
    let Ok(expr1) = Expr::parse(data) else {
        return;
    };

    // Stringify the AST
    let stringified = expr1.ast().to_string();

    // Parse the stringified version
    let Ok(expr2) = Expr::parse(&stringified) else {
        panic!(
            "Failed to parse stringified AST!\nOriginal: {}\nStringified: {}\nAST: {:?}",
            data, stringified, expr1.ast()
        );
    };

    // Compare ASTs (but handle NaN specially since NaN != NaN)
    // For simplicity, we compare the re-stringified versions
    let stringified2 = expr2.ast().to_string();
    if stringified != stringified2 {
        panic!(
            "Roundtrip mismatch!\nOriginal: {}\nFirst stringify: {}\nSecond stringify: {}",
            data, stringified, stringified2
        );
    }
});
