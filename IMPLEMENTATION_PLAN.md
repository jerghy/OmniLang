# OmniLang v0.1 完整实现计划

## 项目概览

| 属性 | 值 |
|------|-----|
| 语言 | Python 3.10+ |
| 解析库 | lark-parser |
| AST | Python dataclass |
| 目标语言 | Rust, Dart, JavaScript, Python |
| CLI 命令 | `omnic build <file.omni> --target <lang>` |

---

## Phase 0 — 项目骨架搭建

### 0.1 目录结构

```
OmniLang/
├── omnilang/
│   ├── __init__.py
│   ├── cli.py                    # CLI 入口 (argparse)
│   ├── grammar/
│   │   └── omnilang.lark         # EBNF 语法定义
│   ├── lexer.py                  # lark-parser 封装
│   ├── parser.py                 # parse tree → AST 转换
│   ├── ast_nodes.py              # 所有 AST 节点 dataclass
│   ├── semantic/
│   │   ├── __init__.py
│   │   ├── resolver.py           # 符号解析 + 作用域构建
│   │   ├── type_checker.py       # 类型标注检查 + 类型推导 (for-in)
│   │   ├── shadow_checker.py     # 变量遮蔽检测 + 重命名计划
│   │   ├── restrict_checker.py   # 禁止规则检查 (浮点除/取模, _前缀, 参数变异等)
│   │   └── main_checker.py       # main 函数存在性及规则检查
│   ├── codegen/
│   │   ├── __init__.py
│   │   ├── base.py               # 公共代码生成逻辑
│   │   ├── rust.py               # Rust 代码生成器
│   │   ├── dart.py               # Dart 代码生成器
│   │   ├── javascript.py         # JavaScript 代码生成器
│   │   └── python.py             # Python 代码生成器
│   └── utils.py                  # 错误报告、名称生成等工具
├── tests/
│   ├── fixtures/                 # OmniLang 测试用例源码
│   │   ├── 01_hello_world.omni
│   │   ├── 02_variables.omni
│   │   ├── 03_arithmetic.omni
│   │   ├── 04_strings.omni
│   │   ├── 05_arrays.omni
│   │   ├── 06_maps.omni
│   │   ├── 07_control_flow.omni
│   │   ├── 08_functions.omni
│   │   ├── 09_shadowing.omni
│   │   ├── 10_ownership.omni
│   │   ├── 11_for_loop.omni
│   │   ├── 12_while_loop.omni
│   │   ├── 13_if_expression.omni
│   │   ├── 14_type_system.omni
│   │   ├── 15_comprehensive.omni
│   │   ├── 16_tail_return.omni
│   │   ├── 17_ownership_complex.omni
│   │   ├── 18_rust_temp_var.omni
│   │   ├── 19_map_key_variants.omni
│   │   └── 20_no_idiv.omni
│   ├── expected/                 # 每个用例 + 每个目标的预期输出
│   │   ├── 01_hello_world.txt
│   │   ├── 02_variables.txt
│   │   └── ...
│   ├── test_compiler.py          # 编译正确性测试
│   ├── test_semantic.py          # 语义检查错误测试
│   ├── test_e2e.py               # 端到端：编译 + 运行 + 输出比对
│   └── conftest.py               # pytest fixtures
├── pyproject.toml                # 项目配置 + 依赖
├── README.md
└── IMPLEMENTATION_PLAN.md        # 本文件
```

### 0.2 依赖 (`pyproject.toml`)

```toml
[project]
name = "omnilang"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "lark-parser>=1.1",
]

[project.optional-dependencies]
dev = [
    "pytest>=7",
    "pytest-subprocess",  # 用于 e2e 测试中管理子进程
]

[project.scripts]
omnic = "omnilang.cli:main"
```

### 0.3 任务清单

- [ ] 创建完整目录结构
- [ ] 编写 `pyproject.toml`
- [ ] 创建 `omnilang/__init__.py`
- [ ] 创建 `cli.py` 骨架（argparse，打印 "Hello OmniLang"）

---

## Phase 1 — 词法与语法定义

### 1.1 EBNF 语法 (`omnilang/grammar/omnilang.lark`)

需要定义完整的 OmniLang 语法，包括：

- **顶层结构**：程序由 0..N 个函数定义组成，其中必须包含 `fn main() { ... }`
- **函数定义**：`fn NAME ( params ) -> TYPE { stmts }` 或 `fn main() { stmts }`（无返回类型）
- **参数列表**：`NAME: TYPE` 逗号分隔
- **语句**：变量声明、赋值、if/while/for-in 循环、break/continue、return、表达式语句、print/println 语句、块语句
- **变量声明**：`let [mut] NAME: TYPE = expr;`
- **赋值**：`NAME [index] OP= expr;`（OP 为 `= += -= *= /= %=`）
- **表达式**：按优先级排列——逻辑 or、逻辑 and、比较、加减、乘除取模、一元、后缀（索引/方法调用）、原子（字面量/标识符/括号/if 表达式）
- **字面量**：整数（十进制/十六进制/八进制/二进制）、浮点数、字符串、布尔、数组、字典
- **方法调用**：`expr.METHOD(args)`

### 1.2 AST 节点 (`omnilang/ast_nodes.py`)

