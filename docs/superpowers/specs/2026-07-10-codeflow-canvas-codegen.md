# CodeFlowCanvas 代码生成设计

> 日期：2026-07-10
> 状态：待实现
> 范围：组件标签页内的 CodeFlowCanvas，单组件代码流

## 一、目标

将 CodeFlowCanvas 从纯视觉占位节点升级为能真正生成 OmniView 代码的可视化编辑器。用户在画布上用节点编排函数体逻辑，保存时自动生成与 learn-omniview 同质量的 `.omni` 源码。

## 二、节点体系

### 2.1 节点类型（4 种）

| 节点 | tag | 作用 | 连接点 |
|------|-----|------|--------|
| fn | `fn` | 函数定义，整个图的入口 | 1 个出口 ● |
| code-block | `code-block` | 自由 OmniView 代码文本 | 1 入口 ● + 1 出口 ● |
| condition | `condition` | 组合判断 if/elif/else | 1 入口 ● + N 个分支出口 ● |
| return | `return` | 返回语句，图终点 | 1 个入口 ● |

### 2.2 删除的节点

删除以下声明类和运算类节点（不在画布上表示）：

`signal`, `computed`, `store`, `let`, `const`, `literal`, `getVariable`, `setVariable`, `add`, `sub`, `mul`, `div`, `compare`, `and`, `or`, `not`, `concat`, `customFn`

这些组件成员声明（signal、computed、prop、event）在左侧面板中配置。

### 2.3 待定节点

`for`、`while` 循环节点复杂度较高，本期不做，后续迭代。

## 三、节点详情

### 3.1 fn 节点

- **外观**：紫色圆角顶栏，显示 `ƒ fnName(params)`
- **交互**：双击编辑函数名和参数列表
- **端口**：1 个 flow-out

### 3.2 condition 节点

- **外观**：黄色边框，每行一个分支，右侧各有一个连接点
- **交互**：节点内有 + 按钮添加 elif/else 分支，点击条件文本可编辑
- **端口**：1 个 flow-in（入口），每个分支一个 flow-out（用 branchId 标识）

```
        ┌─ if: count > 10  ●──────
[判断]  ├─ elif: count > 5 ●──────
        └─ else:           ●──────
```

### 3.3 code-block 节点

- **外观**：灰色 monospace 卡片，显示代码预览（首行前 50 字符截断）
- **交互**：双击弹出内联编辑器，输入任意 OmniView 代码
- **端口**：1 个 flow-in + 1 个 flow-out

### 3.4 return 节点

- **外观**：橙色圆角底栏，显示 `return expr`
- **交互**：双击编辑返回值表达式
- **端口**：1 个 flow-in（无出口，图终点）

## 四、连线语义

Wires 表示**纯控制流**：连线 = "A 执行完之后执行 B"。

连接规则：
```
fn  ─→  code-block  ─→  condition  ─┬─→  code-block  ─→  return
                                    ├─→  code-block  ─→  return
                                    └─→  ...
```

## 五、数据模型

### 5.1 JSON Store（只存布局和连线）

代码内容全部从 `.omni` 源码解析得到，Store 只存位置和拓扑：

```typescript
{
  codeFlowNodes: [
    { pid: "fn_a1",   position: { x: 100, y: 100 } },
    { pid: "cb_b2",   position: { x: 100, y: 220 } },
    { pid: "cond_c3", position: { x: 100, y: 340 } },
  ],
  codeFlowWires: [
    { fromPid: "fn_a1",   toPid: "cb_b2",   fromPort: "flow-out", toPort: "flow-in" },
    { fromPid: "cb_b2",   toPid: "cond_c3", fromPort: "flow-out", toPort: "flow-in" },
    { fromPid: "cond_c3", toPid: "cb_d4",   fromPort: "br1",      toPort: "flow-in" },
    { fromPid: "cond_c3", toPid: "cb_e5",   fromPort: "br2",      toPort: "flow-in" },
  ]
}
```

