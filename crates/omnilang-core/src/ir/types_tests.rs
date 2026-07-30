//! Tests for IR types — serde round-trip and behavioral tests.
//! Extracted from types.rs for better file organization.

use super::*;

#[test]
fn serde_roundtrip_lifecycle_kind_mount() {
    let v = LifecycleKind::Mount;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"mount\"");
    let back: LifecycleKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, LifecycleKind::Mount);
}

#[test]
fn serde_roundtrip_lifecycle_kind_unmount() {
    let v = LifecycleKind::Unmount;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"unmount\"");
    let back: LifecycleKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, LifecycleKind::Unmount);
}

#[test]
fn serde_roundtrip_lifecycle_kind_effect() {
    let v = LifecycleKind::Effect;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"effect\"");
    let back: LifecycleKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, LifecycleKind::Effect);
}

#[test]
fn serde_roundtrip_event_handler_fn_ref() {
    let v = EventHandlerIR::FnRef {
        r#ref: "myFn".into(),
        params: vec!["e".into()],
        event_data: Some(EventDataKind::PointerEvent),
        used_fields: Some(vec!["x".into()]),
        needs_prevent_default: false,
        needs_stop_propagation: true,
    };
    let json = serde_json::to_string(&v).unwrap();
    let back: EventHandlerIR = serde_json::from_str(&json).unwrap();
    assert_eq!(json.contains("\"kind\":\"fn_ref\""), true);
    match back {
        EventHandlerIR::FnRef { r#ref, .. } => assert_eq!(r#ref, "myFn"),
        _ => panic!("Expected FnRef"),
    }
}

#[test]
fn serde_roundtrip_event_handler_inline_fn() {
    let v = EventHandlerIR::InlineFn {
        body: BlockIR { statements: vec![] },
        params: vec![],
        event_data: None,
        used_fields: None,
        needs_prevent_default: false,
        needs_stop_propagation: false,
    };
    let json = serde_json::to_string(&v).unwrap();
    let back: EventHandlerIR = serde_json::from_str(&json).unwrap();
    assert!(json.contains("\"kind\":\"inline_fn\""));
    match back {
        EventHandlerIR::InlineFn { .. } => {} // ok
        _ => panic!("Expected InlineFn"),
    }
}

#[test]
fn serde_roundtrip_event_handler_store_action() {
    let v = EventHandlerIR::StoreAction {
        r#ref: "MyStore.action".into(),
    };
    let json = serde_json::to_string(&v).unwrap();
    let back: EventHandlerIR = serde_json::from_str(&json).unwrap();
    assert!(json.contains("\"kind\":\"store_action\""));
    match back {
        EventHandlerIR::StoreAction { r#ref, .. } => assert_eq!(r#ref, "MyStore.action"),
        _ => panic!("Expected StoreAction"),
    }
}

// ── MutationOp serde round-trip tests ──

#[test]
fn serde_roundtrip_mutation_op_direct_assign() {
    let v = MutationOp::DirectAssign;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"direct_assign\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::DirectAssign);
}

#[test]
fn serde_roundtrip_mutation_op_increment() {
    let v = MutationOp::Increment;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"increment\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::Increment);
}

#[test]
fn serde_roundtrip_mutation_op_decrement() {
    let v = MutationOp::Decrement;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"decrement\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::Decrement);
}

#[test]
fn serde_roundtrip_mutation_op_multiply() {
    let v = MutationOp::Multiply;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"multiply\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::Multiply);
}

#[test]
fn serde_roundtrip_mutation_op_divide() {
    let v = MutationOp::Divide;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"divide\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::Divide);
}

#[test]
fn serde_roundtrip_mutation_op_modulo() {
    let v = MutationOp::Modulo;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"modulo\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::Modulo);
}

#[test]
fn serde_roundtrip_mutation_op_push() {
    let v = MutationOp::Push;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"push\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::Push);
}

#[test]
fn serde_roundtrip_mutation_op_pop() {
    let v = MutationOp::Pop;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"pop\"");
    let back: MutationOp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, MutationOp::Pop);
}

#[test]
fn serde_roundtrip_construction_kind_component() {
    let v = ConstructionKind::Component;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"component\"");
    let back: ConstructionKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ConstructionKind::Component);
}

#[test]
fn serde_roundtrip_construction_kind_conditional() {
    let v = ConstructionKind::Conditional;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"conditional\"");
    let back: ConstructionKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ConstructionKind::Conditional);
}

#[test]
fn serde_roundtrip_construction_kind_list() {
    let v = ConstructionKind::List;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"list\"");
    let back: ConstructionKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ConstructionKind::List);
}

#[test]
fn serde_roundtrip_construction_kind_normal() {
    let v = ConstructionKind::Normal;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"normal\"");
    let back: ConstructionKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ConstructionKind::Normal);
}

#[test]
fn serde_roundtrip_construction_kind_static() {
    let v = ConstructionKind::Static;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"static\"");
    let back: ConstructionKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ConstructionKind::Static);
}

// ── BinaryOpIR serde round-trip tests ──

#[test]
fn serde_roundtrip_binary_op_add() {
    let v = BinaryOpIR::Add;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"add\"");
    let back: BinaryOpIR = serde_json::from_str(&json).unwrap();
    assert_eq!(back, BinaryOpIR::Add);
}