所有节点为 frozen dataclass，带 `span`（源码位置）字段：

```python
# 顶层
@dataclass(frozen=True)
class Program:
    functions: list[FunctionDef]
    span: Span

@dataclass(frozen=True)
class FunctionDef:
    name: str
    params: list[Param]
    return_type: TypeAnnotation | None  # main 为 None
    body: Block
    span: Span

@dataclass(frozen=True)
class Param:
    name: str
    type_ann: TypeAnnotation
    span: Span

# 语句
@dataclass(frozen=True)
class Block:
    stmts: list[Stmt]
    trailing_expr: Expr | None   # 块末尾无分号表达式，用作函数返回值
    span: Span

@dataclass(frozen=True)
class VarDecl:
    mutable: bool
    name: str
    type_ann: TypeAnnotation
    init: Expr
    span: Span

@dataclass(frozen=True)
class Assign:
    target: str              # 变量名
    index: Expr | None       # 索引赋值时为索引表达式
    op: str                  # '=' | '+=' | '-=' | '*=' | '/=' | '%='
    value: Expr
    span: Span

@dataclass(frozen=True)
class IfStmt:
    condition: Expr
    then_body: Block
    else_ifs: list[tuple[Expr, Block]]
    else_body: Block | None
    span: Span

@dataclass(frozen=True)
class WhileStmt:
    condition: Expr
    body: Block
    span: Span

@dataclass(frozen=True)
class ForInStmt:
    var_name: str
    iterable: Expr
    body: Block
    span: Span

@dataclass(frozen=True)
class BreakStmt: ...
@dataclass(frozen=True)
class ContinueStmt: ...

@dataclass(frozen=True)
class ReturnStmt:
    value: Expr | None
    span: Span

@dataclass(frozen=True)
class PrintStmt:
    newline: bool            # True = println, False = print
    arg: Expr
    span: Span

@dataclass(frozen=True)
class ExprStmt:
    expr: Expr               # 仅允许赋值类 (x += 1 等副作用)
    span: Span

# 表达式
@dataclass(frozen=True)
class IfExpr:
    condition: Expr
    then_expr: Expr
    else_expr: Expr
    span: Span

@dataclass(frozen=True)
class BinaryOp:
    left: Expr
    op: str
    right: Expr
    span: Span

@dataclass(frozen=True)
class UnaryOp:
    op: str                  # '-' | '!'
    operand: Expr
    span: Span

@dataclass(frozen=True)
class MethodCall:
    object: Expr
    method: str
    args: list[Expr]
    span: Span

@dataclass(frozen=True)
class IndexGet:
    object: Expr
    index: Expr
    span: Span

@dataclass(frozen=True)
class VarRef:
    name: str
    span: Span

@dataclass(frozen=True)
class Literal:
    value: int | float | str | bool
    kind: str                # 'i32' | 'f64' | 'String' | 'bool'
    span: Span

@dataclass(frozen=True)
class ArrayLiteral:
    elements: list[Expr]
    span: Span

@dataclass(frozen=True)
class MapLiteral:
    entries: list[tuple[Expr, Expr]]  # [(key, value), ...]
    span: Span

# 类型
@dataclass(frozen=True)
class TypeAnnotation:
    name: str                # 'i32'|'f64'|'bool'|'String'|'Array'|'Map'
    type_args: list[TypeAnnotation]  # 泛型参数
    span: Span
```

### 1.3 任务清单

- [ ] 编写 EBNF 语法文件，覆盖所有语法结构
- [ ] 验证语法：用 lark 解析器加载语法文件，无冲突
- [ ] 实现所有 AST dataclass 节点
- [ ] 实现 `parser.py`：lark Transformer 将 parse tree 转为 AST
- [ ] 实现 `lexer.py`：封装 lark.Lark，提供便捷接口

---

## Phase 2 — 语义分析器

语义分析器采用多 pass 架构，按依赖顺序执行。

### 2.1 符号解析 (`semantic/resolver.py`)

**职责**：构建作用域树，建立每个变量引用的 target 链接。

- 实现作用域栈（全局 → 函数 → 块（包括 if/while/for-in 体））
- 遍历 AST，注册每次变量声明
- 解析每个 `VarRef`，指向声明节点
- 检测未定义变量引用 → 报错
- 收集所有作用域信息供后续 pass 使用

### 2.2 类型检查器 (`semantic/type_checker.py`)

**职责**：验证所有类型标注一致性，推导 for-in 循环变量类型。

检查项：
- 变量声明类型标注与初始化表达式类型匹配
- 赋值目标与值类型匹配
- 二元/一元运算操作数类型匹配（仅同类型比较，逻辑仅 bool 等）
- 函数返回类型与 return 表达式类型匹配
- 函数调用实参与形参个数及类型匹配
- 索引取值：数组索引必须 `i32`，字典键类型必须与声明一致
- `for-in` 循环变量从 `Array<T>` 推导为 `T`
- 方法调用类型检查（如 `push` 参数类型与容器元素类型一致）
- `print`/`println` 参数必须为 `String`
- `if` 条件必须为 `bool`
- 字符串拼接仅 `String + String`
- 禁止浮点除法（`/` 和 `%` 操作数含 `f64` → 报错）
- 禁止 `void` 类型用于变量、参数、容器元素
- 容器元素类型限制检查（仅 i32/f64/bool/String）
- 容器嵌套禁止的类型标注检查（解析 `TypeAnnotation` 时即拒绝 `Array<Array<...>>`、`Map<..., Array<...>>` 等嵌套写法，比 Phase 2.4 的 AST 遍历检查更早发现错误）

