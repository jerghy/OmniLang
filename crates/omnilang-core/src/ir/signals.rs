//! Signal, computed, effect, mutation, and lifecycle intent types.
//! Extracted from types.rs for better file organization.

use super::types::{BlockIR, ExprIR, SignalRefIR, TypeIR};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════
// Signal & Binding References (SignalRefIR kept in types.rs)
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalIntent {
    pub name: String,
    pub r#type: TypeIR,
    pub initial_value: ExprIR,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOp {
    DirectAssign,
    Increment,
    Decrement,
    Multiply,
    Divide,
    Modulo,
    Push,
    Pop,
}

impl std::fmt::Display for MutationOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutationOp::DirectAssign => write!(f, "direct_assign"),
            MutationOp::Increment => write!(f, "increment"),
            MutationOp::Decrement => write!(f, "decrement"),
            MutationOp::Multiply => write!(f, "multiply"),
            MutationOp::Divide => write!(f, "divide"),
            MutationOp::Modulo => write!(f, "modulo"),
            MutationOp::Push => write!(f, "push"),
            MutationOp::Pop => write!(f, "pop"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
#[non_exhaustive]
pub enum MutationIntent {
    #[serde(rename = "direct_assign")]
    DirectAssign {
        target: SignalRefIR,
        #[serde(default)]
        value: Option<Box<ExprIR>>,
        /// The owner identifier for `owner.field = value` writes (e.g. the ref
        /// name in `myRef.value = x`). `None` for plain signal/name writes.
        /// Populated by lowering so the semantic ref pass can diagnose ref
        /// field writes with the bind:value migration hint. Codegen consults
        /// it only as a discriminator: the React write-only DCE guard
        /// (review I1/N1) treats a mutation with a `field_object` as an
        /// owner-field write (store/ref), never a component-signal mutation
        /// — such writes are never pruned even when their bare field name
        /// collides with a pruned component signal.
        #[serde(default)]
        field_object: Option<String>,
    },
    #[serde(rename = "increment")]
    Increment {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "decrement")]
    Decrement {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "multiply")]
    Multiply {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "divide")]
    Divide {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "modulo")]
    Modulo {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "push")]
    Push {
        target: SignalRefIR,
        item: Option<Box<ExprIR>>,
    },
    #[serde(rename = "pop")]
    Pop { target: SignalRefIR },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedIntent {
    pub name: String,
    pub r#type: TypeIR,
    pub body: ExprIR,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectIntent {
    pub deps: Vec<String>,
    pub body: BlockIR,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleKind {
    Mount,
    Unmount,
    Effect,
}

impl std::fmt::Display for LifecycleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleKind::Mount => write!(f, "mount"),
            LifecycleKind::Unmount => write!(f, "unmount"),
            LifecycleKind::Effect => write!(f, "effect"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleIntent {
    pub kind: LifecycleKind,
    pub body: BlockIR,
}
