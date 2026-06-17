"""Result types for compilation and analysis operations."""
from __future__ import annotations

from dataclasses import dataclass, field
from omnilang.errors import OmniError


@dataclass
class CompileResult:
    success: bool
    files: dict[str, str] = field(default_factory=dict)
    errors: list[OmniError] = field(default_factory=list)
    warnings: list[OmniError] = field(default_factory=list)

    @classmethod
    def from_rust_response(cls, data: dict, default_file: str = "<input>") -> "CompileResult":
        errors: list[OmniError] = []
        warnings: list[OmniError] = []
        for item in data.get("errors", []):
            err = OmniError(
                severity=item.get("severity", "error"),
                code=item.get("code", "RUST_ERROR"),
                message=str(item.get("message", item)),
                file=item.get("file", default_file),
                line=item.get("line"),
                col=item.get("col"),
                suggestion=item.get("suggestion"),
            )
            if err.severity == "warning":
                warnings.append(err)
            else:
                errors.append(err)
        for item in data.get("warnings", []):
            if isinstance(item, dict):
                warnings.append(OmniError(
                    severity="warning",
                    code=item.get("code", "RUST_WARN"),
                    message=str(item.get("message", item)),
                    file=item.get("file", default_file),
                    line=item.get("line"),
                    col=item.get("col"),
                ))
        return cls(
            success=data.get("success", False),
            files=data.get("files", {}),
            errors=errors,
            warnings=warnings,
        )


@dataclass
class AnalyseResult:
    success: bool
    diagnostics: list[OmniError] = field(default_factory=list)

    @classmethod
    def from_rust_response(cls, data: dict, default_file: str = "<input>") -> "AnalyseResult":
        diagnostics: list[OmniError] = []
        for item in data.get("diagnostics", []):
            diagnostics.append(OmniError(
                severity=item.get("severity", "error"),
                code=item.get("code", "ANALYSE"),
                message=str(item.get("message", item)),
                file=item.get("file", default_file),
                line=item.get("line"),
                col=item.get("col"),
                suggestion=item.get("suggestion"),
            ))
        return cls(
            success=data.get("success", False),
            diagnostics=diagnostics,
        )