返回类型标注信息表：`{Expr节点ID → TypeAnnotation}`，供代码生成使用。

**内置方法签名表：**

类型检查器需维护一个集中的方法签名映射，用于验证方法调用的参数类型和返回类型，同时也标注每个方法是只读(Read)还是变异(Mut)：

```python
BUILTIN_METHODS = {
    "i32": {
        "to_string": (lambda: ([], "String", "Read")),
    },
    "f64": {
        "to_string": (lambda: ([], "String", "Read")),
    },
    "bool": {
        "to_string": (lambda: ([], "String", "Read")),
    },
    "String": {
        "to_string": (lambda: ([], "String", "Read")),
    },
    "Array": {
        "len":       (lambda T: ([], "i32", "Read")),
        "is_empty":  (lambda T: ([], "bool", "Read")),
        "get":       (lambda T: (["i32"], T, "Read")),
        "push":      (lambda T: ([T], "void", "Mut")),
        "pop":       (lambda T: ([], T, "Mut")),
    },
    "Map": {
        "len":       (lambda K, V: ([], "i32", "Read")),
        "is_empty":  (lambda K, V: ([], "bool", "Read")),
        "get":       (lambda K, V: ([K], V, "Read")),
        "contains":  (lambda K, V: ([K], "bool", "Read")),
        "insert":    (lambda K, V: ([K, V], "void", "Mut")),
    },
}
```

类型检查时：根据 receiver 类型查找方法签名，将泛型参数（T, K, V）替换为实际类型，然后校验实参类型是否匹配、是否在只读上下文中调用变异方法。

### 2.3 变量遮蔽检查器 (`semantic/shadow_checker.py`)

**职责**：检测同级和嵌套变量遮蔽，生成重命名计划。

规则：
- **同级作用域**：禁止同名变量声明（报错）
- **嵌套作用域**（块 / for-in 体内）：允许，但必须记录重命名
  - Dart 方案：`原名_shadow_{唯一ID}`
  - Python 方案：同上
  - Rust/JS：自然遮蔽，无需重命名

输出重命名映射表：`{(作用域层级, 原名) → 新名称}`，供 Dart/Python 代码生成器使用。**注意：`for-in` 循环变量声明本身也纳入遮蔽重命名范围**，若其名称与外部作用域变量同名，与普通嵌套块变量一样生成重命名。

### 2.4 限制规则检查器 (`semantic/restrict_checker.py`)

**职责**：执行所有"禁止"类语义规则。

检查项：
- 标识符不得以下划线 `_` 开头（用户代码）
- 函数参数不可重新赋值（`let` 或赋值）
- 函数参数不可调用变异方法
- `for-in` 体内：禁止修改迭代容器（`push`, `pop`, 索引赋值等）
- `for-in` 体内：禁止对循环变量赋值或调用变异方法
- `for-in` 循环变量只能调规定的只读方法
- `break`/`continue` 必须在循环内
- `return` 必须在函数内
- `void` 函数不能有 `return expr;`（只能 `return;` 或末尾无返回）
- `void` 函数（含 `main`）的 `Block.trailing_expr` 必须为 `None`
- 非 `void` 函数：`return expr;` 和 `trailing_expr` 至少存在一个；两者都有时以 `return` 为准（`trailing_expr` 仅在无显式 return 时作为隐式返回值）
- `main` 函数不能有返回值表达式
- 浮点 `/` 和 `%` 操作（已在 type_checker 中处理，这里做二次确认）
- 容器嵌套禁止（数组元素类型不能是 Array/Map，Map 值类型同理；Map 键类型不能是 f64）

### 2.5 `main` 函数检查器 (`semantic/main_checker.py`)

**职责**：验证程序入口规则。

- 必须存在 `fn main()`（无参）
- `main` 可省略返回类型 → 隐式 `void`
- `main` 中 `return;` 合法，`return expr;` 非法
- `main` 的 `Block.trailing_expr` 必须为 `None`（不能有带返回值的尾表达式）

### 2.6 语义分析入口 (`semantic/__init__.py`)

编排所有 pass，合并结果，返回带标注的 AST。

### 2.7 任务清单

- [ ] 实现作用域栈与符号解析器
- [ ] 实现类型检查器（表达式类型推导 + 类型匹配验证）
- [ ] 实现变量遮蔽检测与重命名计划生成（**含 for-in 循环变量声明本身的重命名**，以及循环体内所有引用的同步替换；Dart/Python 后端均需应用）
- [ ] 实现限制规则检查器
- [ ] 实现 main 函数检查器（**必须检查 main 函数是否存在**，无 main 函数 → 报错；同时检查返回/参数规则）
- [ ] 实现语义分析入口，编排所有 pass
- [ ] 为每个语义错误编写可读的错误消息（含源码位置）

---

## Phase 3 — 代码生成器

### 3.1 公共基类 (`codegen/base.py`)

