# Editor .omni 保存格式重设计规格

> 状态：草稿 | 日期：2026-06-21

## 概述

将 Editor 从「编辑器生成代码」翻转到「源代码是真相源，编辑器是可视化交互层」。编辑器元数据嵌入 `.omni` 注释中，使 `.omni` 成为唯一真相源——可版本控制、可多人协作、可手动编辑。

## 核心原则

1. `.omni` 源码 = 唯一真相源
2. 编译器可推导的数据不存注释（tag、props、children、事件绑定）
3. 编辑器元数据嵌入严格的注释格式
4. 格式不对 → 解析失败，直接报错
5. 多文件，每个文件 = 一个 Code Tab

---

## 一、文件头部声明

### 格式

```
// @code-tab type=<logic|event>
```

### 规则

- 必须是文件第一行（空行和普通注释可跳过）
- `type` 只允许 `logic` / `event`
- 每个 `.omni` 文件必须有此声明

### 示例

```omniview
// @code-tab type=logic
import omniview;
```

---

## 二、View 元素画布状态

### 格式

```
// @edit [folded] [render=dom|canvas]
```

### 语义

| flag | 含义 | 默认 |
|---|---|---|
| `folded` | 节点在画布上折叠 | false |
| `render=dom` | 编辑器画布 DOM 渲染 | dom |
| `render=canvas` | 编辑器画布 Canvas 渲染 | — |

### 规则 (P)

| # | 规则 |
|---|---|
| P2 | `// @edit` 下一行必须是 `<` 开头的 XML 元素，中间不许有空行 |
| P3 | `// @edit` 和它的 `<` 元素的缩进必须一致 |
| C4 | View block 内每个 `<` 元素（含嵌套）必须有唯一对应的 `// @edit` |
| F1 | `// @edit` 后面只允许 `folded`、`render=dom`、`render=canvas`，其他 token → 报错 |

### 元素标识

用 Omni 原生 `id="xxx"` 属性（用户可选，编辑器可自动生成）。同一文件内所有 `id="xxx"` 必须唯一 (I1)。

### 示例

```omniview
view {
    // @edit
    <div id="box" class: box>
        // @edit folded
        <h3 id="title">计数器演示</h3>
        // @edit
        <button id="btn-inc" class: btn @onPress={inc}>+</button>
    </div>
}
```

> **注意：** `@if` / `@for` / `@empty` 内元素的 `@edit` 规则将在编辑器开发对应功能后补充。

---

## 三、链（逻辑画布）

### 概念

- **链 (chain)** = 一个函数的可视化形态
- 链活在文件内，跟随它对应的 `pub fn` / `action` / `fn`
- 同文件内链的集合 = 该文件的 Code Tab
- 链可跨文件被 import 调用

### 格式

```
// ═══════════════════════════════════════════════════════
// @chain <name>
//   @node <id> <tag> <attrs...>  pos=x,y
//   @wire <id> <from.port> -> <to.port> [collapsed] [group=g1]
// ═══════════════════════════════════════════════════════
```

### 规则

| # | 规则 |
|---|---|
| P4 | `@chain` 块和后面的函数之间不许有其他声明 |
| P5 | `@node` 和 `@wire` 只能出现在 `@chain` 块内 |
| P6 | `@chain` 块内 `@node` 必须全在 `@wire` 之前 |
| I2 | 同一 `@chain` 内所有 `@node` id 必须唯一 |
| I3 | 同一 `@chain` 内所有 `@wire` id 必须唯一 |
| I4 | `@chain` name 必须与紧随其后的函数名完全一致 |

### @node 节点类型速查

#### 变量类

| tag | 必填属性 | 可选属性 |
|---|---|---|
| `signal` | name, type | init |
| `store` | name | — |
| `computed` | name, deps | — |

#### 数据访问类

| tag | 必填属性 |
|---|---|
| `getVariable` | name |
| `setVariable` | name |

#### 算术类

| tag | 属性 |
|---|---|
| `add` | — |
| `sub` | — |
| `mul` | — |
| `div` | — |

