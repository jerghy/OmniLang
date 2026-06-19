# Learn OmniView Phase 8 — 编译器新特性演示

**Date:** 2026-06-19
**Status:** Design approved, pending implementation
**Depends on:** Phase 1-7 complete (21 files, ~89 features, 410 tests)

## 目标

在 `test/projects/learn-omniview` 中新增 4 个源文件，展示 Phase 1-7 完成后编译器新增的 6 项核心特性：

| 特性 | 语法 | 编译器支持 |
|------|------|-----------|
| `match` 表达式 | `match val { Pattern { b } => { ... }, _ => { ... } }` | parser + IR + JS codegen |
| `await` 表达式 | `await async_call()` | parser + IR + 8 后端 |
| Variant 构造 | `UnionType.Variant(fields)` | VariantLiteral IR + 9 后端 |
| HTTP stdlib | `import { http_get } from "std/http"` | 全局注册表 + 按目标生成 |
| JSON stdlib | `import { json_parse } from "std/json"` | 全局注册表 + 按目标生成 |
| Storage stdlib | `import { storage_get } from "std/storage"` | JS/Flutter 目标 + 按目标生成 |

## 文件规划

### 1. `src/lib/api.omni` — API 类型 + 工具函数

纯逻辑文件，定义共享类型和封装函数。

**覆盖特性：**
- `tagged union` — 定义 `ApiResult<T>` = `Ok { data: T }` | `Err { error: String }` | `Loading`
- `variant 构造` — `ApiResult.Ok(data)` / `ApiResult.Err(msg)` / `ApiResult.Loading`
- `match` 表达式 — `unwrap_or_default()` 函数中模式匹配 3 个变体
- `async fn` — 封装 `fetch_json(url)` 使用 await + http_get + json_parse
- `pub fn` — 导出供其他文件使用

**核心函数：**
```
pub fn unwrap_or_default(result: ApiResult<i32>, default: i32) -> i32
pub async fn fetch_json(url: String) -> ApiResult<String>
```

### 2. `src/pages/DataFetching.omni` — 远程数据加载

组件形式的 GET 请求演示页面。

**覆盖特性：**
- `import { http_get } from "std/http"` — stdlib 导入语法
- `await` — 在 async fn 中 await http_get 结果
- `json_parse` — 解析 API 返回的 JSON
- `match` — 处理 ApiResult 的 3 种状态（Ok/Err/Loading），驱动 UI 渲染
- `@if` / `@else if` — 按加载状态显示不同 UI（spinner / 数据 / 错误）

**组件结构：**
```
component DataFetching {
    @store NotificationStore;
    signal status: String = "idle";   // idle | loading | success | error
    signal resultText: String = "";
    signal errorMsg: String = "";

    action async loadData(url: String) { ... }
    view { ... }
}
```

### 3. `src/pages/DataSubmit.omni` — 表单提交

POST 请求 + JSON 序列化的表单演示。

**覆盖特性：**
- `json_stringify` — 将表单数据序列化为 JSON 字符串
- `http_post` — POST 到远程端点（async + await）
- `match` — 处理提交响应（成功/失败/网络错误）
- `@onSubmit` — 表单提交事件
- `@onChange` — 输入框双向绑定

**组件结构：**
```
component DataSubmit {
    @store NotificationStore;
    signal title: String = "";
    signal body: String = "";
    signal submitStatus: String = "";

    action async submitForm() { ... }
    view { ... }
}
```

### 4. `src/pages/UserSettings.omni` — 本地偏好持久化

纯客户端 localStorage 读写演示。

**覆盖特性：**
- `storage_get` — 读取本地存储的配置
- `storage_set` — 保存配置到本地存储
- `json_parse` — 反序列化存储的 JSON 配置
- `json_stringify` — 序列化配置对象为 JSON
- `@mount` — 组件挂载时从 storage 加载配置
- `signal` — 响应式绑定到表单控件

**组件结构：**
```
component UserSettings {
    @store NotificationStore;
    signal username: String = "";
    signal fontSize: String = "medium";
    signal autoSave: bool = true;

    @mount { loadSettings(); }
    action loadSettings() { ... }
    action saveSettings() { ... }
    view { ... }
}
```

## 文件间关系

```
src/lib/api.omni           ← 基础层：类型 + 封装
    ↓ import
src/pages/DataFetching.omni  ← GET 场景
src/pages/DataSubmit.omni    ← POST 场景
src/pages/UserSettings.omni  ← Storage 场景（不依赖 api.omni）
```

`DataFetching` 和 `DataSubmit` 依赖 `api.omni` 的 `ApiResult` 类型和 `fetch_json` 函数。`UserSettings` 独立使用 JSON/Storage stdlib。

## 入口集成

在 `src/main.omni` 的 App 组件中：
- 新增 import：`DataFetching`、`DataSubmit`、`UserSettings`
- 在 `demo-section` 中添加对应的 `<DataFetching />`、`<DataSubmit />`、`<UserSettings />`

## 已知限制（来自 Phase 1-7 经验）

编写新文件时需避免：
- 不在 view text 中使用 `[`、`=`、`$`、`>`、`&` 字符
- hex 颜色不使用含 `e` 的值（`#e5e7eb` → 用 `#d1d5db` 或 `rgb()` 代替）
- 不在 `class:` map 中使用带连字符的类名
- `let` 声明只放在函数顶层，不在循环体内
- 不在 `style: { }` 内联样式中使用带连字符的 CSS 属性
- Store actions/fields 只在 view 事件处理中调用，不在独立 `fn` 体内

## 成功标准

1. **编译通过**：`omnic build --target omniview` 零错误
2. **测试通过**：`cargo test -p omni-compiler` 全部 410+ 测试通过
3. **特性覆盖**：6 项新特性（match / await / variant 构造 / http / json / storage）在项目中可找到使用示例
4. **运行验证**：`omnic run` 在浏览器中页面正常渲染，无 JS 控制台错误