提供：
- 缩进管理（`indent()` / `dedent()` / `emit(line)`）
- JS 辅助函数按需收集（`require_js_helper(name)` → 最后统一输出）
- Python 辅助函数按需收集（同上）
- 变量重命名查询接口（从重命名计划中查询）
- 辅助函数输出方法

**辅助函数按需生成触发条件：**

| 辅助函数 | 触发条件 |
|---------|---------|
| `_idiv` | 源码中出现整数除法 `/` 或复合赋值 `/=`（操作数为 `i32`） |
| `_irem` | 源码中出现整数取模 `%` 或复合赋值 `%=`（操作数为 `i32`） |
| `_arrlen` | 调用 `arr.len()` |
| `_arrisempty` | 调用 `arr.is_empty()` |
| `_arrget` | 数组索引取值 `arr[i]` 或调用 `arr.get(i)` |
| `_arrset` | 数组索引赋值 `arr[i] = v` |
| `_arrpop` | 调用 `arr.pop()` |
| `_maplen` | 调用 `map.len()` 或 `map.is_empty()` |
| `_mapget` | Map 索引取值 `map[k]` 或调用 `map.get(k)` |
| `_arrClone` | 从变量初始化 `Array`（`let arr2: Array<T> = arr1;`） |
| `_mapClone` | 从变量初始化 `Map`（`let map2: Map<K,V> = map1;`） |

实现方式：每个代码生成器（JS / Python）内部维护一个 `needed_helpers: set[str]`，在生成表达式/语句时按上表标记所需函数；最终在输出文件头/尾统一输出被标记的辅助函数定义。

### 3.2 Rust 代码生成器 (`codegen/rust.py`)

按照 §7.1 规则实现。

**⚠️ 高优先级 — 函数调用实参传递规则（统一临时变量策略）：**

这是 Rust 后端最关键且最容易出错的规则。必须在代码生成器中实现为独立方法（`emit_call_arg(expr, param_type)`），并有专门测试覆盖。

**统一规则：对于非 Copy 类型的实参（String / Array / Map），最终都生成带 `&` 的引用传递。区别仅在于引用的对象：**

| 实参类别 | 示例 | 生成方式 |
|---------|------|---------|
| 局部变量 | `greet(s)`, `process(arr)` | 直接 `&var`：`greet(&s)`, `process(&arr)` |
| 字符串字面量 | `greet("Alice")` | 直接用 `&str` 字面量：`greet("Alice")`（Rust 自动 coerces `&str` → `&String` 的调用点视函数签名而定；若函数签名为 `name: &str` 则直接匹配） |
| 非 Copy 字面量（Array/Map） | `process([1,2])` | 先求值到临时变量再引用：`let _tmp_1 = vec![1, 2]; process(&_tmp_1);` |
| 复合表达式 | `greet("Hello, " + name)` | 先求值到临时变量再引用：`let _tmp_1 = format!("{}{}", "Hello, ", name); greet(&_tmp_1);` |

**Copy 类型实参（i32 / f64 / bool）：直接传值，不加 `&`。**

**临时变量命名规则：** `_tmp_{全局自增ID}`，`_` 前缀 + 用户标识符禁止 `_` 开头（Phase 2.4 检查），确保无冲突。

**代码生成器实现方法签名：**
```python
def emit_call_arg(self, expr: Expr, param_type: TypeAnnotation) -> str:
    """为函数调用生成单个实参的 Rust 代码，自动决定是否加 & 或生成临时变量。"""
```

**验证用例：`08_functions.omni` 中必须包含以下场景：**
- `greet("Alice")` — 字符串字面量直接传
- `greet(s)` — 局部 String 变量 → `greet(&s)`
- `process(arr)` — 局部 Array 变量 → `process(&arr)`
- `process([1,2])` — 数组字面量 → 临时变量 + `&`
- `greet("Hello, " + name)` — 复合表达式 → 临时变量 + `&`
- `add(1, 2)` — Copy 类型不加 `&`

---

**⚠️ 高优先级 — 所有权上下文识别机制：**

代码生成器需要能判断一个表达式是处于"只读"还是"取所有权（需要克隆）"位置。这在 for-in 循环变量、临时表达式等场景中至关重要。

为表达式生成方法提供 `need_ownership: bool` 上下文参数，在以下位置设为 `True`：

| 上下文 | 条件 | 说明 |
|--------|------|------|
| 变量声明初始化 | 右侧为非 Copy 类型 | `let s2: String = s;` → 需克隆（`.to_string()`） |
| 函数调用实参 | 参数为非 Copy 类型 | 生成方式见上文"实参传递规则"（最终传 `&`，变量直接引用，表达式先临时变量再引用）；字面量/临时值通过临时变量获取引用，本质是所有权转移 |
| 容器字面量元素 | 元素为非 Copy 类型 | `[s, t]` 中的 `s`/`t` 需克隆 |
| 变异方法实参 | 实参为非 Copy 类型 | `arr.push(s)` 中的 `s` 需克隆（`clone_arg` 逻辑） |
| `return` 返回值 | 函数返回非 Copy 类型 | `return s;` → 若 `s` 是局部变量，Rust 中直接移动所有权即可（不加克隆）；若 `s` 是参数（借用），则需克隆 |
| 尾表达式 `trailing_expr` | 函数返回非 Copy 类型 | 同上 |

