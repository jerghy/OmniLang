import pytest
from omnilang.errors import OmniError, CompilerUnavailable, InputError, CompileError, ConfigError


def test_omni_error_format():
    err = OmniError(
        severity="error", code="E0001",
        message="cannot resolve import",
        file="src/main.omni", line=3, col=8,
        suggestion="check the import path"
    )
    output = err.format()
    assert "Error:" in output
    assert "src/main.omni:3:8" in output
    assert "E0001" in output
    assert "check the import path" in output


def test_compiler_unavailable():
    err = CompilerUnavailable()
    assert err.exit_code == 2
    assert "omni_compiler.dll" in err.format()


def test_input_error_file_not_found():
    err = InputError.file_not_found("missing.omni")
    assert err.exit_code == 4
    assert "missing.omni" in err.format()


def test_multiple_errors():
    err = CompileError([
        OmniError("error", "E0001", "first error", "a.omni", 1, 0, None),
        OmniError("warning", "W0001", "first warning", "a.omni", 2, 0, None),
    ])
    output = err.format()
    assert "1 error(s)" in output
    assert "1 warning(s)" in output


def test_config_error_not_found():
    err = ConfigError.not_found()
    assert err.exit_code == 3
    assert ".omnilang.toml" in err.format()


def test_config_error_malformed():
    err = ConfigError([OmniError(
        severity="error", code="CONFIG_MALFORMED",
        message="Failed to parse .omnilang.toml: unexpected token at line 3",
        file=".omnilang.toml", line=3, col=1,
    )])
    assert err.exit_code == 3
    output = err.format()
    assert ".omnilang.toml" in output
    assert (":3" in output)
