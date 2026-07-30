// Tests for `binary_op_to_str` — the operator mapping function
// that translates OmniLang's internal operator names into target
// language symbols.

use crate::codegen::binary_op_to_str;
use crate::codegen::sanitize::sanitize_css_value;
use crate::ir::BinaryOpIR;

#[test]
fn binary_op_to_str_all_variants_non_empty() {
    // Verify every BinaryOpIR variant produces a non-empty string
    use BinaryOpIR::*;
    for op in &[
        Add,
        Sub,
        Mul,
        Div,
        Mod,
        Eq,
        Neq,
        Lt,
        Gt,
        LtEq,
        GtEq,
        And,
        Or,
        NullishCoalesce,
        Assign,
        AddAssign,
        SubAssign,
        MulAssign,
        DivAssign,
        ModAssign,
    ] {
        let result = binary_op_to_str(op, false, false);
        assert!(!result.is_empty(), "{op:?} produced empty string");
        let result_strict = binary_op_to_str(op, true, false);
        assert!(
            !result_strict.is_empty(),
            "{op:?} (strict) produced empty string"
        );
        let result_textual = binary_op_to_str(op, false, true);
        assert!(
            !result_textual.is_empty(),
            "{op:?} (textual) produced empty string"
        );
    }
}

#[test]
fn binary_op_to_str_arithmetic() {
    use BinaryOpIR::*;
    for (op, expected) in &[(Add, "+"), (Sub, "-"), (Mul, "*"), (Div, "/"), (Mod, "%")] {
        assert_eq!(
            binary_op_to_str(op, false, false),
            *expected,
            "arithmetic op {op:?}"
        );
        assert_eq!(
            binary_op_to_str(op, true, false),
            *expected,
            "arithmetic op {op:?} (strict)"
        );
        assert_eq!(
            binary_op_to_str(op, false, true),
            *expected,
            "arithmetic op {op:?} (textual)"
        );
    }
}

#[test]
fn binary_op_to_str_comparison() {
    use BinaryOpIR::*;
    assert_eq!(binary_op_to_str(&Eq, false, false), "==");
    assert_eq!(binary_op_to_str(&Eq, true, false), "===");
    assert_eq!(binary_op_to_str(&Neq, false, false), "!=");
    assert_eq!(binary_op_to_str(&Neq, true, false), "!==");
    assert_eq!(binary_op_to_str(&Lt, false, false), "<");
    assert_eq!(binary_op_to_str(&Gt, false, false), ">");
    assert_eq!(binary_op_to_str(&LtEq, false, false), "<=");
    assert_eq!(binary_op_to_str(&GtEq, false, false), ">=");
    // Strict mode doesn't affect <, >, <=, >=
    assert_eq!(binary_op_to_str(&Lt, true, false), "<");
    assert_eq!(binary_op_to_str(&Gt, true, false), ">");
    assert_eq!(binary_op_to_str(&LtEq, true, false), "<=");
    assert_eq!(binary_op_to_str(&GtEq, true, false), ">=");
}

#[test]
fn binary_op_to_str_logical() {
    use BinaryOpIR::*;
    assert_eq!(binary_op_to_str(&And, false, false), "&&");
    assert_eq!(binary_op_to_str(&And, false, true), "and");
    assert_eq!(binary_op_to_str(&Or, false, false), "||");
    assert_eq!(binary_op_to_str(&Or, false, true), "or");
    assert_eq!(binary_op_to_str(&NullishCoalesce, false, false), "??");
}

#[test]
fn binary_op_to_str_assignment() {
    use BinaryOpIR::*;
    assert_eq!(binary_op_to_str(&Assign, false, false), "=");
    assert_eq!(binary_op_to_str(&AddAssign, false, false), "+=");
    assert_eq!(binary_op_to_str(&SubAssign, false, false), "-=");
    assert_eq!(binary_op_to_str(&MulAssign, false, false), "*=");
    assert_eq!(binary_op_to_str(&DivAssign, false, false), "/=");
    assert_eq!(binary_op_to_str(&ModAssign, false, false), "%=");
}

#[test]
fn sanitize_css_value_blocks_crlf_javascript_bypass() {
    // CR/LF before "javascript:" should be stripped and caught
    assert_eq!(
        sanitize_css_value("url(jav\r\nascript:alert(1))"),
        "",
        "CR/LF should be stripped and javascript: caught"
    );
    // TAB before "expression(" should be stripped and caught
    assert_eq!(
        sanitize_css_value("url(jav\tascript:alert(1))"),
        "",
        "TAB should be stripped and javascript: caught"
    );
    // CR-only between "java" and "script:"
    assert_eq!(
        sanitize_css_value("url(java\rscript:alert(1))"),
        "",
        "CR should be stripped and javascript: caught"
    );
    // LF-only between "java" and "script:"
    assert_eq!(
        sanitize_css_value("url(java\nscript:alert(1))"),
        "",
        "LF should be stripped and javascript: caught"
    );
    // Multiple whitespace chars combined
    assert_eq!(
        sanitize_css_value("url(jav\ta\r\nscript:alert(1))"),
        "",
        "Mixed TAB/CR/LF should all be stripped and javascript: caught"
    );
    // Normal safe values should still pass
    assert_eq!(
        sanitize_css_value("url(https://example.com/image.png)"),
        "url(https://example.com/image.png)",
        "Normal safe URLs should pass sanitization unchanged"
    );
}
