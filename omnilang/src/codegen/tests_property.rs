//! Property-based tests for codegen utilities.
//!
//! Focuses on `binary_op_to_str` — the operator mapping function
//! that translates OmniLang's internal operator names into target
//! language symbols.

use crate::codegen::binary_op_to_str;
use proptest::prelude::*;

proptest! {
    #[test]
    fn binary_op_to_str_known_ops_non_empty(
        op in "(add|sub|mul|div|mod|eq|neq|lt|gt|lteq|gteq|and|or|assign)"
    ) {
        let result = binary_op_to_str(&op, false, false);
        assert!(!result.is_empty(), "known op {:?} produced empty string", op);
    }

    #[test]
    fn binary_op_to_str_unknown_passthrough(op in "[a-z]+") {
        let result = binary_op_to_str(&op, false, false);
        assert!(!result.is_empty(), "unknown op {:?} produced empty string", op);
    }
}