#[test]
fn serde_roundtrip_binary_op_sub() {
    let v = BinaryOpIR::Sub;
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"sub\"");
    let back: BinaryOpIR = serde_json::from_str(&json).unwrap();
    assert_eq!(back, BinaryOpIR::Sub);
}

#[test]
fn serde_roundtrip_binary_op_arithmetic() {
    use BinaryOpIR::*;
    for (op, expected_json) in &[
        (Add, "\"add\""),
        (Sub, "\"sub\""),
        (Mul, "\"mul\""),
        (Div, "\"div\""),
        (Mod, "\"mod\""),
    ] {
        let json = serde_json::to_string(op).unwrap();
        assert_eq!(&json, expected_json);
        let back: BinaryOpIR = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, op);
    }
}

#[test]
fn serde_roundtrip_binary_op_comparison() {
    use BinaryOpIR::*;
    for (op, expected_json) in &[
        (Eq, "\"eq\""),
        (Neq, "\"neq\""),
        (Lt, "\"lt\""),
        (Gt, "\"gt\""),
        (LtEq, "\"lteq\""),
        (GtEq, "\"gteq\""),
    ] {
        let json = serde_json::to_string(op).unwrap();
        assert_eq!(&json, expected_json);
        let back: BinaryOpIR = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, op);
    }
}

#[test]
fn serde_roundtrip_binary_op_logical() {
    use BinaryOpIR::*;
    for (op, expected_json) in &[
        (And, "\"and\""),
        (Or, "\"or\""),
        (NullishCoalesce, "\"\u{3f}\u{3f}\""),
    ] {
        let json = serde_json::to_string(op).unwrap();
        assert_eq!(&json, expected_json);
        let back: BinaryOpIR = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, op);
    }
}

#[test]
fn serde_roundtrip_binary_op_assignment() {
    use BinaryOpIR::*;
    for (op, expected_json) in &[
        (Assign, "\"assign\""),
        (AddAssign, "\"addassign\""),
        (SubAssign, "\"subassign\""),
        (MulAssign, "\"mulassign\""),
        (DivAssign, "\"divassign\""),
        (ModAssign, "\"modassign\""),
    ] {
        let json = serde_json::to_string(op).unwrap();
        assert_eq!(&json, expected_json);
        let back: BinaryOpIR = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, op);
    }
}

#[test]
fn serde_roundtrip_binary_op_expr_in_context() {
    // Test BinaryOpIR serde within ExprIR::BinaryOpExpr
    let expr = ExprIR::BinaryOpExpr {
        op: BinaryOpIR::Add,
        left: Box::new(ExprIR::ident("a")),
        right: Box::new(ExprIR::ident("b")),
    };
    let json = serde_json::to_string(&expr).unwrap();
    assert!(json.contains("\"_kind\":\"BinaryOpExpr\""));
    assert!(json.contains("\"op\":\"add\""));
    let back: ExprIR = serde_json::from_str(&json).unwrap();
    match back {
        ExprIR::BinaryOpExpr { op, .. } => assert_eq!(op, BinaryOpIR::Add),
        _ => panic!("Expected BinaryOpExpr"),
    }
}

// ── TypeKind serde round-trip tests ──

#[test]
fn test_typekind_named_serializes_as_flat_string() {
    let array_string = TypeKind::Named {
        name: "Array".into(),
    };

    let json = serde_json::to_string(&array_string).expect("serialize");
    // Named always serializes as flat string now
    assert_eq!(json, r#""Array""#, "Named must serialize as flat string");

    let roundtripped: TypeKind = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(roundtripped, array_string, "Round-trip must preserve name");
}

#[test]
fn test_typekind_deserialize_backward_compat_flat_string() {
    let old_json = r#""Array""#;
    let ty: TypeKind = serde_json::from_str(old_json).expect("deserialize old format");
    assert_eq!(
        ty,
        TypeKind::Named {
            name: "Array".into(),
        },
        "Old flat-string format must still deserialize"
    );
}

#[test]
fn test_typekind_deserialize_backward_compat_structured() {
    // Old structured format with type_args — must still deserialize (type_args ignored)
    let old_json = r#"{"name": "Array", "type_args": [{"name": "String", "type_args": []}]}"#;
    let ty: TypeKind = serde_json::from_str(old_json).expect("deserialize old structured format");
    assert_eq!(
        ty,
        TypeKind::Named {
            name: "Array".into(),
        },
        "Old structured format must still deserialize (type_args ignored)"
    );
}

#[test]
fn test_typekind_deserialize_backward_compat_structured_no_type_args() {
    // Structured format without type_args
    let old_json = r#"{"name": "Array"}"#;
    let ty: TypeKind = serde_json::from_str(old_json).expect("deserialize structured format");
    assert_eq!(
        ty,
        TypeKind::Named {
            name: "Array".into(),
        },
        "Structured format without type_args must deserialize"
    );
}

#[test]
fn test_typekind_simple_variants_still_flat() {
    let int_ty = TypeKind::Int;
    let json = serde_json::to_string(&int_ty).expect("serialize");
    assert_eq!(json, r#""int""#, "Simple types stay flat strings");
}
