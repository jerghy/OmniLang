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
    pub is_copy: bool,
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