**实现建议：** 在 Rust 代码生成器中封装统一方法 `emit_expr(expr, need_ownership: bool)`，内部根据类型和上下文自动处理 `*` 解引用、`.clone()` / `.to_string()`、临时变量等逻辑，避免在每个语句/表达式生成位置散落判断。

---

**⚠️ 高优先级 — 容器元素取值的克隆细节（Rust 特有）：**

根据设计文档 §6.1，`arr.get(i)` 和索引取值 `arr[i]` 在 Rust 中需要根据元素类型动态生成代码：

| 元素类型 | Rust 生成 |
|---------|----------|
| Copy 类型（i32, f64, bool） | `arr[i as usize]` — 直接按值复制 |
| `String` | `arr[i as usize].to_string()` — 必须显式克隆 |

**实现建议：** 在 Rust 代码生成器中实现专门方法 `emit_element_access(container, index, element_type)`，内部根据 `element_type` 决定是否追加 `.to_string()`，所有索引取值和 `get` 调用统一走此方法，避免遗漏。

**其余规则：**

- [ ] `fn main()` 无返回类型
- [ ] 其他函数显式返回类型映射（`void` → 无返回类型，`i32` → `-> i32` 等）
- [ ] 变量声明：`let` → `let`；`let mut` → `let mut`；容器始终 `let mut`
- [ ] `String` 字面量 → `String::from("...")`
- [ ] 数组字面量 → `vec![...]`
- [ ] 字典字面量 → `HashMap::from([...])`
- [ ] 从变量初始化 String → `.to_string()`；Array/Map → `.clone()`
- [ ] `for-in`：`for item in &arr`；Copy 类型使用值加 `*`；非 Copy 需要所有权时调用克隆
- [ ] 字符串拼接：`+` → `format!("{}{}", a, b)`；`+=` → `s = format!("{}{}", s, t);`
- [ ] 数组索引：`arr[i as usize]`
- [ ] Map 键引用：字面量直接使用，变量加 `&`
- [ ] 方法调用映射（按 §6.1 映射表）
- [ ] `clone_arg()` 逻辑：变量加克隆，临时值移所有权
- [ ] `arr.len()` → `arr.len() as i32`
- [ ] `print`/`println` → `print!`/`println!`
- [ ] 自动 `use std::collections::HashMap;`（出现 Map 时）
- [ ] 变量遮蔽：使用 Rust 自然遮蔽（无特殊处理）

### 3.3 Dart 代码生成器 (`codegen/dart.py`)

按照 §7.2 规则实现：

- [ ] `let` → `final`；`let mut` → `var`
- [ ] 函数参数、循环变量加 `final`
- [ ] 类型映射：`void` → `void`，`i32` → `int`，`f64` → `double`，`bool` → `bool`，`String` → `String`，`Array<T>` → `List<T>`，`Map<K,V>` → `Map<K,V>`
- [ ] 数组字面量：`<T>[...]`
- [ ] 字典字面量：`<K,V>{...}`
- [ ] 从变量初始化容器：`List.of(x)` / `Map.of(x)`
- [ ] 属性访问（无括号）：`arr.length`，`arr.isEmpty`，`map.length`，`map.isEmpty`
- [ ] 方法 `containsKey` 而非 `contains`
- [ ] 整数除法：`a ~/ b`
- [ ] 复合赋值：`~/=` 对应 `/=`；`%=` 不变
- [ ] 索引越界：依赖 Dart 原生异常（`RangeError`）
- [ ] Map 取值 `map[key]` 不存在时抛异常：`(map[k] ?? (throw Exception('Key not found')))`
- [ ] `print`/`println`：`stdout.write(s);` / `stdout.writeln(s);`
- [ ] 变量遮蔽：应用重命名（`原名_shadow_{ID}`）
- [ ] **条件导入**：仅当程序包含输出调用时添加 `import 'dart:io';`
- [ ] 函数定义格式：箭头函数（`=>`）用于简单返回表达式

### 3.4 JavaScript 代码生成器 (`codegen/javascript.py`)

按照 §7.3 规则实现：

- [ ] `let` → `const`；`let mut` → `let`
- [ ] 类型擦除：无类型标注
- [ ] 数组字面量：`[...]`
- [ ] 字典字面量：`new Map([...])`
- [ ] 从变量初始化容器：`arr.slice()` / `new Map(map)`
- [ ] 整数除法：`_idiv(a, b)`
- [ ] 整数取模：`_irem(a, b)`
- [ ] 复合赋值对应调整：`x = _idiv(x, y)` 等
- [ ] 方法调用映射（按 §6.1）：`arr.length`，`map.size`，`map.has(k)`，`map.set(k,v)`，`map.get(k)` 等
- [ ] 索引取值：`_arrget(arr, i)` 带越界检查
- [ ] 索引赋值：`_arrset(arr, i, val)` 带越界检查
- [ ] `map[k]` 取值：用 `_mapget(map, k)` 带键存在检查
- [ ] `print`/`println`：`process.stdout.write(s)` / `console.log(s)`
- [ ] **程序入口**：在文件末尾追加 `main();`
- [ ] 辅助函数按需生成（按完整清单 §6.1 末尾）
- [ ] 变量遮蔽：无需处理（JS `const`/`let` 自然块级作用域）

