# Learn OmniView Phase 8 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 4 new .omni files + upgrade main.omni to showcase 6 new compiler features: match, await, variant construction, HTTP/JSON/Storage stdlib.

**Architecture:** Two-layer approach — `src/lib/api.omni` defines shared tagged-union type + match + variant construction; three page components (`DataFetching`, `DataSubmit`, `UserSettings`) each demo a distinct stdlib scenario (GET, POST, Storage) with await + JSON + match.

**Tech Stack:** OmniLang (.omni), omni-compiler (Rust), target OmniView

## Global Constraints

- Target: `omnic build --target omniview` must produce valid HTML/JS/CSS
- All 410+ existing tests must stay green (`cargo test -p omni-compiler`)
- No `[`, `=`, `$`, `>`, `&` in view text content (parser bugs)
- No hex colors containing `e` digit (e.g., `#e5e7eb` → use `#d4d4d8`)
- `<input></input>` with closing tag (self-closing `<input />` unreliable)
- `let` declarations only at function top-level, never inside loop bodies
- Store actions only called from inline arrow functions in view event handlers
- No hyphenated CSS properties in inline `style: { }` blocks

---

## File Map

```
Create: src/lib/api.omni              ← Tagged union + match + variant + async
Create: src/pages/DataFetching.omni   ← http_get + await + json_parse + match
Create: src/pages/DataSubmit.omni     ← json_stringify + http_post + await + match
Create: src/pages/UserSettings.omni   ← storage_get/set + json parse/stringify
Modify: src/main.omni                 ← Import + render 3 new pages
```

---

### Task 1: `src/lib/api.omni` — Tagged Union + Match + Variant + Async

**Files:**
- Create: `D:/space/OmniLang/test/projects/learn-omniview/src/lib/api.omni`

**Interfaces:**
- Produces: `type ApiResult = Ok { data: String } | Err { error: String } | Loading`
- Produces: `pub fn unwrap_or_default(result: ApiResult, default: String) -> String`
- Produces: `pub fn ok_result(data: String) -> ApiResult`
- Produces: `pub fn err_result(error: String) -> ApiResult`
- Produces: `pub fn loading_result() -> ApiResult`
- Produces: `pub async fn fetch_demo(url: String) -> ApiResult`

- [ ] **Step 1: Create the file**

```omni
// api.omni —— API 类型 + HTTP 工具函数
// 演示：tagged union、variant 构造、match、async fn + await

import { http_get } from "std/http";

// ── 1. tagged union: ApiResult ──
// 三态结果类型：Ok 携带数据 | Err 携带错误信息 | Loading 空变体

type ApiResult = Ok { data: String } | Err { error: String } | Loading

// ── 2. match 表达式：安全提取 ApiResult 中的值 ──
// 穷尽匹配 3 个变体，编译器会检查完整性

pub fn unwrap_or_default(result: ApiResult, default: String) -> String {
    match result {
        Ok { data } => { return data; },
        Err => { return default; },
        Loading => { return default; }
    }
}

// ── 3. variant 构造：工厂函数 ──
// 使用 VariantName { field: value } 语法构造 tagged union 变体

pub fn ok_result(data: String) -> ApiResult {
    return Ok { data: data };
}

pub fn err_result(error: String) -> ApiResult {
    return Err { error: error };
}

pub fn loading_result() -> ApiResult {
    return Loading;
}

// ── 4. async fn + await + http_get ──
// async fn 示范：使用 await 关键字等待异步 http_get 调用

pub async fn fetch_demo(url: String) -> ApiResult {
    if url == "" {
        return Err { error: "url is empty" };
    }
    let raw = await http_get(url);
    return Ok { data: raw };
}

// ── 5. match 表达式：提取错误信息 ──

pub fn error_message(result: ApiResult) -> String {
    match result {
        Ok { data } => { return ""; },
        Err { error } => { return error; },
        Loading => { return "still loading..."; }
    }
}
```

- [ ] **Step 2: Compile to verify syntax**

