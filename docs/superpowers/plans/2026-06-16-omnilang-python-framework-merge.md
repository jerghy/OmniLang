# OmniLang Python 框架合并实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 omnilang + omniview 合并为单一的 omnilang Python 封装框架，删除 ~5000 行死代码，更新所有 import 路径。

**Architecture:** 6 模块扁平结构 — cli → api → rust_bridge → DLL。所有编译走 Rust `omni-compiler`，Python 仅做 CLI/编排/FFI 桥接。

**Tech Stack:** Python 3.10+, ctypes, subprocess, setuptools

---

### Task 1: 删除 omnilang 死代码目录和文件

**Files:**
- Delete: `omnilang/analyze/` (整个目录)
- Delete: `omnilang/grammar/` (整个目录)
- Delete: `omnilang/modules/` (整个目录)
- Delete: `omnilang/lexer.py`
- Delete: `omnilang/utils.py`

- [ ] **Step 1: 删除文件**

```bash
rm -rf omnilang/analyze
rm -rf omnilang/grammar
rm -rf omnilang/modules
rm omnilang/lexer.py
rm omnilang/utils.py
```

- [ ] **Step 2: 验证 omnilang 目录**

Run: `ls -la omnilang/`
Expected: 只剩 `__init__.py`, `cli.py`, `config.py`, `lsp/`

- [ ] **Step 3: Commit**

```bash
git add omnilang/
git commit -m "chore: delete dead code from omnilang (analyze/grammar/modules/lexer/utils)"
```

---

### Task 2: 删除 omniview 死代码和不需要的文件

**Files:**
- Delete: `omniview/examples/` (整个目录)
- Delete: `omniview/fixtures/` (整个目录)
- Delete: `omniview/plugins/` (整个目录)
- Delete: `omniview/modules/` (整个目录)
- Delete: `omniview/tests/` (整个目录)
- Delete: `omniview/formatter.py`
- Delete: `omniview/gen_extern.py`

- [ ] **Step 1: 删除文件**

```bash
rm -rf omniview/examples
rm -rf omniview/fixtures
rm -rf omniview/plugins
rm -rf omniview/modules
rm -rf omniview/tests
rm omniview/formatter.py
rm omniview/gen_extern.py
```

- [ ] **Step 2: 验证 omniview 目录**

Run: `ls -la omniview/`
Expected: 只剩 `__init__.py`, `api.py`, `sourcemap.py`, `codegen/rust_bridge.py`

- [ ] **Step 3: Commit**

```bash
git add omniview/
git commit -m "chore: delete dead code from omniview (examples/fixtures/plugins/modules/formatter/gen-extern)"
```

---

### Task 3: 将 omniview 剩余文件迁入 omnilang

**Files:**
- Move: `omniview/api.py` → `omnilang/api.py`
- Move: `omniview/sourcemap.py` → `omnilang/sourcemap.py`
- Move: `omniview/codegen/rust_bridge.py` → `omnilang/rust_bridge.py`

- [ ] **Step 1: 移动文件**

```bash
mv omniview/api.py omnilang/api.py
mv omniview/sourcemap.py omnilang/sourcemap.py
mv omniview/codegen/rust_bridge.py omnilang/rust_bridge.py
```

- [ ] **Step 2: 删除 omniview 残余**

```bash
rm -rf omniview/codegen
rm omniview/__init__.py
rmdir omniview
```

- [ ] **Step 3: 验证**

Run: `ls omnilang/`
Expected: `__init__.py`, `api.py`, `cli.py`, `config.py`, `rust_bridge.py`, `sourcemap.py`, `lsp/`

- [ ] **Step 4: Commit**

```bash
git add omnilang/ omniview/
git commit -m "refactor: merge omniview files into omnilang, delete omniview directory"
```

---

### Task 4: 更新 rust_bridge.py 内部 import 路径

**Files:**
- Modify: `omnilang/rust_bridge.py` — PATH 常量中的相对路径计算

rust_bridge.py 的 `_load_dll` 方法用 `Path(__file__).parent.parent.parent` 来定位 DLL，现在从 `omniview/codegen/rust_bridge.py` 变为 `omnilang/rust_bridge.py`，路径层级少了两层。

- [ ] **Step 1: 修改 DLL 路径计算**

Read `omnilang/rust_bridge.py:235-237` 当前：