### 3.5 Python 代码生成器 (`codegen/python.py`)

按照 §7.4 规则实现：

- [ ] `let` / `let mut` → 均普通赋值（无 const）
- [ ] 类型擦除
- [ ] 数组字面量：`[...]`
- [ ] 字典字面量：`{...}`
- [ ] 从变量初始化容器：`list(x)` / `dict(x)`
- [ ] 整数除法：`_idiv(a, b)`
- [ ] 整数取模：`_irem(a, b)`
- [ ] 复合赋值：`x = _idiv(x, y)` 等
- [ ] `len()` → `len(arr)`
- [ ] `is_empty()` → `len(arr) == 0`
- [ ] `push(v)` → `append(v)`
- [ ] `pop()` → `pop()`（越界抛 `IndexError`）
- [ ] Map `contains(k)` → `k in map`
- [ ] Map `get(k)` → `map[k]`（依赖 `KeyError`）
- [ ] Map `insert(k,v)` → `map[k] = v`
- [ ] `print`/`println`：`print(s, end='')` / `print(s)`
- [ ] 变量遮蔽：应用重命名
- [ ] 程序入口：文件末尾生成 `if __name__ == "__main__": main()`
- [ ] Python 辅助函数按需生成（`_idiv`, `_irem`）
- [ ] 缩进：4 空格

### 3.6 代码生成器入口 (`codegen/__init__.py`)

```python
def generate(program: Program, target: str, rename_map: dict, type_map: dict) -> str:
    if target == "rust":
        return RustGenerator().generate(program, rename_map, type_map)
    elif target == "dart":
        return DartGenerator().generate(program, rename_map, type_map)
    elif target == "js":
        return JavaScriptGenerator().generate(program, rename_map, type_map)
    elif target == "python":
        return PythonGenerator().generate(program, rename_map, type_map)
```

### 3.7 任务清单

- [ ] 实现公共基类（缩进、emit、辅助函数管理）
- [ ] 实现 Rust 代码生成器
- [ ] 实现 Dart 代码生成器
- [ ] 实现 JavaScript 代码生成器
- [ ] 实现 Python 代码生成器
- [ ] 实现代码生成器入口（工厂函数）

---

## Phase 4 — CLI 工具

### 4.1 命令接口 (`omnilang/cli.py`)

```
omnic build <file.omni> [--target rust|dart|js|python] [--output <dir>]
```

行为：
1. 读取 `.omni` 源文件
2. 调用 lexer 获取 tokens
3. 调用 parser 生成 AST
4. 调用语义分析器所有 pass
5. 如有错误，打印所有错误（含文件名、行号、列号、消息），以非零退出码退出
6. 如仅指定单个 target，输出到 stdout 或 `--output` 文件
7. 如未指定 target，编译到所有 4 个目标，输出到 `--output` 目录（默认 `./build/`）

文件名规则：
- `file.omni` → `file.rs` / `file.dart` / `file.js` / `file.py`

### 4.2 错误报告格式

```
Error: file.omni:10:15: type mismatch: expected i32, found String
  --> file.omni:10:15
   |
10 | let x: i32 = "hello";
   |               ^^^^^^^
```

### 4.3 任务清单

- [ ] 实现 argparse CLI
- [ ] 实现错误报告格式化
- [ ] 实现完整编译流水线编排
- [ ] 测试 CLI 端到端可用

---

## Phase 5 — 测试体系

### 5.1 测试策略总览

| 测试类型 | 内容 | 工具 |
|---------|------|-----|
| 单元测试 | AST 节点、解析器、各语义 pass、各代码生成器 | pytest |
| 快照测试 | 给定 .omni 源码，验证四个目标代码生成结果 | pytest + 内联断言 |
| 语义错误测试 | 给定有问题的 .omni 源码，验证编译器拒绝并报正确错误 | pytest |
| 端到端测试 | .omni → 编译 → 运行 → 收集输出 → 跨语言比对 | pytest-subprocess |

### 5.2 测试用例清单

每个测试用例是一个 `.omni` 文件，包含一个完整可运行的 OmniLang 程序。

#### 01_hello_world.omni — 基本输出
```
fn main() {
    println("Hello, World!");
    print("no newline");
}
```
预期输出：`Hello, World!\nno newline`

#### 02_variables.omni — 变量声明与赋值
- `let` 不可变变量
- `let mut` 可变变量
- Copy 类型赋值不丢失原变量
- 复合赋值 (`+= -= *=`)
- 非 Copy 克隆行为验证

#### 03_arithmetic.omni — 算术运算
- 整数加减乘除取模
- 浮点加减乘
- 运算符优先级
- 除零 panic（所有目标均须抛异常）
- 负整数除法（向零截断验证）
- 取模与除法关系：`a == (a / b) * b + (a % b)`

#### 04_strings.omni — 字符串操作
- 字符串字面量
- 字符串拼接 `+`
- 字符串 `+=` 复合赋值
- `to_string()` 方法（所有基本类型）
- 转义字符

#### 05_arrays.omni — 数组操作
- 创建 `Array<i32>`、`Array<f64>`、`Array<bool>`、`Array<String>`
- `push` / `pop`
- `len()` / `is_empty()`
- `get(i)` 索引取值
- 索引赋值
- 越界 panic

