# editor/server/annotation/__init__.py
"""OmniLang Editor — annotation comment parser/generator/validator."""

from editor.server.annotation.types import (
    AnnotationFile,
    CodeTabType,
    Chain,
    NodeEntry,
    WireEntry,
    ViewMarker,
    ViewRenderMode,
    AnnotationError,
    AnnotationErrorCode,
)
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.generator import generate_annotations
from editor.server.annotation.validator import validate_annotations

__all__ = [
    "AnnotationFile",
    "CodeTabType",
    "Chain",
    "NodeEntry",
    "WireEntry",
    "ViewMarker",
    "ViewRenderMode",
    "AnnotationError",
    "AnnotationErrorCode",
    "parse_annotations",
    "generate_annotations",
    "validate_annotations",
]
