"""OmniLang 编译管道共享入口。（G1）

被 omniview/cli.py 和 editor/server/ 共同导入。
纯函数 compile_omni() — 无文件 I/O，输入源码字符串，输出字典。

用法:
    from omniview.api import compile_omni, CompileResult, CompileError

    result = compile_omni(source, filepath="/abs/path/to/App.omni", target="react")
    if result.success:
        for filename, content in result.files.items():
            ...
"""

from __future__ import annotations

import re as _re
from dataclasses import dataclass, field
from pathlib import Path


# ── 错误协议 (G2) ──────────────────────────────────────

@dataclass
class CompileError:
    """统一的编译错误格式。parser/lowering/codegen 均返回此类型。"""
    file: str                           # 源文件名
    line: int                           # 行号 (1-based)
    col: int = 0                        # 列号 (1-based)
    severity: str = "error"             # "error" | "warning"
    code: str = ""                      # 错误码，如 "E0101"
    message: str = ""                   # 人类可读描述
    suggestion: str | None = None       # 修复建议

    def to_dict(self) -> dict:
        return {
            "file": self.file,
            "line": self.line,
            "col": self.col,
            "severity": self.severity,
            "code": self.code,
            "message": self.message,
            "suggestion": self.suggestion,
        }


@dataclass
class CompileResult:
    """编译结果。success=True 时 files 和 diff 有效；success=False 时 errors 有效。"""
    success: bool
    files: dict[str, str] = field(default_factory=dict)     # { "App.tsx": "...", "App.module.css": "..." }
    diff: dict[str, str] | None = None                       # { "App.tsx": "- old\n+ new" }
    errors: list[CompileError] = field(default_factory=list)
    source: str = ""                                         # 原始 .omni 源码（用于后续 diff）


# ── 主编译函数 ──────────────────────────────────────────

def compile_omni(source: str, filepath: str = "<input>", target: str = "react",
                 previous_files: dict[str, str] | None = None,
                 debug_mode: bool = False) -> CompileResult:
    """编译 .omni 源码 → { filename: content } 字典。纯函数，无文件 I/O。

    Args:
        source: .omni 源码字符串。
        filepath: 源文件路径（用于错误消息）。
        target: 目标框架名 ("react"|"vue"|"dioxus"|"flutter"|"python"|"javascript"|"rust"|"dart")。
        previous_files: 上次编译结果（用于计算 diff）。
        debug_mode: 启用运行时调试注入（useTrackedSignal 等）。

    Returns:
        CompileResult — success/files/diff/errors。
    """
    errors: list[CompileError] = []
    is_ui_mode = _detect_ui_mode(source)

    # 纯逻辑模式自动回退 target：react/vue/dioxus/flutter → python
    PURE_LOGIC_TARGETS = {"python", "javascript", "js", "rust", "dart"}
    if not is_ui_mode and target not in PURE_LOGIC_TARGETS:
        target = "python"

    # Try Rust compiler first (if available) for UI mode
    if is_ui_mode:
        from omniview.codegen.rust_bridge import RustBridge
        if RustBridge.is_available():
            ir_result = RustBridge.compile_to_ir(source)
            if "_rust_errors" in ir_result:
                # Rust found errors -> report them
                for err in ir_result["_rust_errors"]:
                    errors.append(CompileError(
                        file=filepath, line=err.get("line", 1), col=err.get("col", 0),
                        severity=err.get("severity", "error"),
                        code=err.get("code", "RUST"), message=err.get("message", str(err)),
                    ))
                return CompileResult(success=False, files={}, errors=errors, source=source)
            # Rust succeeded -> pass IR to Python codegen
            return _compile_from_ir(ir_result, target, previous_files, source)

    try:
        if is_ui_mode:
            compiled_files = _compile_ui_mode(source, filepath, target, errors, debug_mode)
        else:
            compiled_files = _compile_pure_logic(source, filepath, target, errors)
    except SystemExit:
        # 语义分析/代码生成调用 sys.exit(1) → 转换为 CompileError
        if not errors:
            errors.append(CompileError(
                file=filepath, line=1, severity="error", code="SEMANTIC",
                message="Semantic analysis failed (see errors above).",
            ))
    except Exception as exc:
        errors.append(CompileError(
            file=filepath,
            line=1,
            severity="error",
            code="INTERNAL",
            message=f"Compiler internal error: {exc}",
        ))
        return CompileResult(success=False, files={}, errors=errors, source=source)

    # 计算 diff
    diff: dict[str, str] | None = None
    if previous_files is not None:
        diff = _compute_diff(previous_files, compiled_files)

    # WARNING 不是错误——只有 severity=="error" 才算失败
    has_errors = any(e.severity == "error" for e in errors)

    return CompileResult(
        success=not has_errors,
        files=compiled_files,
        diff=diff,
        errors=errors,
        source=source,
    )