```python
dll_paths = [
    str(Path(__file__).parent.parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll"),
    str(Path("omni-compiler/target/release/omni_compiler.dll")),
]
```

改为：

```python
dll_paths = [
    str(Path(__file__).parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll"),
    str(Path("omni-compiler/target/release/omni_compiler.dll")),
]
```

> 变化：`parent.parent.parent` → `parent.parent`（因为从 `omnilang/codegen/` 变为 `omnilang/`，少了两层目录）

- [ ] **Step 2: 验证路径正确性**

Run:
```bash
python -c "from pathlib import Path; p = Path('omnilang/rust_bridge.py'); print(Path(p).parent.parent / 'omni-compiler' / 'target' / 'release' / 'omni_compiler.dll')"
```
Expected: 输出包含 `omni-compiler/target/release/omni_compiler.dll`

- [ ] **Step 3: Commit**

```bash
git add omnilang/rust_bridge.py
git commit -m "fix: update DLL discovery path after directory flattening"
```

> ⚠️ **Task 3-5 中间态警告：** Task 3 删除了 `omniview/` 目录，Task 4-5 才更新 api.py/rust_bridge.py 的内部 import。Task 3 的 commit 之后到 Task 5 完成之前，Python import 会失败。三个 Task 应连续执行，中间不要运行 Python 或测试。

---

### Task 5: 更新 api.py 内部 import 路径

**Files:**
- Modify: `omnilang/api.py:7` — docstring 示例
- Modify: `omnilang/api.py:105` — RustBridge import
- Modify: `omnilang/api.py:214` — RustBridge import
- Modify: `omnilang/api.py:358` — RustBridge import

- [ ] **Step 1: 更新 docstring 示例**

`omnilang/api.py:7` 当前：

```python
#    from omniview.api import compile_omni, CompileResult, CompileError
```

改为：

```python
#    from omnilang.api import compile_omni, CompileResult, CompileError
```

- [ ] **Step 2: 更新 RustBridge import（3 处相同变更）**

`omnilang/api.py:105`、`:214`、`:358` 当前：

```python
from omniview.codegen.rust_bridge import RustBridge
```

全部改为：

```python
from omnilang.rust_bridge import RustBridge
```

Run to verify no other `from omniview` remains in api.py:
```bash
grep "from omniview" omnilang/api.py
```
Expected: 无输出

- [ ] **Step 3: Commit**

```bash
git add omnilang/api.py
git commit -m "refactor: update api.py internal imports after merge"
```

---

### Task 6: 更新 cli.py — 删除 gen-extern + 更新 import 路径

**Files:**
- Modify: `omnilang/cli.py` — 5 处 import 路径 + 删除 gen-extern 代码

- [ ] **Step 1: 更新 4 处 `from omniview.api import compile_omni`**

`omnilang/cli.py:70`、`:123`、`:237`、`:430` 当前：

```python
from omniview.api import compile_omni
```

全部改为：

```python
from omnilang.api import compile_omni
```

- [ ] **Step 2: 更新 1 处 `from omniview.codegen.rust_bridge import RustBridge`**

`omnilang/cli.py:130` 当前：

```python
from omniview.codegen.rust_bridge import RustBridge
```

改为：

```python
from omnilang.rust_bridge import RustBridge
```

- [ ] **Step 3: 删除 `cmd_gen_extern` 函数**

删除 `omnilang/cli.py:507-517`：

```python
def cmd_gen_extern(args):
    """Generate Rust extern declarations from a .d.ts file."""
    from omniview.gen_extern import gen_extern_command
    # ...
    sys.exit(gen_extern_command(raw_args))
```

- [ ] **Step 4: 从 main() 中删除 gen-extern 子命令注册**

删除 `omnilang/cli.py:627-636`（gen_extern_parser 定义）和 `:656-657`（dispatch 分支）：

```python
# 删除 argparser 定义 (line 627-636)
gen_extern_parser = subparsers.add_parser(
    "gen-extern", help="Generate Rust extern declarations from a .d.ts file"
)
# ... 

# 删除 dispatch 分支 (line 656-657)
elif args.command == "gen-extern":
    cmd_gen_extern(args)
```

- [ ] **Step 5: 验证 CLI 命令列表**

Run:
```bash
python -c "from omnilang.cli import main; print('import ok')"
```
Expected: `import ok`（无 ImportError）

