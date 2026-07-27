//! Intent IR — types and lowering logic.
//!
//! Pipeline: AST → IR types → codegen / JSON serialization.

pub mod design;
pub mod logic;
mod lower;
pub mod lower_logic;
pub mod optimize;
mod types;
mod types_motion;
pub mod visibility;

pub use lower::lower;
pub use lower::lower_design;

// ── Explicit re-exports from types.rs ──
pub use types::{
    event_data_kind_label, event_data_schema, map_event_data_kind, BinaryOpIR, BindingIR, BlockIR,
    ComponentIR, ComputedIntent, ConditionalBranch, ConditionalIntent, ConstructionKind,
    EffectIntent, ElementIR, EventDataKind, EventFieldDef, EventHandlerIR, ExprIR, IRProgram,
    LifecycleIntent, LifecycleKind, ListRenderingIntent, MatchArmIR, MatchPatternIR,
    MatchPatternKindIR, MutationIntent, MutationOp, ParamIR, SignalRefIR, SignalIntent, StatementIR,
    StoreIR, StyleRuleIR, TypeIR, TypeKind, UnaryOpIR, ViewTree,
};

// ── Explicit re-exports from logic.rs ──
pub use logic::{
    EnumIR, ExportIR, FieldIR, FunctionIR, ImportIR, ImportKind, ModuleIR, StructIR, TypeAliasIR,
    VariantIR, Visibility,
};

// ── Explicit re-exports from design.rs ──
// Note: VariantIR is NOT re-exported here to avoid a name conflict with
// `logic::VariantIR` (tagged union variant). Use `ir::design::VariantIR` instead.
pub use design::{
    ClassIR, DesignIR, PseudoStateIR, TokenIR, TokenTypeIR, VariantBranchIR, VariantDimIR,
};

// ── Explicit re-exports from types_motion.rs ──
pub use types_motion::{
    CallbackBodyIR, CallbackKindIR, ConnectorIR, DirectionIR, DirectionValue, DragBehavior,
    EasingIR, FillValue, HeroBehavior, HeroIntent, InterruptValue, KeyframeIR, KeyframeStopIR,
    LoopValue, MotionBehavior, MotionBodyIR, MotionExprIR, MotionIntent, PlaybackIR,
    ScrollBehavior, StaggerDirection, StaggerTimingIR, StateTweenIR, StatesIR, TimelineIR,
    TimelineIntent, TimelineItemIR, TimelineItemKindIR, TriggerIntent, TriggerKind, WhenValue,
};

pub(crate) use lower::{lower_block, lower_type};

// ── Private imports visible to child modules via `use super::*;` ──
use rustc_hash::FxHashMap;