# ── 模式检测 ───────────────────────────────────────────

def _detect_ui_mode(source: str) -> bool:
    """检测源码是否为 UI 模式（含 OmniView 组件/Store/Theme）。"""
    ui_keywords = ("component ", "store ", "theme ", "view {", "@if", "@for", "signal ", "computed ")
    has_ui_keywords = any(kw in source for kw in ui_keywords)
    has_ui_keywords = has_ui_keywords or bool(_re.search(r'<[A-Z]', source))
    has_import_omniview = "import omniview" in source
    return has_import_omniview or has_ui_keywords


# ── 纯逻辑模式 ─────────────────────────────────────────

def _compile_pure_logic(source: str, filepath: str, target: str, errors: list[CompileError]) -> dict[str, str]:
    """纯逻辑模式编译（无 OmniView 组件）。"""

    from omnilang.lexer import create_parser
    from omnilang.parser import OmniTransformer

    try:
        tree = create_parser().parse(source)
    except Exception as exc:
        errors.append(_parse_error_to_compile_error(exc, filepath))
        return {}

    program = OmniTransformer().transform(tree)

    base = Path(filepath).stem
    ext_map = {"python": ".py", "javascript": ".js", "js": ".js", "rust": ".rs", "dart": ".dart"}
    # Normalize "javascript" → canonical "js" for codegen
    codegen_target = "js" if target in ("javascript", "js") else target

    if codegen_target not in ("python", "js", "rust", "dart"):
        errors.append(CompileError(
            file=filepath, line=1, severity="error", code="UNSUPPORTED",
            message=f"Pure logic target '{target}' not supported. Use: python, javascript, rust, dart.",
        ))
        return {}

    from omnilang.codegen import generate
    from omnilang.semantic import analyze

    # 捕获语义分析的 stderr 输出和 sys.exit
    import io
    import sys as _sys
    _old_stderr = _sys.stderr
    _stderr_buf = io.StringIO()
    _sys.stderr = _stderr_buf

    try:
        source_lines = {filepath: source.split("\n")}
        type_map, rename_map = analyze(program, source_lines)
    except SystemExit:
        # 语义分析发现错误，调用了 sys.exit(1)
        # 从 stderr 解析错误
        captured = _stderr_buf.getvalue()
        for line in captured.strip().split("\n"):
            if line.startswith("Error:"):
                errors.append(CompileError(
                    file=filepath, line=1, severity="error", code="SEMANTIC",
                    message=line.replace("Error:", "").strip(),
                ))
        return {}
    finally:
        _sys.stderr = _old_stderr

    code = generate(program, codegen_target, rename_map, type_map)
    return {f"{base}{ext_map[target]}": code}


# ── UI 模式 ────────────────────────────────────────────

def _compile_ui_mode(source: str, filepath: str, target: str, errors: list[CompileError],
                     debug_mode: bool = False) -> dict[str, str]:
    """UI 模式编译（含 OmniView 组件/Store）。"""

    from omniview.preprocessor import preprocess
    from omnilang.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer

    # Stage 1: Preprocess
    try:
        pp_result = preprocess(source)
    except Exception as exc:
        errors.append(CompileError(file=filepath, line=1, severity="error", code="PREPROC", message=str(exc)))
        return {}

    # Stage 2: Parse
    try:
        tree = create_omniview_parser().parse(pp_result.processed_source)
    except Exception as exc:
        errors.append(_parse_error_to_compile_error(exc, filepath))
        return {}

    transformer = OmniViewTransformer(block_map=pp_result.blocks)
    program = transformer.transform(tree)

    # Stage 3: Semantic analysis
    from omniview.semantic.analyzer import SemanticAnalyzer
    analyzer = SemanticAnalyzer(source_lines=source.split('\n'))
    result = analyzer.analyze(program)
    for d in result.diagnostics:
        if d.level in ("ERROR", "WARNING"):
            errors.append(CompileError(
                file=filepath, line=getattr(d, 'line', 1), col=getattr(d, 'col', 0),
                severity="error" if d.level == "ERROR" else "warning",
                code=getattr(d, 'code', ''), message=str(d),
            ))

    if result.has_errors:
        return {}

    # Stage 4: Lower to IR
    from omniview.lowering import IRFixture
    lower = IRFixture()
    ir_map = lower.lower_program(program)

    # Stage 5: Codegen
    codegen = _get_codegen(target, filepath, errors, debug_mode)
    if codegen is None:
        return {}

    output: dict[str, str] = {}
    for name, ir in ir_map.items():
        try:
            if "states" in ir or "view" in ir:
                gen_result = codegen.generate_component(ir, ir_map, source_line=1)
            else:
                gen_result = codegen.generate_store(ir, source_line=1)

            for filename, content in gen_result.files.items():
                output[filename] = content

            for err_msg in gen_result.errors:
                errors.append(CompileError(file=filepath, line=1, severity="error", code="CODEGEN", message=err_msg))
        except Exception as exc:
            errors.append(CompileError(
                file=filepath, line=1, severity="error", code="CODEGEN",
                message=f"Codegen error for '{name}': {exc}",
            ))

    return output