- [ ] **Step 6: Commit**

```bash
git add omnilang/cli.py
git commit -m "refactor: update cli.py imports, remove gen-extern command"
```

---

### Task 7: 更新 editor/server/main.py 的 import 路径

**Files:**
- Modify: `editor/server/main.py:245` — `from omniview.api import compile_omni`
- Modify: `editor/server/main.py:280` — `from omniview.codegen.rust_bridge import RustBridge`
- Modify: `editor/server/main.py:373` — `from omniview.api import parse_view_tree`
- Modify: `editor/server/main.py:394` — `from omniview.api import parse_code_blocks`
- Modify: `editor/server/main.py:611` — `from omniview.api import parse_native_ports`

- [ ] **Step 1: 批量替换 import 路径**

使用 `sed` 或手动替换以下 5 处：

| 行号 | 旧 | 新 |
|------|----|----|
| 245 | `from omniview.api import compile_omni` | `from omnilang.api import compile_omni` |
| 280 | `from omniview.codegen.rust_bridge import RustBridge` | `from omnilang.rust_bridge import RustBridge` |
| 373 | `from omniview.api import parse_view_tree` | `from omnilang.api import parse_view_tree` |
| 394 | `from omniview.api import parse_code_blocks` | `from omnilang.api import parse_code_blocks` |
| 611 | `from omniview.api import parse_native_ports` | `from omnilang.api import parse_native_ports` |

Run to verify:
```bash
grep "from omniview" editor/server/main.py
```
Expected: 无输出

- [ ] **Step 2: Commit**

```bash
git add editor/server/main.py
git commit -m "refactor: update editor main.py imports to omnilang"
```

---

### Task 8: 更新 editor 其他文件的 import 路径

**Files:**
- Modify: `editor/server/api/watch.py:87` — `from omniview.api import compile_omni` → `from omnilang.api import compile_omni`
- Modify: `editor/server/api/code_ownership.py:191` — `from omniview.api import parse_code_blocks` → `from omnilang.api import parse_code_blocks`
- Modify: `editor/server/omni_json/exporter.py:50` — `from omniview.api import parse_view_tree, parse_code_blocks` → `from omnilang.api import parse_view_tree, parse_code_blocks`
- Modify: `editor/server/omni_json/importer.py:457-461` — 删除 `_format_source` 方法中的 formatter 调用，直接返回未格式化的 source

- [ ] **Step 1: 更新 watch.py**

`editor/server/api/watch.py:87`：

```python
        from omnilang.api import compile_omni
```

- [ ] **Step 2: 更新 code_ownership.py**

`editor/server/api/code_ownership.py:191`：

```python
    from omnilang.api import parse_code_blocks
```

- [ ] **Step 3: 更新 exporter.py**

`editor/server/api/omni_json/exporter.py:50`：

```python
        from omnilang.api import parse_view_tree, parse_code_blocks
```

- [ ] **Step 4: 修复 importer.py（删除 formatter 依赖）**

`editor/server/omni_json/importer.py:456-461` 当前：

```python
    @staticmethod
    def _format_source(source: str) -> str:
        """Pass reconstructed source through OmniViewFormatter for canonical output."""
        from omniview.formatter import OmniViewFormatter
        formatter = OmniViewFormatter()
        return formatter.format(source)
```

改为直接返回 source：

```python
    @staticmethod
    def _format_source(source: str) -> str:
        """Return source as-is. (Formatter removed — to be re-added post-merge.)"""
        return source
```

- [ ] **Step 5: 验证所有 editor import 路径**

```bash
grep -r "from omniview" editor/
```
Expected: 无输出（除了注释/字符串外的 import 语句）

- [ ] **Step 6: Commit**

```bash
git add editor/
git commit -m "refactor: update all editor imports to omnilang, remove formatter usage"
```

---

### Task 9: 更新测试文件的 import 路径

**Files:**
- Modify: `tests/regression/test_rust_vs_python_codegen.py:15-16`
- Delete: `tests/unit/test_modules.py`
- Modify: `editor/server/omni_json/tests/test_roundtrip.py:13`

- [ ] **Step 1: 更新 regression 测试 import**

`tests/regression/test_rust_vs_python_codegen.py:15-16` 当前：

```python
from omniview.api import compile_omni
from omniview.codegen.rust_bridge import RustBridge
```