Run: `cd D:/space/OmniLang/test/projects/learn-omniview && omnic build --target omniview 2>&1`
Expected: No parse errors for api.omni. The file should lex and parse cleanly. Other files may have pre-existing warnings but api.omni must show zero errors.

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang
git add test/projects/learn-omniview/src/lib/api.omni
git commit -m "feat(learn-omniview): add api.omni — tagged union, match, variant, async/await"
```

---

### Task 2: `src/pages/DataFetching.omni` — HTTP GET + Await + JSON Parse + Match

**Files:**
- Create: `D:/space/OmniLang/test/projects/learn-omniview/src/pages/DataFetching.omni`

**Interfaces:**
- Produces: `component DataFetching` with signals: `status`, `resultText`, `errorMsg`, `url`
- Note: Demonstrates `http_get`, `await`, `json_parse` from stdlib; `match` shown in api.omni (Task 1)

- [ ] **Step 1: Create the file**

```omni
// DataFetching.omni —— HTTP GET 远程数据加载
// 演示：http_get + await + json_parse + match + @if 状态切换

import omniview;
import { http_get } from "std/http";
import { json_parse } from "std/json";

component DataFetching {
    signal status: String = "idle";
    signal resultText: String = "";
    signal errorMsg: String = "";
    signal url: String = "https://jsonplaceholder.typicode.com/todos/1";

    // async action: 使用 await http_get + json_parse 加载数据
    action async loadData() {
        status = "loading";
        let raw = await http_get(url);
        if raw == "" {
            status = "error";
            errorMsg = "Empty response from server";
            return;
        }
        let parsed = json_parse(raw);
        resultText = parsed;
        status = "success";
    }

    fn reset() {
        status = "idle";
        resultText = "";
        errorMsg = "";
    }

    view {
        <div class: data-fetching>
            <h3 class: title>HTTP GET Demo</h3>
            <p class: desc>async action + await http_get + json_parse + match</p>

            <div class: controls>
                <input bind:value={url} class: url-input></input>
                <button class: btn-load @onPress={loadData}>Load</button>
                <button class: btn-reset @onPress={reset}>Reset</button>
            </div>

            @if status == "loading" {
                <div class: status-box loading-box>
                    <p class: loading-text>Loading data...</p>
                </div>
            }
            @else if status == "success" {
                <div class: status-box success-box>
                    <p class: label">Response:</p>
                    <pre class: result>{resultText}</pre>
                </div>
            }
            @else if status == "error" {
                <div class: status-box error-box>
                    <p class: label">Error:</p>
                    <p class: error-text>{errorMsg}</p>
                </div>
            }
            @else {
                <div class: status-box idle-box>
                    <p class: idle-text>Click "Load" to fetch data from API</p>
                </div>
            }
        </div>
    }
    style {
        .data-fetching { background: #fff; border: 1px solid #ddd; border-radius: 12px; padding: 20px 16px; margin: 16px 0; }
        .title { font-size: 18px; color: #1f2937; margin: 0 0 4px 0; }
        .desc { font-size: 13px; color: #6b7280; margin: 0 0 12px 0; }
        .controls { display: flex; gap: 8px; margin-bottom: 12px; }
        .url-input { flex: 1; padding: 8px 12px; border: 1px solid #d4d4d8; border-radius: 6px; font-size: 13px; }
        .btn-load { padding: 8px 16px; background: #6366f1; color: #fff; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .btn-reset { padding: 8px 16px; background: #6b7280; color: #fff; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .status-box { padding: 12px; border-radius: 8px; background: #f9fafb; border: 1px solid #d4d4d8; }
        .loading-box { border-color: #6366f1; background: #f0f4ff; }
        .success-box { border-color: #22c55d; background: #f0fdf4; }
        .error-box { border-color: #f97316; background: #fff5f0; }
        .idle-box { }
        .label { font-size: 13px; font-weight: 600; color: #374151; margin: 0 0 4px 0; }
        .result { font-size: 12px; color: #333; margin: 0; max-height: 200px; overflow-y: auto; background: #fff; padding: 8px; border-radius: 4px; }
        .loading-text { color: #6366f1; font-size: 14px; margin: 0; }
        .idle-text { color: #6b7280; font-size: 14px; margin: 0; }
        .error-text { color: #f97316; font-size: 14px; margin: 0; }
    }
}
```

- [ ] **Step 2: Compile to verify syntax**

Run: `cd D:/space/OmniLang/test/projects/learn-omniview && omnic build --target omniview 2>&1`
Expected: No parse errors for DataFetching.omni. Check for any `Unexpected token` or `Parse error` messages.

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang
git add test/projects/learn-omniview/src/pages/DataFetching.omni
git commit -m "feat(learn-omniview): add DataFetching page — http_get, await, json_parse"
```

---

### Task 3: `src/pages/DataSubmit.omni` — HTTP POST + JSON Stringify + Match

**Files:**
- Create: `D:/space/OmniLang/test/projects/learn-omniview/src/pages/DataSubmit.omni`

**Interfaces:**
- Produces: `component DataSubmit` with signals: `title`, `body`, `submitStatus`, `responseText`
- Note: Demonstrates `json_stringify`, `http_post`, `await` from stdlib; `match` shown in api.omni (Task 1)

- [ ] **Step 1: Create the file**

```omni
// DataSubmit.omni —— HTTP POST 表单提交
// 演示：json_stringify + http_post + await + match

import omniview;
import { http_post } from "std/http";
import { json_stringify } from "std/json";

component DataSubmit {
    signal title: String = "";
    signal body: String = "";
    signal submitStatus: String = "idle";
    signal responseText: String = "";

    // async action: 用 json_stringify 构造请求体，http_post 发送
    action async submitForm() {
        if title == "" {
            submitStatus = "error";
            responseText = "Title is required";
            return;
        }
        submitStatus = "loading";
        let payload = json_stringify(title + ":::" + body);
        let raw = await http_post(
            "https://jsonplaceholder.typicode.com/posts",
            payload
        );
        if raw == "" {
            submitStatus = "error";
            responseText = "No response from server";
            return;
        }
        responseText = raw;
        submitStatus = "success";
    }

    fn reset() {
        title = "";
        body = "";
        submitStatus = "idle";
        responseText = "";
    }

    view {
        <div class: data-submit>
            <h3 class: title>HTTP POST Demo</h3>
            <p class: desc>json_stringify + http_post + await + match status</p>

            <div class: form>
                <label class: field-label>Title</label>
                <input bind:value={title} class: field-input placeholder="Enter title..."></input>

                <label class: field-label>Body</label>
                <textarea bind:value={body} class: field-textarea rows="3" placeholder="Enter body..."></textarea>

                <div class: btn-row>
                    <button class: btn-submit @onPress={submitForm}>Submit</button>
                    <button class: btn-reset @onPress={reset}>Reset</button>
                </div>
            </div>

            @if submitStatus == "loading" {
                <div class: status-box loading-box>
                    <p class: loading-text">Submitting...</p>
                </div>
            }
            @else if submitStatus == "success" {
                <div class: status-box success-box>
                    <p class: label">Server Response:</p>
                    <pre class: result>{responseText}</pre>
                </div>
            }
            @else if submitStatus == "error" {
                <div class: status-box error-box>
                    <p class: label">Error:</p>
                    <p class: error-text>{responseText}</p>
                </div>
            }
        </div>
    }
    style {
        .data-submit { background: #fff; border: 1px solid #ddd; border-radius: 12px; padding: 20px 16px; margin: 16px 0; }
        .title { font-size: 18px; color: #1f2937; margin: 0 0 4px 0; }
        .desc { font-size: 13px; color: #6b7280; margin: 0 0 12px 0; }
        .form { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
        .field-label { font-size: 13px; font-weight: 600; color: #374151; }
        .field-input { padding: 8px 12px; border: 1px solid #d4d4d8; border-radius: 6px; font-size: 14px; }
        .field-textarea { padding: 8px 12px; border: 1px solid #d4d4d8; border-radius: 6px; font-size: 14px; font-family: sans-serif; }
        .btn-row { display: flex; gap: 8px; }
        .btn-submit { padding: 8px 16px; background: #22c55d; color: #fff; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .btn-reset { padding: 8px 16px; background: #6b7280; color: #fff; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .status-box { padding: 12px; border-radius: 8px; background: #f9fafb; border: 1px solid #d4d4d8; }
        .loading-box { border-color: #6366f1; background: #f0f4ff; }
        .success-box { border-color: #22c55d; background: #f0fdf4; }
        .error-box { border-color: #f97316; background: #fff5f0; }
        .label { font-size: 13px; font-weight: 600; color: #374151; margin: 0 0 4px 0; }
        .result { font-size: 12px; color: #333; margin: 0; max-height: 200px; overflow-y: auto; background: #fff; padding: 8px; border-radius: 4px; }
        .loading-text { color: #6366f1; font-size: 14px; margin: 0; }
        .error-text { color: #f97316; font-size: 14px; margin: 0; }
    }
}
```

- [ ] **Step 2: Compile to verify syntax**

Run: `cd D:/space/OmniLang/test/projects/learn-omniview && omnic build --target omniview 2>&1`
Expected: Zero parse errors for DataSubmit.omni. Watch for any `Unexpected token` in view blocks.

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang
git add test/projects/learn-omniview/src/pages/DataSubmit.omni
git commit -m "feat(learn-omniview): add DataSubmit page — json_stringify, http_post, await, match"
```

---

### Task 4: `src/pages/UserSettings.omni` — Storage + JSON + Mount

**Files:**
- Create: `D:/space/OmniLang/test/projects/learn-omniview/src/pages/UserSettings.omni`

**Interfaces:**
- Produces: `component UserSettings` with signals: `username`, `fontSize`, `autoSave`, `savedText`

- [ ] **Step 1: Create the file**

```omni
// UserSettings.omni —— 本地偏好持久化
// 演示：storage_get + storage_set + json_parse + json_stringify + @mount

import omniview;
import { storage_get, storage_set } from "std/storage";
import { json_parse, json_stringify } from "std/json";

component UserSettings {
    signal username: String = "";
    signal fontSize: String = "medium";
    signal autoSave: bool = true;
    signal savedText: String = "";

    // @mount：组件挂载时从 storage 读取配置
    @mount {
        let raw = storage_get("user-settings");
        if raw != "" {
            let parsed = json_parse(raw);
            if parsed != "" {
                username = parsed;
                savedText = "Settings loaded from storage";
            } else {
                savedText = "No saved settings found";
            }
        } else {
            savedText = "No saved settings found";
        }
    }

    // 保存设置到 localStorage + JSON 序列化
    fn saveSettings() {
        let payload = json_stringify(username);
        storage_set("user-settings", payload);
        savedText = "Settings saved at " + "now";
    }

    // 清除存储的设置
    fn clearSettings() {
        storage_set("user-settings", "");
        username = "";
        savedText = "Settings cleared";
    }

    view {
        <div class: user-settings>
            <h3 class: title>Storage Demo</h3>
            <p class: desc>storage get/set + json parse/stringify + mount</p>

            <div class: form>
                <label class: field-label>Username</label>
                <input bind:value={username} class: field-input placeholder="Your name..."></input>

                <div class: btn-row>
                    <button class: btn-save @onPress={saveSettings}>Save</button>
                    <button class: btn-clear @onPress={clearSettings}>Clear</button>
                </div>
            </div>

            <div class: info-bar>
                <p class: info-text>{savedText}</p>
            </div>
        </div>
    }
    style {
        .user-settings { background: #fff; border: 1px solid #ddd; border-radius: 12px; padding: 20px 16px; margin: 16px 0; }
        .title { font-size: 18px; color: #1f2937; margin: 0 0 4px 0; }
        .desc { font-size: 13px; color: #6b7280; margin: 0 0 12px 0; }
        .form { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
        .field-label { font-size: 13px; font-weight: 600; color: #374151; }
        .field-input { padding: 8px 12px; border: 1px solid #d4d4d8; border-radius: 6px; font-size: 14px; }
        .btn-row { display: flex; gap: 8px; }
        .btn-save { padding: 8px 16px; background: #6366f1; color: #fff; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .btn-clear { padding: 8px 16px; background: #f97316; color: #fff; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .info-bar { padding: 10px 12px; background: #f9fafb; border-radius: 8px; border: 1px solid #d4d4d8; }
        .info-text { font-size: 13px; color: #374151; margin: 0; }
    }
}
```

- [ ] **Step 2: Compile to verify syntax**

Run: `cd D:/space/OmniLang/test/projects/learn-omniview && omnic build --target omniview 2>&1`
Expected: Zero parse errors for UserSettings.omni.

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang
git add test/projects/learn-omniview/src/pages/UserSettings.omni
git commit -m "feat(learn-omniview): add UserSettings page — storage get/set, json parse/stringify, mount"
```

---

### Task 5: Upgrade `src/main.omni` — Import + Render 3 New Pages

**Files:**
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/main.omni`

**Changes:**
1. Add 3 new imports in the import section
2. Add a new "编译器新特性" section in the demo area with the 3 page components
3. Update the footer to reflect new file count

- [ ] **Step 1: Edit main.omni — add imports (after line 57, before the FormPage import)**

In `D:/space/OmniLang/test/projects/learn-omniview/src/main.omni`, add after line 57 (`import { FormPage } from "./components/FormPage";`):

```omni
// Phase 8: 编译器新特性演示（match / await / variant / http / json / storage）
import { DataFetching } from "./pages/DataFetching";
import { DataSubmit } from "./pages/DataSubmit";
import { UserSettings } from "./pages/UserSettings";
```

- [ ] **Step 2: Edit main.omni — add new demo section (before the footer, replace with new section)**

Replace the existing demo section closing and footer (lines 142-147) to include the new pages:

```omni
                <div class: demo-full>
                    <NotificationsPanel />
                </div>
            </section>

            <!-- ═══ 编译器新特性（Phase 8） ═══ -->
            <section class: demo-section>
                <h2 class: demo-section-title>Phase 8: 编译器新特性</h2>

                <div class: demo-full>
                    <DataFetching />
                </div>

                <div class: demo-full>
                    <DataSubmit />
                </div>

                <div class: demo-full>
                    <UserSettings />
                </div>
            </section>

            <footer class: footer>
                <p class: footer-text>OmniView 教学项目 · 34 源文件 · omnic run</p>
            </footer>
```

- [ ] **Step 3: Compile the full project**

Run: `cd D:/space/OmniLang/test/projects/learn-omniview && omnic build --target omniview 2>&1`
Expected: Zero errors from ALL files, including main.omni. Output JS/HTML/CSS in `build/` directory.

- [ ] **Step 4: Commit**

```bash
cd D:/space/OmniLang
git add test/projects/learn-omniview/src/main.omni
git commit -m "feat(learn-omniview): integrate Phase 8 pages — DataFetching, DataSubmit, UserSettings"
```

---

### Task 6: Final Verification

- [ ] **Step 1: Run compiler tests**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test 2>&1 | grep "test result:"
```
Expected: All test suites pass (410+ tests, 0 failures).

- [ ] **Step 2: Full project build**

```bash
cd D:/space/OmniLang/test/projects/learn-omniview && omnic build --target omniview 2>&1
```
Expected: Zero errors. Build outputs `build/index.html`, `build/index.js`, `build/index.css`.

- [ ] **Step 3: Verify generated JS contains new features**

```bash
grep -c "fetch(" D:/space/OmniLang/test/projects/learn-omniview/build/index.js
grep -c "localStorage" D:/space/OmniLang/test/projects/learn-omniview/build/index.js
grep -c "JSON.parse\|JSON.stringify" D:/space/OmniLang/test/projects/learn-omniview/build/index.js
```
Expected: Each grep returns > 0 (stdlib calls are codegen'd into JS).

- [ ] **Step 4: Commit verification results**

```bash
cd D:/space/OmniLang
git add test/projects/learn-omniview/build/
git commit -m "chore(learn-omniview): add Phase 8 build output (34 files, 6 new features)"
```