def _get_codegen(target: str, filepath: str, errors: list[CompileError], debug_mode: bool = False):
    """获取 codegen 实例。"""
    if target == "react":
        from omniview.codegen.react import ReactCodegen
        return ReactCodegen(debug_mode=debug_mode)
    elif target == "vue":
        from omniview.codegen.vue import VueCodegen
        return VueCodegen()
    elif target == "dioxus":
        from omniview.codegen.dioxus import DioxusCodegen
        return DioxusCodegen()
    elif target == "flutter":
        from omniview.codegen.flutter import FlutterCodegen
        return FlutterCodegen()
    elif target == "omniview":
        from omniview.codegen.omniview import OmniViewCodegen
        return OmniViewCodegen(debug_mode=debug_mode)
    else:
        errors.append(CompileError(
            file=filepath, line=1, severity="error", code="UNSUPPORTED",
            message=f"Unknown target '{target}'. Supported: react, vue, dioxus, flutter.",
        ))
        return None


# ── 辅助 ───────────────────────────────────────────────

def _parse_error_to_compile_error(exc: Exception, filepath: str) -> CompileError:
    """将 Lark 解析异常转换为 CompileError。"""
    msg = str(exc)
    # 尝试提取行号
    line = 1
    col = 0
    m = _re.search(r"at line (\d+)", msg)
    if m:
        line = int(m.group(1))
    m = _re.search(r"at line (\d+), column (\d+)", msg)
    if m:
        line = int(m.group(1))
        col = int(m.group(2))
    return CompileError(file=filepath, line=line, col=col, severity="error", code="PARSE", message=msg.split("\n")[0])


def _compute_diff(previous: dict[str, str], current: dict[str, str]) -> dict[str, str]:
    """计算两次编译结果的 diff（简化版：逐文件对比）。"""
    import difflib
    result: dict[str, str] = {}
    all_files = set(previous.keys()) | set(current.keys())
    for filename in sorted(all_files):
        old_text = previous.get(filename, "")
        new_text = current.get(filename, "")
        if old_text == new_text:
            continue
        diff_lines = difflib.unified_diff(
            old_text.splitlines(keepends=True),
            new_text.splitlines(keepends=True),
            fromfile=filename, tofile=filename,
        )
        result[filename] = "".join(diff_lines)
    return result


def _compile_from_ir(ir_program: dict, target: str,
                     previous_files: dict[str, str] | None = None,
                     source: str = "") -> CompileResult:
    """Compile from IR JSON dict (Rust output) via Python codegen only.

    This is the Rust-pipeline path: Rust produces IR JSON, Python codegen
    consumes it to produce framework-specific target code.
    """
    codegen = _get_codegen(target, "<input>", [], debug_mode=False)
    if codegen is None:
        return CompileResult(
            success=False, files={},
            errors=[CompileError(
                file="<input>", line=1, severity="error", code="CODEGEN",
                message=f"Unsupported target: {target}",
            )],
            source=source,
        )

    current_files: dict[str, str] = {}
    errors: list[CompileError] = []

    for name, comp_ir in ir_program.get("components", {}).items():
        result = codegen.generate_component(comp_ir)
        current_files.update(result.files)
        for err_msg in result.errors:
            errors.append(CompileError(
                file=f"{name}.omni", line=1, severity="error",
                code="CODEGEN", message=err_msg,
            ))
        for warn_msg in result.warnings:
            errors.append(CompileError(
                file=f"{name}.omni", line=1, severity="warning",
                code="CODEGEN", message=warn_msg,
            ))

    for name, store_ir in ir_program.get("stores", {}).items():
        result = codegen.generate_store(store_ir)
        current_files.update(result.files)
        for err_msg in result.errors:
            errors.append(CompileError(
                file=f"{name}.omni", line=1, severity="error",
                code="CODEGEN", message=err_msg,
            ))

    # Compute diff if previous_files provided
    diff: dict[str, str] | None = None
    if previous_files:
        diff = _compute_diff(previous_files, current_files)

    has_errors = any(e.severity == "error" for e in errors)
    return CompileResult(
        success=not has_errors,
        files=current_files,
        diff=diff,
        errors=errors,
        source=source,
    )