改为：

```python
from omnilang.api import compile_omni
from omnilang.rust_bridge import RustBridge
```

- [ ] **Step 2: 删除 test_modules.py**

```bash
rm tests/unit/test_modules.py
```

- [ ] **Step 3: 更新 roundtrip 测试 import**

`editor/server/omni_json/tests/test_roundtrip.py:13` 当前：

```python
from omniview.api import compile_omni
```

改为：

```python
from omnilang.api import compile_omni
```

- [ ] **Step 4: Commit**

```bash
git add tests/
git commit -m "test: update test imports to omnilang, delete test_modules"
```

---

### Task 10: 更新 pyproject.toml + omnilang/__init__.py

**Files:**
- Modify: `pyproject.toml:24`
- Modify: `omnilang/__init__.py`

- [ ] **Step 1: 更新 pyproject.toml packages.find**

`pyproject.toml:24` 当前：

```toml
include = ["omnilang*", "omniview*", "editor*"]
```

改为：

```toml
include = ["omnilang*", "editor*"]
```

- [ ] **Step 2: 更新 omnilang/__init__.py 添加公开 API**

`omnilang/__init__.py` 当前：

```python
"""OmniLang — a minimal, strongly-typed, imperative language."""

__version__ = "0.1.0"
```

改为：

```python
"""OmniLang — compile .omni to React, Vue, Python, Rust, and more.

Usage:
    from omnilang import compile_omni, CompileResult, CompileError
    result = compile_omni(source, target="react")
"""

__version__ = "0.2.0"

from omnilang.api import compile_omni, CompileResult, CompileError, parse_view_tree, parse_code_blocks, parse_native_ports
from omnilang.rust_bridge import RustBridge

__all__ = [
    "compile_omni",
    "CompileResult",
    "CompileError",
    "RustBridge",
    "parse_view_tree",
    "parse_code_blocks",
    "parse_native_ports",
]
```

- [ ] **Step 3: Commit**

```bash
git add pyproject.toml omnilang/__init__.py
git commit -m "refactor: update pyproject.toml, add public API re-exports"
```

---

### Task 11: 端到端验证

- [ ] **Step 1: 安装包**

```bash
pip install -e .
```
Expected: 无错误

- [ ] **Step 2: 验证 CLI 帮助**

```bash
omnic --help
```
Expected: 显示 8 个命令（build, check, fix, run, init, fmt, serve, watch），无 gen-extern

- [ ] **Step 3: 验证 import**

```bash
python -c "from omnilang import compile_omni, CompileResult, CompileError, RustBridge; print('OK')"
```
Expected: `OK`

- [ ] **Step 4: 运行回归测试**

```bash
pytest tests/regression/test_rust_vs_python_codegen.py -v
```
Expected: 取决于 Rust DLL 是否可用。如果 DLL 不可用，测试应 skip（require_rust marker），不 crash。

- [ ] **Step 5: 验证 editor 可导入**

```bash
python -c "from editor.server.main import app; print('OK')"
```
Expected: `OK`（无 ImportError）。可能需要 `pip install fastapi uvicorn`。

- [ ] **Step 6: 运行全部测试**

```bash
pytest tests/ -v --ignore=tests/unit/test_modules.py
```

- [ ] **Step 7: 最终确认无 omniview 残留引用**

```bash
grep -r "from omniview" . --include="*.py" | grep -v __pycache__ | grep -v ".git/"
```
Expected: 无输出

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: final cleanup after merge verification"
```

---

## 自审

1. **Spec 覆盖率:** ✅ 每个 spec 验收标准有对应 Task
2. **无占位符:** ✅ 无 TBD/TODO，每步有具体代码
3. **类型一致性:** ✅ import 路径在所有 Task 中一致使用 `omnilang.*`
4. **sourcemap.py:** ✅ 无内部 import 依赖，迁移后不需修改

## 已知遗留问题

- **`_detect_ui_mode()` 重复：** `cli.py:524` 和 `api.py:59` 各有一份相同实现。合并后两份代码在同一个包内。本次不动（spec 约束"不新增功能"），后续可统一为 cli 直接调 `api._detect_ui_mode()`，删 cli 内的副本。

## 执行顺序

Tasks 1-2（删除）可并行。Tasks 3-10 按序执行（有 import 路径依赖）。Task 11 最后。