#### 逻辑类

| tag | 必填属性 |
|---|---|
| `compare` | op（==, !=, >, <, >=, <=） |
| `and` | — |
| `or` | — |
| `not` | — |
| `concat` | — |

#### 字面量

| tag | 必填属性 |
|---|---|
| `literal` | value, type |

#### 流程控制

| tag | 必填属性 |
|---|---|
| `branch` | — |
| `sequence` | outputs |
| `forEach` | — |

#### UI 操作

| tag | 必填属性 |
|---|---|
| `updateUIProperty` | target, property |

#### 函数/代码

| tag | 必填属性 | 可选属性 |
|---|---|---|
| `fn` | source | — |
| `customFn` | name, params, return | desc |
| `return` | — | value |

#### @node 通用规则

| # | 规则 |
|---|---|
| C1 | 必填属性必须齐全，缺 → 报错 |
| C2 | `pos=x,y` 必须存在，x/y 为整数 |
| S4 | tag 只允许上表中列出的值 |
| F3 | 属性值含空格时必须用引号包裹 |

### @wire 格式

```
@wire <id> <from_id>.<from_port> -> <to_id>.<to_port> [collapsed] [group=<id>]
```

| # | 规则 |
|---|---|
| C3 | `from.port -> to.port` 缺端点 → 报错 |
| F2 | 箭头必须是 `->` |
| S1 | 引用的 `from_id` / `to_id` 必须在同一 `@chain` 的 `@node` 中存在 |
| S2 | 引用的端口名必须是该节点的合法端口 |

### branch 分支

branch 有 `true` / `false` 两个输出端口 + 一个 `condition` 输入端口，各自独立 wire：

```
@wire w1  n2.value -> n4.condition
@wire w2  n4.true -> n5.value
@wire w3  n4.false -> n6
```

### fn 节点特殊规则

fn 节点无端口声明，但有连接。wire 连到 fn 不写端口名，表示数据流接入/发出：

```
@wire w5  n4.false -> n6       // n6 为 fn 节点
@wire w6  n6 -> n7.value       // fn 输出到 return
```

### 完整示例

```omniview
// @code-tab type=logic
import omniview;

// ═══════════════════════════════════════════════════════
// @chain validate
//   @node n1  getVariable  name=value              pos=50,50
//   @node n2  literal      value=""   type=String  pos=50,150
//   @node n3  compare      op="=="                 pos=250,50
//   @node n4  branch                               pos=450,50
//   @node n5  return       value="error:empty"     pos=650,120
//   @node n6  fn           source="..."            pos=450,250
//   @node n7  return       value="ok"              pos=650,250
//   @wire w1  n1.value -> n3.a
//   @wire w2  n2.value -> n3.b
//   @wire w3  n3.result -> n4.condition
//   @wire w4  n4.true -> n5.value
//   @wire w5  n4.false -> n6
//   @wire w6  n6 -> n7.value
// ═══════════════════════════════════════════════════════
pub fn validate(value: String): String {
    if value == "" {
        return "error:empty";
    }
    // ... generated code
    return "ok";
}
```

---

## 四、自定义组件

用户在编辑器中创建自定义组件 → 编辑器生成独立的 `.omni` 文件，放在 `src/components/`。

```omniview
// @code-tab type=logic
import omniview;

component MyCard {
    prop title: String = "";
    event onAction;

    view {
        // @edit
        <div id="card" class: card>
            // @edit
            <h3 id="card-title">{title}</h3>
            // @edit
            <button id="card-btn" @onPress={() => onAction()}>确认</button>
        </div>
    }
    style {
        .card { ... }
    }
}
```

不再需要 `.omni.json` 中的 `components: ComponentRefEntry[]`。

---

## 五、项目级 UI 状态

**文件：** `.omni-editor.toml`（项目根目录）

