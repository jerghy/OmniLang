# OmniLang Python 框架合并设计

> **状态：** 已批准
> **日期：** 2026-06-16
> **目标：** 将 omnilang + omniview 两个 Python 目录合并为单一的 omnilang 封装框架

---

## 背景

commit `0a8f9d37` 删除了 Python parser/codegen/preprocessor/semantic，Rust `omni-compiler` 成为唯一编译实现。原本 omnilang 和 omniview 各司其职的理由（两套独立编译管线）已不成立。当前两个目录都是 Rust 编译器的薄封装层，合计 ~8000 行 Python，其中 ~5000 行是孤立死代码。

## 目标架构

```
omnilang/                    # 单一 Python 封装框架
├── __init__.py              # 公开 API re-export
├── cli.py                   # omnic CLI 入口
├── config.py                # .omni.toml 项目管理
├── api.py                   # compile_omni() 编译 API
├── rust_bridge.py           # Rust DLL FFI + CLI fallback
├── sourcemap.py             # SourceMap 查询工具
└── lsp/
    ├── __init__.py
    └── server.py            # 保持不动（后续重写）
```

## 数据流

```
omnic build file.omni --target react
  │
  └→ cli.py:cmd_build()
       ├─ config.py → 加载项目配置
       └─ api.py:compile_omni(source, target)
            ├─ _detect_ui_mode() → 自动检测 UI/纯逻辑
            ├─ target 校验 + 纯逻辑回退
            └─ rust_bridge.py:RustBridge.compile_to_target()
                 ├─ 优先: ctypes CDLL → omni_compiler.dll
                 └─ 备用: subprocess → omni-compiler CLI
                      └→ Rust: Lex→Parse→Semantic→IR→Codegen
```

## 删除清单

| 路径 | 原因 |
|------|------|
| `omnilang/analyze/` | 40+ lint 规则，零外部调用，Rust compiler 已有独立 lint 框架 |
| `omnilang/grammar/` | .lark.bak + .scm 遗留文件 |
| `omnilang/lexer.py` | 引用不存在文件，零调用 |
| `omnilang/utils.py` | Span/ErrorCollector，零外部引用 |
| `omnilang/modules/` | resolver/graph/loader，Rust `compile_multi_to_target` 替代 |
| `omniview/formatter.py` | editor importer 唯一调用方，同步删除 |
| `omniview/gen_extern.py` | 过渡期工具，不再需要 |
| `omniview/modules/` | loader，已被 Rust 替代 |
| `omniview/examples/` | .omni 样本文件 |
| `omniview/fixtures/` | 测试固件 |
| `omniview/plugins/` | npm/cargo 插件包 |
| `omniview/tests/` | test_rust_bridge（可迁入 tests/） |

## Import 路径变更

```
旧                                  → 新
omniview.api.compile_omni          → omnilang.api.compile_omni
omniview.codegen.rust_bridge       → omnilang.rust_bridge
omniview.sourcemap                 → omnilang.sourcemap
omniview.formatter                 → 删除（editor 同步修）
omniview.gen_extern                → 删除
omnilang.modules                   → 删除
```

## 受影响的外部文件

1. `editor/server/main.py` — 3 处 import 路径更新
2. `editor/server/api/watch.py` — 1 处
3. `editor/server/api/code_ownership.py` — 1 处
4. `editor/server/omni_json/exporter.py` — 1 处
5. `editor/server/omni_json/importer.py` — 删除 formatter 调用
6. `tests/regression/test_rust_vs_python_codegen.py` — 2 处
7. `tests/unit/test_modules.py` — 删除（测试已删除的 modules）

## CLI 命令变化

| 命令 | 状态 |
|------|------|
| `build` | 保留 |
| `check` | 保留 |
| `run` | 保留 |
| `init` | 保留 |
| `fmt` | 保留（删除内联 `_format_omni`，与 formatter.py 对齐）|
| `serve` | 保留 |
| `watch` | 保留 |
| `fix` | 保留占位 |
| `gen-extern` | **删除** |

## pyproject.toml

```toml
[tool.setuptools.packages.find]
include = ["omnilang*", "editor*"]  # 去掉 "omniview*"
```

## 验收标准

- [ ] omnilang/ 目录只含上述 6 个模块 + lsp
- [ ] omniview/ 目录完全删除
- [ ] `omnic build` 全部 target 正常工作（通过 DLL + CLI 两种路径）
- [ ] 7 个外部文件的 import 路径更新
- [ ] 所有现有测试通过（编辑受影响测试）
- [ ] `pip install -e .` 成功
- [ ] `omnic --help` 显示正确的 8 个命令

## 非目标

- 不修改任何 Rust 编译器代码
- 不修改 LSP server 逻辑
- 不改动 editor 功能行为（只修 import 路径）
- 不新增任何功能