# ── 视图树提取 (M2) ────────────────────────────────────

@dataclass
class ViewTreeNode:
    """视图树节点，用于画布渲染。"""
    id: str
    tag: str
    props: dict = field(default_factory=dict)
    children: list["ViewTreeNode"] = field(default_factory=list)
    test_id: str | None = None
    source_line: int = 1


def parse_view_tree(source: str, filepath: str = "<input>") -> list[ViewTreeNode]:
    """解析 .omni 源码 → 提取视图元素树（用于画布渲染）。

    只解析 UI 模式的源码。返回根元素列表。
    """
    from omniview.preprocessor import preprocess
    from omnilang.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer
    from omniview.lowering import IRFixture
    from omniview.ast_nodes import (
        Element, ViewBlock, IfDirective, ForDirective,
        TextNode, Interpolation, StaticProp,
        ComponentDef,
    )

    pp_result = preprocess(source)
    tree = create_omniview_parser().parse(pp_result.processed_source)
    transformer = OmniViewTransformer(block_map=pp_result.blocks)
    program = transformer.transform(tree)

    result: list[ViewTreeNode] = []
    for comp in program.components:
        if comp.view:
            node = _convert_view_to_tree(comp.view, comp.name)
            if node:
                result.append(node)

    return result


@dataclass
class CodeBlockNode:
    """从 .omni 源码中提取的逻辑块（signal/computed/action/store）。"""
    id: str
    blockType: str  # "store" | "signal" | "computed" | "action"
    name: str
    parentStore: str | None = None
    typeAnnotation: str | None = None
    sourceLine: int = 0


def parse_code_blocks(source: str, filepath: str = "<input>") -> list[CodeBlockNode]:
    """解析 .omni 源码 → 提取 store / signal / computed / action 声明。

    这些块可以渲染为画布上的带端口节点，用于连线可视化。
    """
    from omniview.preprocessor import preprocess
    from omnilang.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer

    pp_result = preprocess(source)
    tree = create_omniview_parser().parse(pp_result.processed_source)
    transformer = OmniViewTransformer(block_map=pp_result.blocks)
    program = transformer.transform(tree)

    blocks: list[CodeBlockNode] = []
    node_id = 0

    for store in program.stores:
        store_name = store.name
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-store-{store_name}",
            blockType="store",
            name=store_name,
            sourceLine=getattr(store, 'span', None) and store.span.line if store.span else 0,
        ))

        # signals
        for sig in store.signals:
            node_id += 1
            type_ann = str(sig.type_annotation or "") if hasattr(sig, 'type_annotation') and sig.type_annotation else ""
            blocks.append(CodeBlockNode(
                id=f"cb-{store_name}-signal-{sig.name}",
                blockType="signal",
                name=sig.name,
                parentStore=store_name,
                typeAnnotation=type_ann,
                sourceLine=getattr(sig, 'span', None) and sig.span.line if hasattr(sig, 'span') and sig.span else 0,
            ))

        # getters (computed)
        for getter in store.getters:
            node_id += 1
            type_ann = str(getter.return_type or "") if hasattr(getter, 'return_type') and getter.return_type else ""
            blocks.append(CodeBlockNode(
                id=f"cb-{store_name}-computed-{getter.name}",
                blockType="computed",
                name=getter.name,
                parentStore=store_name,
                typeAnnotation=type_ann,
                sourceLine=getattr(getter, 'span', None) and getter.span.line if hasattr(getter, 'span') and getter.span else 0,
            ))

        # actions
        for action in store.actions:
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{store_name}-action-{action.name}",
                blockType="action",
                name=action.name,
                parentStore=store_name,
                sourceLine=getattr(action, 'span', None) and action.span.line if hasattr(action, 'span') and action.span else 0,
            ))

    # Component-level declarations (signals, fns, computeds directly in components)
    for comp in program.components:
        comp_name = comp.name
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-store-{comp_name}",
            blockType="store",
            name=comp_name,
            sourceLine=getattr(comp, 'span', None) and comp.span.line if comp.span else 0,
        ))
        for sig in getattr(comp, 'signals', []) or []:
            node_id += 1
            type_ann = str(sig.type_annotation or "") if hasattr(sig, 'type_annotation') and sig.type_annotation else ""
            blocks.append(CodeBlockNode(
                id=f"cb-{comp_name}-signal-{sig.name}",
                blockType="signal",
                name=sig.name,
                parentStore=comp_name,
                typeAnnotation=type_ann,
                sourceLine=getattr(sig, 'span', None) and sig.span.line if hasattr(sig, 'span') and sig.span else 0,
            ))
        for fn in getattr(comp, 'fns', []) or []:
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{comp_name}-action-{fn.name}",
                blockType="action",
                name=fn.name,
                parentStore=comp_name,
                sourceLine=getattr(fn, 'span', None) and fn.span.line if hasattr(fn, 'span') and fn.span else 0,
            ))
        for c in getattr(comp, 'computeds', []) or []:
            node_id += 1
            type_ann = str(c.type_annotation or "") if hasattr(c, 'type_annotation') and c.type_annotation else ""
            blocks.append(CodeBlockNode(
                id=f"cb-{comp_name}-computed-{c.name}",
                blockType="computed",
                name=c.name,
                parentStore=comp_name,
                typeAnnotation=type_ann,
                sourceLine=getattr(c, 'span', None) and c.span.line if hasattr(c, 'span') and c.span else 0,
            ))

    return blocks