```toml
# .omni-editor.toml — 编辑器 UI 状态，不影响编译
[editor]
active_file = "src/main.omni"
active_mode = "design"

[editor.open_files]
files = [
    "src/main.omni",
    "src/lib/validation.omni",
]

[editor.panels]
left_open = true
right_width = 320

[editor.viewports."src/main.omni"]
zoom = 1.0
scroll_x = 0
scroll_y = 100

[editor.viewports."src/lib/validation.omni"]
zoom = 0.8
scroll_x = 0
scroll_y = 0
```

---

## 六、多文件项目结构

```
my-project/
├── .omnilang.toml          ← 编译配置（已有）
├── .omni-editor.toml       ← 编辑器 UI 状态（新增）
└── src/
    ├── main.omni           ← 入口 + App 组件
    ├── lib/
    │   ├── validation.omni  ← pub fn + 链
    │   └── types.omni       ← pub struct + pub type
    ├── stores/
    │   └── TodoStore.omni   ← store + action + 链
    └── components/
        ├── Counter.omni     ← component + view + @edit
        ├── MyCard.omni      ← 自定义组件（编辑器生成）
        └── Form.omni        ← component + fn + 链
```

---

## 七、解析规则完整列表

### 位置规则 (P)

| # | 规则 |
|---|---|
| P1 | `// @code-tab type=...` 必须是文件第一行（空行和普通注释可跳过） |
| P2 | `// @edit` 下一行必须是 `<` 开头的 XML 元素，中间不许有空行 |
| P3 | `// @edit` 和它的 `<` 元素的缩进必须一致 |
| P4 | `@chain` 块和后面的函数之间不许有其他声明（signal、computed、fn、action、prop、event、@store 等） |
| P5 | `@node` 和 `@wire` 只能出现在 `@chain` 块内 |
| P6 | `@chain` 块内 `@node` 必须全在 `@wire` 之前 |

### 标识规则 (I)

| # | 规则 |
|---|---|
| I1 | 同一文件内所有 View 元素 `id="xxx"` 必须唯一 |
| I2 | 同一 `@chain` 内所有 `@node` id 必须唯一 |
| I3 | 同一 `@chain` 内所有 `@wire` id 必须唯一 |
| I4 | `@chain` name 必须与紧随其后的函数名完全一致 |

### 完整性规则 (C)

| # | 规则 |
|---|---|
| C1 | 每种 `@node` tag 的必填属性必须齐全 |
| C2 | `@node` 的 `pos=x,y` 必须存在，x 和 y 必须是整数 |
| C3 | `@wire` 的 `from.port -> to.port` 缺端点 → 报错 |
| C4 | View block 内每个 `<` 元素必须有唯一对应的 `// @edit` |

### 一致性规则 (S)

| # | 规则 |
|---|---|
| S1 | `@wire` 引用的 `from_id` / `to_id` 必须在同一 `@chain` 的 `@node` 中存在 |
| S2 | `@wire` 引用的端口名必须是该节点的合法端口 |
| S3 | `@code-tab type=` 只允许 `logic` / `event` |
| S4 | `@node` tag 只允许 nodeRegistry 中已注册的 tag |
| S5 | `@node` 属性中引用变量名的，变量必须在链内或组件作用域已声明 |

### 格式规则 (F)

| # | 规则 |
|---|---|
| F1 | `// @edit` 后面只允许 `folded`、`render=dom`、`render=canvas` |
| F2 | `@wire` 的箭头必须是 `->` |
| F3 | 属性值含空格时必须用引号包裹 |

---

## 八、从旧 `.omni.json` 迁移

旧项目（`.omni.json` 为真相源）需要一次性迁移：

1. 读取 `.omni.json` → 解析 design canvas + code canvas
2. View 元素 → 在 `.omni` 源码的 `<` 标签前插入 `// @edit`
3. Code 链 → 在对应函数前生成 `@chain` 块
4. 自定义组件 → 生成独立 `.omni` 文件
5. 生成 `.omni-editor.toml`
6. 删除 `.omni.json`

迁移工具：`omnic migrate --from-editor-json <path>`