### 5.2 .omni 源码（存所有代码内容）

```omni
// @pid:c1
component Counter {
    // 信号、prop、event 由解析器解析，不由 code flow 管理
    signal count: i32 = 0;

    // @pid:fn_a1
    fn increment(step: i32) {
        // @pid:cb_b2
        count = count + step;
        // @pid:cond_c3
        if count > 10 {
            // @pid:cb_d4
            count = 0;
        } else {
            // @pid:cb_e5
            count = count + 1;
        }
        // @pid:ret_f6
        return count;
    }

    view { ... }
    style { ... }
}
```

### 5.3 PID 规则

每个 code flow 节点对应一个 `// @pid:xxx` 注释，保证往返解析稳定性。

## 六、代码生成（GraphCompiler）

### 6.1 触发时机

保存时触发（Ctrl+S 或切换标签页），不走实时编译。DesignPreview 只管样式预览，不响应代码变更。

### 6.2 编译算法

```
输入：code flow 图 (nodes + wires)
输出：fn body 代码字符串

1. 找到 fn 节点作为入口
2. 生成 "fn name(params) {"
3. 沿 flow-out 连线递归遍历后继节点：
   - code-block → 输出 source 文本行
   - condition  → 输出 "if cond {" + 编译分支子链 + "} elif cond {...} else {...}"
   - return     → 输出 "return expr;"
4. 生成 "}"
5. 注入到 component 的完整 .omni 源码中
```

### 6.3 保存流程

```
CodeFlowCanvas 图
    ↓ GraphCompiler
  fn body 代码字符串
    ↓ 嵌入 component 源码
  完整 component 源码
    ↓ 现有 source-emitter（添加 // @pid:xxx 注释）
  .omni 文件写入磁盘
    ↓
JSON Store 更新 positions + wires
```

## 七、代码解析（GraphParser）

### 7.1 解析流程

```
parser 读取 .omni 文件
    ↓ 识别 component { ... } 块
    ↓ 识别 fn name(params) { ... } 函数体
    ↓ GraphParser 解析函数体：
        - 按 // @pid:xxx 注释切分节点
        - fn name(params) { → fn 节点
        - 普通语句 → code-block 节点（code = 语句文本）
        - if/elif/else → condition 节点 + 分支解析
        - return → return 节点
    ↓ 根据嵌套关系重建 wires
    ↓ 从 JSON Store 加载 positions
  CodeFlowCanvas 图
```

## 八、UI 变更

### 8.1 右键菜单

```
控制流:
  - 函数定义 (fn)
  - 判断 (condition)
  - 代码块 (code-block)
  - 返回 (return)
```

### 8.2 节点交互

- **双击 code-block**：弹出内联编辑器输入 OmniView 代码
- **双击 fn**：编辑函数名和参数
- **双击 return**：编辑返回值表达式
- **condition 节点内**：+ 按钮添加 elif/else 分支，点击条件文本编辑
- **所有节点**：右键删除

## 九、实现顺序

1. **数据层**：更新 `CodeblockElementData`、`flowConstants`、Store types —— 新增 condition tag，删除废弃 tag
2. **节点 UI**：重写 4 种节点组件 + condition 分支编辑器
3. **GraphCompiler**：图 → .omni 源码字符串（fn body）
4. **GraphParser**：.omni 源码 → 图（nodes + wires）
5. **保存集成**：挂到现有保存流程，emit 时注入 fn body
6. **解析集成**：挂到现有 parse 流程，parse 后重建 code flow 图
7. **清理**：删除旧节点组件、旧 tag 常量、旧菜单项

## 十、不做

- ❌ for / while 节点（后续迭代）
- ❌ 实时编译（保存时触发）
- ❌ DesignPreview 联动代码变更（DesignPreview 只管样式）
- ❌ 多文件项目管理（本次只做单组件内 code flow）
- ❌ 闭包节点（由 code-block 覆盖）
- ❌ signal/computed/store 等声明节点（在左侧面板配置）
