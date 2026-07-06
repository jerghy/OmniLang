//! General helper functions for semantic analysis.

use crate::lexer::Span;
use crate::parser::ast::*;

/// Convert a TypeAnnotation to a human-readable string.
pub(super) fn type_to_string(ty: &TypeAnnotation) -> String {
    match ty {
        TypeAnnotation::Named {
            name, type_args, ..
        } => {
            if type_args.is_empty() {
                name.clone()
            } else {
                let args: Vec<_> = type_args.iter().map(type_to_string).collect();
                format!("{}<{}>", name, args.join(", "))
            }
        }
        TypeAnnotation::Optional { base, .. } => format!("{}?", type_to_string(base)),
        TypeAnnotation::Union { types, .. } => {
            let t: Vec<_> = types.iter().map(type_to_string).collect();
            t.join(" | ")
        }
        TypeAnnotation::Fn {
            params,
            return_type,
            ..
        } => {
            let p: Vec<_> = params.iter().map(type_to_string).collect();
            format!("fn({}) -> {}", p.join(", "), type_to_string(return_type))
        }
    }
}

/// Check if a CSS property name is non-interpolatable (cannot be animated).
pub(super) fn is_non_interpolatable(prop: &str) -> bool {
    matches!(
        prop,
        "display"
            | "position"
            | "overflow"
            | "flex-direction"
            | "flexDirection"
            | "grid-template"
            | "gridTemplate"
            | "visibility"
            | "content"
            | "cursor"
            | "pointer-events"
            | "pointerEvents"
            | "user-select"
            | "userSelect"
            | "font-family"
            | "fontFamily"
            | "text-decoration"
            | "textDecoration"
            | "white-space"
            | "whiteSpace"
    )
}

/// Collect all motion/timeline names referenced by a timeline definition.
pub(super) fn collect_timeline_refs(timeline: &TimelineDef) -> Vec<&str> {
    timeline
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            TimelineItemKind::MotionRef(name) => Some(name.as_str()),
            TimelineItemKind::Stagger(stagger) => {
                if let Expr::VarRef(name, _) = stagger.motion.as_ref() {
                    Some(name.as_str())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect()
}

/// Extension trait to get span from TypeAnnotation.
impl TypeAnnotation {
    pub fn span(&self) -> Span {
        match self {
            TypeAnnotation::Named { span, .. } => *span,
            TypeAnnotation::Optional { span, .. } => *span,
            TypeAnnotation::Union { span, .. } => *span,
            TypeAnnotation::Fn { span, .. } => *span,
        }
    }
}