#### 06_maps.omni — 字典操作
- 创建 `Map<String,i32>`、`Map<i32,String>`、`Map<String,String>`
- `insert` / `get`
- `len()` / `is_empty()`
- `contains`
- 索引取值 / 索引赋值
- 键缺失 panic

#### 07_control_flow.omni — 控制流
- `if` / `else if` / `else`
- `while` 循环（含 `break` / `continue`）
- 嵌套控制流

#### 08_functions.omni — 函数定义与调用
- 多参数函数
- 返回值
- `void` 函数
- Copy 类型参数值传递
- 非 Copy 参数不可变借用验证
- 递归函数（如阶乘）

#### 09_shadowing.omni — 变量遮蔽
- 嵌套块内同名变量
- 验证 Dart/Python 重命名后行为与 Rust/JS 一致
- for-in 循环变量遮蔽

#### 10_ownership.omni — 所有权与克隆
- 非 Copy 变量赋值克隆
- 非 Copy 变量传参克隆
- 容器从变量初始化克隆
- 字面量内元素克隆

#### 11_for_loop.omni — for-in 循环
- `for item in arr` 遍历所有元素
- `break` / `continue` 在 for-in 中
- 循环变量只读验证
- 空数组遍历

#### 12_while_loop.omni — while 循环专项
- 计数器模式
- 条件表达式中的复杂比较
- 嵌套 while

#### 13_if_expression.omni — if 表达式
- `let x: i32 = if cond { 1 } else { 2 };`
- 嵌套 if 表达式
- 与 if 语句的区别验证

#### 14_type_system.omni — 类型系统边界
- 各种类型正确组合
- 所有 `to_string()` 用法
- 类型推导（for-in）验证

#### 15_comprehensive.omni — 综合测试
一个较大程序，结合多种特性，计算有明确结果的任务（如排序、斐波那契数列、词频统计等）。

#### 16_tail_return.omni — 尾表达式返回（新增）
- 非 void 函数通过尾无分号表达式返回值
- void 函数无尾表达式
- `return expr;` 与尾表达式同时存在时以 return 为准
- 验证 Rust 后端不生成多余 `return`，JS/Python 在非 void 尾表达式前生成隐式 `return`

#### 17_ownership_complex.omni — 复杂所有权转移（新增）
- for-in 循环变量（非 Copy）用于初始化同类型变量（触发克隆）
- for-in 循环变量（非 Copy）作为函数实参传入非 Copy 参数
- for-in 循环变量放入容器字面量元素位置
- 验证 Rust 后端在所有权上下文中自动生成 `.clone()` / `.to_string()`

#### 18_rust_temp_var.omni — Rust 临时变量生成验证（新增）
- `"Hello, " + name` 作为函数实参（需临时变量 + `&`）
- `[1, 2]` 字面量作为函数实参（需临时变量 + `&`）
- `{"a": 1}` 字面量作为函数实参（需临时变量 + `&`）
- 多个复合表达式实参 → 多个 `_tmp_1`, `_tmp_2` 互不冲突
- 此用例主要验证 Rust 后端生成的代码可编译且行为一致

#### 19_map_key_variants.omni — Map 键类型变体（新增）
- `Map<String, i32>` 分别使用字符串字面量键和 String 变量键
- `Map<i32, String>` 分别使用整数常量键和 i32 变量键
- 验证 Rust 后端：字面量键不加 `&`，变量键加 `&`
- 验证 Python 后端：i32 键不与 bool 键混淆（bool 是 int 子类）

#### 20_no_idiv.omni — 辅助函数按需生成验证（新增）
- 程序不含整数除法、取模，完全不需要 `_idiv` / `_irem`
- 验证 JS/Python 输出中不包含 `_idiv` / `_irem` 定义
- 对比：`03_arithmetic.omni`（含除法取模）的输出中应包含它们

#### 语义错误测试用例（独立文件）

在 `tests/fixtures/errors/` 下，每个 .omni 文件包含一个应被编译器拒绝的代码：

- `err_var_no_type.omni` — 变量声明缺少类型标注
- `err_var_no_init.omni` — 变量声明缺少初始化
- `err_type_mismatch.omni` — 类型不匹配
- `err_shadow_same_scope.omni` — 同级作用域遮蔽
- `err_underscore_prefix.omni` — 下划线开头标识符
- `err_float_div.omni` — 浮点除法
- `err_float_mod.omni` — 浮点取模
- `err_param_mutate.omni` — 修改函数参数
- `err_for_mutate_container.omni` — for-in 中修改容器
- `err_for_mutate_var.omni` — for-in 中修改循环变量
- `err_main_with_return.omni` — main 有返回值
- `err_print_not_string.omni` — print 参数非 String
- `err_f64_key.omni` — Map 的 key 为 f64
- `err_nested_container.omni` — 容器嵌套
- `err_break_outside_loop.omni` — 循环外 break

### 5.3 端到端测试设计

```
对于每个 .omni 测试用例：
  1. 用 omnic 编译到 4 个目标语言
  2. 运行 Rust 版本：cd build/rs && cargo run
  3. 运行 Dart 版本：dart run build/xx.dart
  4. 运行 JS 版本：node build/xx.js
  5. 运行 Python 版本：python build/xx.py
  6. 收集 4 个进程的 stdout
  7. 断言 4 个输出完全相等
  8. 断言输出与 expected/ 中的预期文件一致
```

