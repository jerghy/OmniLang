//! Intent IR — types and lowering logic.
//!
//! Pipeline: AST → IR types → codegen / JSON serialization.

pub mod logic;
mod lower;
pub mod lower_logic;
mod types;
mod types_motion;
pub mod visibility;

pub use lower::lower;

// ── Explicit re-exports from types.rs ──
pub use types::{
    BinaryOpIR, BindingIR, BlockIR, ComponentIR, ComputedIntent, ConditionalBranch, ConditionalIntent,
    ConstructionKind, EffectIntent, ElementIR, EventDataKind, EventFieldDef, EventHandlerIR,
    ExprIR, IRProgram, LifecycleIntent, LifecycleKind, ListRenderingIntent, MatchArmIR,
    MatchPatternIR, MatchPatternKindIR, MutationIntent, MutationOp, ParamIR, SignalRefIR,
    StateIntent, StatementIR, StoreIR, StyleRuleIR, TypeIR, TypeKind, ViewTree,
    event_data_kind_label, event_data_schema, map_event_data_kind,
};

// ── Explicit re-exports from logic.rs ──
pub use logic::{
    EnumIR, ExportIR, FieldIR, FunctionIR, ImportIR, ImportKind, ModuleIR, StructIR,
    TypeAliasIR, VariantIR, Visibility,
};

// ── Explicit re-exports from types_motion.rs ──
pub use types_motion::{
    CallbackBodyIR, CallbackKindIR, ConnectorIR, DirectionIR, DirectionValue, DragBehavior,
    EasingIR, FillValue, HeroBehavior, HeroIntent, InterruptValue, KeyframeIR, KeyframeStopIR,
    LoopValue, MotionBehavior, MotionBodyIR, MotionExprIR, MotionIntent, PlaybackIR,
    ScrollBehavior, StaggerDirection, StaggerTimingIR, StateTweenIR, StatesIR, TimelineIntent,
    TimelineItemIR, TimelineItemKindIR, TriggerIntent, TriggerKind, WhenValue,
};

pub(crate) use lower::{lower_block, lower_type};

// ── Private imports visible to child modules via `use super::*;` ──
use std::collections::HashMap;