@dataclass
class NativePortInfo:
    """Extracted @in/@out port info from @native block annotations."""
    id: str
    target: str            # e.g. "react", "flutter"
    inputs: list[dict]     # [{name, type}]
    outputs: list[dict]    # [{name, type}]
    source_line: int = 0


def parse_native_ports(source: str, filepath: str = "<input>") -> list[NativePortInfo]:
    """Parse @native blocks and extract @in/@out comment annotations for canvas ports.

    Scans for patterns like:
        // @in: videoUrl (String)
        // @out: isPlaying (bool)
        @native(react) { ... }

    Returns list of NativePortInfo with extracted port definitions.
    """
    import re

    results: list[NativePortInfo] = []
    node_id = 0

    # Match @native blocks with their @in/@out annotations
    # Pattern: optional @in/@out comments followed by @native(target) { ... }
    _ANNOTATION_RE = re.compile(
        r"(?:^\s*//\s*@(in|out):\s*(\w+)\s*\((\w+)\)\s*$\n)*"
        r"^\s*@native\((\w+)\)\s*\{",
        re.MULTILINE,
    )

    # Simpler approach: scan line by line
    lines = source.split("\n")
    current_ins: list[dict] = []
    current_outs: list[dict] = []

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Detect @in annotation
        in_match = re.match(r"//\s*@in:\s*(\w+)\s*\((\w+)\)", stripped)
        if in_match:
            current_ins.append({"name": in_match.group(1), "type": in_match.group(2)})
            continue

        # Detect @out annotation
        out_match = re.match(r"//\s*@out:\s*(\w+)\s*\((\w+)\)", stripped)
        if out_match:
            current_outs.append({"name": out_match.group(1), "type": out_match.group(2)})
            continue

        # Detect @native block start
        native_match = re.match(r"@native\((\w+)\)\s*\{", stripped)
        if native_match:
            node_id += 1
            target = native_match.group(1)
            results.append(NativePortInfo(
                id=f"native-{target}-{node_id}",
                target=target,
                inputs=list(current_ins),
                outputs=list(current_outs),
                source_line=i + 1,
            ))
            current_ins = []
            current_outs = []
            continue

        # Reset annotations on blank lines or non-comment lines
        if not stripped or (not stripped.startswith("//") and not stripped.startswith("@")):
            current_ins = []
            current_outs = []

    return results


def _convert_view_to_tree(view: "ViewBlock", name: str) -> ViewTreeNode | None:
    """将 OmniView ViewBlock AST 转换为 ViewTreeNode。"""
    children = []
    for child in view.children:
        converted = _convert_view_node(child)
        if converted:
            children.append(converted)

    return ViewTreeNode(
        id=f"comp-{name}",
        tag="Container",
        props={"data-component": name},
        children=children,
    )