### 5.4 任务清单

- [ ] 创建 tests/ 目录结构
- [ ] 编写 20 个正向测试用例 (.omni)
- [ ] 编写每个正向测试用例的预期输出文件
- [ ] 编写 15 个语义错误测试用例
- [ ] 实现 `test_compiler.py`：快照测试验证四个目标代码生成
- [ ] 实现 `test_semantic.py`：验证语义错误被正确检测
- [ ] 实现 `test_e2e.py`：端到端编译-运行-比对框架
- [ ] 实现 `conftest.py`：共享 fixtures

---

## Phase 6 — 集成与最终验证

### 6.1 集成验证

- [ ] 所有正向测试用例通过四目标编译
- [ ] 所有生成的代码可通过各自编译器/解释器运行
- [ ] 所有四目标输出一致
- [ ] 所有语义错误测试用例被正确拒绝

### 6.2 对照设计文档逐项验证

- [ ] §2 词法基础：关键字、标识符规则、字面量、注释、分隔符
- [ ] §3 类型系统：所有类型的参数/局部/返回映射、所有权规则、容器限制
- [ ] §4 表达式与运算符：所有运算符、字面量、索引、if 表达式、print 语句
- [ ] §5 语句与控制流：变量声明/赋值、if、while、for-in（含所有静态约束）、函数定义/调用
- [ ] §6 标准库：所有只读/变异方法的四语言映射
- [ ] §7 转译映射补充规则：各语言特有处理
- [ ] §8 完整示例：对照设计文档中的示例验证输出一致

---

## 实现顺序建议

按依赖关系，推荐的实现顺序：

```
Phase 0 (骨架) → Phase 1 (词法/语法/AST) → Phase 2 (语义分析)
→ Phase 3 (代码生成) → Phase 4 (CLI) → Phase 5 (测试) → Phase 6 (集成验证)
```

代码生成器内部实现顺序建议：**Python → JS → Dart → Rust**（从简到繁）。

---

## 风险提示与实施建议

### 关键风险点

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| Rust 实参引用传递遗漏 | 生成的 Rust 代码无法编译 | 实现为统一方法 `emit_call_arg()`，每个场景添加单元测试 |
| 变量遮蔽重命名不同步 | Dart/Python 输出行为与 Rust/JS 不一致 | 在 Python 后端首先打通重命名流程，验证通过后再做 Dart |
| for-in 循环变量所有权判断错误 | Rust 生成代码编译错误或跨语言行为不一致 | 所有容器元素取值统一走 `emit_element_access()` |
| 尾表达式与 return 语义混淆 | void 函数错误返回了值 | 语义检查器在 Phase 2.4 中严格区分 |
| 辅助函数遗漏/冗余 | JS/Python 运行时报错或代码膨胀 | 每个代码生成器维护 `needed_helpers` set，测试用例 `20_no_idiv.omni` 验证按需生成 |
| 跨语言浮点差异 | f64 比较/运算结果微小不一致（已知局限，但需关注意外放大） | 测试用例中避免依赖浮点精确相等，`println` 输出时统一格式 |

### 实施节奏建议

1. **先跑通 Python 后端 + 重命名流程**：Python 是最简单的目标语言，但变量遮蔽的重命名机制需要提前验证。用 09_shadowing.omni 作为第一个端到端通过的测试用例（不含容器/函数调用，专注验证重命名正确性）。

2. **JS 后端紧随其后**：JS 不需要重命名，但需要辅助函数机制。用 03_arithmetic.omni 验证 `_idiv`/`_irem` 的按需生成。

3. **Dart 后端在重命名稳定后开始**：Dart 同时需要属性访问（`arr.length` 而非 `arr.length()`）、类型标注、条件导入，复杂度居中。

4. **Rust 最后攻坚**：等前三个后端的测试用例输出稳定后，再实现 Rust 后端。此时已有准确的跨语言预期输出作为参照，只需关注 Rust 代码能否编译及输出是否一致。

### CI / 本地验证脚本

建议尽早搭建一个脚本 `scripts/verify_all.py`，对每个正向测试用例执行：

```
omnic build tests/fixtures/NN_xxx.omni --output build/
# 分别运行四个目标，diff 输出
python build/NN_xxx.py       > out/python.txt
node build/NN_xxx.js         > out/js.txt
dart run build/NN_xxx.dart   > out/dart.txt
cd build && cargo run        > out/rust.txt  # 需预先 cargo init
diff out/python.txt out/js.txt && diff out/js.txt out/dart.txt && diff out/dart.txt out/rust.txt
```

一旦出现不一致立即发现，避免到 Phase 6 才集中排查。

---

## 工作量估算

| Phase | 内容 | 预估工时 |
|-------|------|---------|
| Phase 0 | 项目骨架 | 0.5 天 |
| Phase 1 | 词法/语法/AST | 2 天 |
| Phase 2 | 语义分析器 | 2.5 天 |
| Phase 3 | 代码生成器 | 3 天 |
| Phase 4 | CLI | 1 天 |
| Phase 5 | 测试 | 2 天 |
| Phase 6 | 集成验证 | 1 天 |
| **合计** | | **12 天** |