def _expr_to_label(expr) -> str:
    """将 AST 表达式节点转换为用户友好的标签字符串。"""
    from omnilang.ast_nodes import VarRef, Literal, BinaryOp, FuncCall, MethodCall
    from omniview.ast_nodes import FieldAccess as OmniFieldAccess

    if isinstance(expr, VarRef):
        return expr.name
    if hasattr(expr, 'object') and hasattr(expr, 'field'):
        # FieldAccess (both omnilang and omniview types)
        obj = _expr_to_label(expr.object) if expr.object else ""
        return f"{obj}.{expr.field}" if obj else expr.field
    if isinstance(expr, Literal):
        return str(expr.value)
    if isinstance(expr, BinaryOp):
        return f"{_expr_to_label(expr.left)} {expr.op} {_expr_to_label(expr.right)}"
    if isinstance(expr, FuncCall):
        name = _expr_to_label(expr.func) if hasattr(expr, 'func') else "fn"
        args = ", ".join(_expr_to_label(a) for a in (expr.args or []))
        return f"{name}({args})"
    # Fallback: extract name-related attributes
    if hasattr(expr, 'name'):
        return str(expr.name)
    if hasattr(expr, 'field'):
        return str(expr.field)
    if hasattr(expr, 'method'):
        return str(expr.method)
    # Last resort
    s = str(expr)
    return s[:40] if len(s) > 40 else s


def _convert_view_node(node) -> ViewTreeNode | None:
    """递归转换单个 ViewNode。"""
    from omniview.ast_nodes import (
        Element, IfDirective, ForDirective, SlotDirective,
        NativeBlock, FallbackBlock, ErrorBoundary,
        TextNode, Interpolation, StaticProp,
    )

    if isinstance(node, TextNode):
        return ViewTreeNode(
            id=f"text-{id(node)}",
            tag="Text",
            props={"label": node.content},
        )

    if isinstance(node, Interpolation):
        expr_str = _expr_to_label(node.expr) if hasattr(node, 'expr') else "{}"
        return ViewTreeNode(
            id=f"interp-{id(node)}",
            tag="Text",
            props={"label": f"{{{expr_str}}}"},
        )

    if isinstance(node, Element):
        props = {}
        test_id = None
        for prop in node.props:
            val = prop.value
            if isinstance(val, StaticProp):
                raw = val.value
                if isinstance(raw, str):
                    props[prop.name] = raw
                elif raw is not None:
                    props[prop.name] = _expr_to_label(raw)
            else:
                # BindProp / EventProp / ClassProp / StyleProp → clean label
                type_name = type(val).__name__
                if type_name == "BindProp":
                    props[prop.name] = f"{{bind:{getattr(val, 'signal_ref', '?')}}}"
                elif type_name == "EventProp":
                    h = getattr(val, 'handler', None)
                    if hasattr(h, 'fn_name'): props[prop.name] = f"→{h.fn_name}()"
                    elif hasattr(h, 'action_path'): props[prop.name] = f"→{h.action_path}"
                    else: props[prop.name] = "→fn()"
                elif type_name == "ClassProp":
                    props[prop.name] = "{class}"
                elif type_name == "StyleProp":
                    props[prop.name] = "{style}"
                elif type_name == "RefProp":
                    props[prop.name] = f"#ref({getattr(val, 'ref_name', '?')})"
                else:
                    props[prop.name] = f"{{{type_name}}}"

        if hasattr(node, 'test_id') and node.test_id:
            test_id = node.test_id

        children = []
        for child in node.children:
            c = _convert_view_node(child)
            if c:
                children.append(c)

        return ViewTreeNode(
            id=f"el-{node.tag}-{id(node)}",
            tag=node.tag,
            props=props,
            children=children,
            test_id=test_id,
        )

    if isinstance(node, IfDirective):
        container = ViewTreeNode(
            id=f"if-{id(node)}",
            tag="Container",
            props={"data-directive": "@if"},
        )
        for branch in node.branches:
            for c in branch.body:
                child_node = _convert_view_node(c)
                if child_node:
                    container.children.append(child_node)
        return container

    if isinstance(node, ForDirective):
        container = ViewTreeNode(
            id=f"for-{id(node)}",
            tag="Container",
            props={"data-directive": f"@for {node.item_name} in ..."},
        )
        for c in node.body:
            child_node = _convert_view_node(c)
            if child_node:
                container.children.append(child_node)
        return container

    return None
