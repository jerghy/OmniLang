# Learn-OmniView 语法缺口补充实施计划

> **For agentic workers:** 逐任务实现。步骤使用 checkbox (`- [ ]`) 语法追踪。

**Goal:** 将 22 个未使用的 OmniView 编译器语法特性补充到 learn-omniview 项目中，实现编译器语法 100% 覆盖。

**Architecture:** 12 个任务：3 个新文件 + 9 个修改现有文件。每个任务聚焦一个独立特性或一组紧密相关特性，修改集中在每个文件的局部区域。

**Tech Stack:** OmniView .omni files only. No compilation testing required.

## Global Constraints

- **不编译，不测试** — 只管写 .omni 文件代码
- 所有改动在 `test/projects/learn-omniview/src/` 下
- 遵循现有代码风格：中文注释头、`import omniview;` 前缀
- 每个组件自包含：有 view + style
- 修改现有文件时只增量添加，不删除已有功能

---

### Task 1: 三元表达式 `?:` + 可选链 `?.` — 更新 NullishDemo

**Files:**
- Modify: `test/projects/learn-omniview/src/components/NullishDemo.omni`

**What:** 在 NullishDemo 中新增 ternary 和 optional chaining 演示区。

- [ ] 在 signal 区域新增：
```omniview
// ternary demo signals
signal score: i32 = 85;
signal hasPermission: bool = true;
// optional chaining demo signals
signal maybeData: Map<String, String>? = null;
signal maybeStruct: UserProfile? = null;
```

- [ ] 新增辅助 struct + fn + computeds：
```omniview
struct UserProfile {
    name: String,
    email: String,
    nickname: String?,
}

// ternary computed
computed grade: String = score >= 90 ? "A" : score >= 80 ? "B" : score >= 70 ? "C" : "D";
computed accessLabel: String = hasPermission ? "Granted" : "Denied";

// optional chaining: ?. on signal + ??
computed safeNickname: String = maybeStruct?.nickname ?? "(no nickname)";
computed safeEmail: String = maybeStruct?.email ?? "(no email)";

fn loadProfile() {
    maybeStruct = UserProfile { name: "Test", email: "test@test.com", nickname: null };
}

fn clearProfile() {
    maybeStruct = null;
    maybeData = null;
}

fn togglePermission() {
    hasPermission = !hasPermission;
}

fn randomScore() {
    let val = 55 + (10 * 3);
    score = val;
}
```

- [ ] 在 view 中新增两个 section（加在现有内容之后，最后 `</div>` 之前）：
```omniview
<!-- Ternary expression demo -->
<div class: section>
    <p class: section-title>Ternary expression (cond ? a : b):</p>
    <p class: code-text>score ({score}) → grade = score &gt;= 90 ? "A" : score &gt;= 80 ? "B" : ... &rarr; "{grade}"</p>
    <p class: code-text>hasPermission ? "Granted" : "Denied" &rarr; "{accessLabel}"</p>
    <div class: btn-row>
        <button class: btn @onPress={randomScore}>Random Score</button>
        <button class: btn @onPress={togglePermission}>Toggle Permission</button>
    </div>
</div>

<!-- Optional chaining demo -->
<div class: section>
    <p class: section-title>Optional chaining (obj?.field):</p>
    <p class: code-text>maybeStruct?.nickname (null coalesce) "(no nickname)" = "{safeNickname}"</p>
    <p class: code-text>maybeStruct?.email (null coalesce) "(no email)" = "{safeEmail}"</p>
    <p class: code-text>maybeStruct is {if maybeStruct == null { "null" } else { "set" }}</p>
    <div class: btn-row>
        <button class: btn @onPress={loadProfile}>Load Profile</button>
        <button class: btn-clear @onPress={clearProfile}>Clear Profile</button>
    </div>
</div>
```

---

### Task 2: 模板字符串插值 `${expr}` — 更新 StringLiteralsDemo

**Files:**
- Modify: `test/projects/learn-omniview/src/components/StringLiteralsDemo.omni`

**What:** 新增带 `${}` 插值的模板字符串演示。

- [ ] 新增 signal：
```omniview
signal userName: String = "OmniView";
signal version: f64 = 1.0;
signal interpolated: String = `Hello ${userName}, current version is ${version}`;
```

- [ ] 新增 fn：
```omniview
fn buildGreeting(name: String, age: i32) -> String {
    return `User ${name} is ${age} years old`;
}

fn updateTemplate() {
    userName = "User" + version.to_string();
    interpolated = `Hello ${userName}, v${version}`;
}
```

- [ ] 在 view 中（`</div>` 之前）新增 section：
```omniview
<div class: card>
    <h4 class: label>模板字符串插值 {"`${expr}`"}</h4>
    <p class: value>buildGreeting("Alice", 30) &rarr; "{buildGreeting("Alice", 30)}"</p>
    <p class: value>dynamic: {interpolated}</p>
    <button class: btn-sm @onPress={updateTemplate}>Update</button>
</div>
```

- [ ] 新增 style：
```css
.btn-sm { padding: 6px 12px; border: 1px solid #d4d4d8; border-radius: 6px; background: #6366f1; color: white; font-size: 13px; cursor: pointer; margin-top: 8px; }
```

---

### Task 3: class:"string literal" 绑定 — 更新 ViewAttributesDemo

**Files:**
- Modify: `test/projects/learn-omniview/src/components/ViewAttributesDemo.omni`

**What:** 新增字符串字面量 class 绑定演示。

- [ ] 在 view 中最后一张 card 之后 (`</div>` 之前) 新增：
```omniview
<div class: card>
    <h4 class: label>class:"string" 字面量绑定</h4>
    <div class: "bg-blue-100 border border-blue-300 rounded-lg p-3 mb-2">
        <p class: info-text>此 div 使用 class: "bg-blue-100 border border-blue-300 rounded-lg p-3 mb-2" 字符串字面量</p>
    </div>
    <div class: "bg-green-100 border border-green-300 rounded-lg p-3 mb-2">
        <p class: info-text>此 div 使用 class: "bg-green-100 ..." 字符串字面量</p>
    </div>
    <div class: "bg-amber-100 border border-amber-300 rounded-lg p-3">
        <p class: info-text>字符串字面量 class 适用于无对应 style 规则的工具类</p>
    </div>
</div>
```

---

### Task 4: @for key 表达式 — 更新 FeatureList + DataTable

**Files:**
- Modify: `test/projects/learn-omniview/src/components/FeatureList.omni`
- Modify: `test/projects/learn-omniview/src/components/DataTable.omni`

**What:** 在现有 @for 中添加 key 表达式。

- [ ] FeatureList.omni — 将 `@for f in features {` 改为：
```omniview
@for f in features key={f} {
    <span class: tag>{f}</span>
}
```

- [ ] DataTable.omni — 将 `@for row in rows {` 改为：
```omniview
@for row in rows key={row.id} {
    <!-- 现有内容不变 -->
    ...
}
```

---

### Task 5: Slot 内容投影（父传子）— 新建 SlotContentDemo

**Files:**
- Create: `test/projects/learn-omniview/src/components/SlotContentDemo.omni`

**What:** 新建组件演示父组件向子组件的 @slot 投影实际内容。

- [ ] 完整文件内容：
```omniview
// ╔══════════════════════════════════════════════════════════════╗
// ║  SlotContentDemo.omni —— Slot 内容投影演示                     ║
// ║  演示：父组件传递内容到子组件 @slot（header/body/footer）        ║
// ╚══════════════════════════════════════════════════════════════╝

import omniview;
import { Modal } from "./Modal";

component SlotContentDemo {
    signal showModal: bool = false;
    signal feedback: String = "";

    fn openModal() { showModal = true; feedback = ""; }
    fn closeModal() { showModal = false; }
    fn handleConfirm() {
        feedback = "Confirmed!";
        closeModal();
    }
    fn handleCancel() {
        feedback = "Cancelled";
        closeModal();
    }

    view {
        <div class: slot-demo>
            <h3 class: title>Slot Content Projection Demo</h3>
            <p class: subtitle>父组件传递内容到子组件的 @slot（header/body/footer）</p>

            <button class: btn @onPress={openModal}>Open Modal with Custom Slots</button>

            @if feedback != "" {
                <div class: feedback>
                    <p class: feedback-text>{feedback}</p>
                </div>
            }

            @if showModal {
                <!-- 关键：将实际 UI 内容投影到 Modal 的 @slot -->
                <Modal>
                    <!-- 投影到 @slot header -->
                    <div class: custom-header>
                        <span class: icon>📋</span>
                        <span>Custom Header Content</span>
                    </div>

                    <!-- 投影到 @slot body（默认 slot） -->
                    <div class: custom-body>
                        <p class: body-title>This is projected body content</p>
                        <p class: body-text>父组件 SlotContentDemo 向 Modal 子组件的 @slot header 和 @slot body 注入自定义内容。</p>
                        <p class: body-text>子组件 Modal 中的 @slot 是占位符，父组件通过 &lt;Modal&gt;...children...&lt;/Modal&gt; 语法填充内容。</p>
                    </div>

                    <!-- 投影到 @slot footer -->
                    <div class: custom-footer>
                        <button class: footer-btn confirm @onPress={handleConfirm}>✅ Confirm</button>
                        <button class: footer-btn cancel @onPress={handleCancel}>❌ Cancel</button>
                    </div>
                </Modal>
            }
        </div>
    }
    style {
        .slot-demo { background: white; border: 1px solid #d4d4d8; border-radius: 12px; padding: 20px 16px; margin: 16px 0; }
        .title { font-size: 18px; color: #1f2937; margin: 0 0 4px 0; }
        .subtitle { font-size: 13px; color: #6b7280; margin: 0 0 16px 0; }
        .btn { padding: 10px 20px; background: #6366f1; color: white; border: none; border-radius: 8px; font-size: 14px; cursor: pointer; }
        .feedback { margin-top: 12px; padding: 10px 14px; background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 8px; }
        .feedback-text { font-size: 14px; color: #166534; margin: 0; }

        .custom-header { display: flex; align-items: center; gap: 8px; font-size: 16px; font-weight: 600; color: #1f2937; }
        .icon { font-size: 20px; }
        .custom-body { margin-top: 8px; }
        .body-title { font-size: 15px; font-weight: 600; color: #374151; margin: 0 0 8px 0; }
        .body-text { font-size: 13px; color: #6b7280; margin: 4px 0; line-height: 1.5; }
        .custom-footer { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
        .footer-btn { padding: 8px 16px; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .confirm { background: #10b981; color: white; }
        .cancel { background: #f3f4f6; color: #374151; border: 1px solid #d4d4d8; }
    }
}
```

---

### Task 6: computed 块体形式 — 更新 Counter

**Files:**
- Modify: `test/projects/learn-omniview/src/components/Counter.omni`

**What:** 新增一个 computed 块体形式来演示多行 computed body。

- [ ] 在现有 computed 之后、fn inc() 之前新增：
```omniview
// computed 块体形式：包含多行逻辑的派生计算
computed stats() -> String {
    let prefix = if count >= 0 { "正向" } else { "负向" };
    let magnitude = if count < 0 { -count } else { count };
    let tier = if magnitude > 50 { "高" } else if magnitude > 20 { "中" } else { "低" };
    return prefix + " · " + tier + "档位 (abs=" + magnitude.to_string() + ")";
}
computed rangeHint() -> String {
    if count > 100 { return "Count is very high!"; }
    if count < -50 { return "Count is very low!"; }
    return "Count is normal";
}
```

- [ ] 在 view 中 `{doubled}` 那一行下面新增一行：
```omniview
<p class: info>{stats()} | {rangeHint()}</p>
```

---

### Task 7: 内联 Store — 新建 InlineStoreDemo

**Files:**
- Create: `test/projects/learn-omniview/src/components/InlineStoreDemo.omni`

**What:** 演示在 component 内部定义 store。

- [ ] 完整文件内容：
```omniview
// ╔══════════════════════════════════════════════════════════════╗
// ║  InlineStoreDemo.omni —— 内联 Store 演示                       ║
// ║  演示：在 component 内部使用 store Name { ... } 定义局部 store   ║
// ╚══════════════════════════════════════════════════════════════╝

import omniview;

component InlineStoreDemo {
    // 内联 store 定义在 component 内部，不导出到外部
    store CartStore {
        signal items: Array<String> = [];
        signal totalPrice: f64 = 0.0;

        getter itemCount: i32 = items.len();
        getter summary: String = if items.len() == 0 {
            "购物车为空"
        } else {
            "共 " + items.len().to_string() + " 件商品"
        };

        action addItem(name: String, price: f64) {
            items.push(name);
            totalPrice = totalPrice + price;
        }

        action removeLast() {
            if items.len() > 0 {
                items.pop();
                totalPrice = totalPrice - 10.0;
            }
        }

        action clear() {
            items = [];
            totalPrice = 0.0;
        }
    }

    @store CartStore;

    signal log: String = "";

    fn addApple() {
        CartStore.addItem("苹果", 5.0);
        log = "Added: 苹果 ¥5.0";
    }

    fn addBook() {
        CartStore.addItem("OmniView 指南", 39.9);
        log = "Added: OmniView 指南 ¥39.9";
    }

    fn addLaptop() {
        CartStore.addItem("笔记本电脑", 5999.0);
        log = "Added: 笔记本电脑 ¥5999.0";
    }

    fn remove() {
        CartStore.removeLast();
        log = "Removed last item";
    }

    fn clearCart() {
        CartStore.clear();
        log = "Cart cleared";
    }

    view {
        <div class: container>
            <h3 class: title>内联 Store 演示</h3>
            <p class: subtitle>component 内 store Name { ... } + @store Name</p>

            <!-- 状态展示 -->
            <div class: status-area>
                <p class: status-text>{CartStore.summary}</p>
                <p class: status-text>总价: ¥{CartStore.totalPrice}</p>
                <p class: status-text>商品数: {CartStore.itemCount}</p>
            </div>

            <!-- 操作按钮 -->
            <div class: btn-group>
                <button class: btn @onPress={addApple}>加苹果</button>
                <button class: btn @onPress={addBook}>加书</button>
                <button class: btn @onPress={addLaptop}>加电脑</button>
                <button class: btn-danger @onPress={remove}>移除最后</button>
                <button class: btn-clear @onPress={clearCart}>清空</button>
            </div>

            <!-- 商品列表 -->
            @if CartStore.itemCount > 0 {
                <div class: items-area>
                    <p class: items-title>商品列表:</p>
                    @for item in CartStore.items key={item} {
                        <span class: item-tag>{item}</span>
                    }
                </div>
            }
            @else {
                <div class: empty-area>
                    <p class: empty-text>购物车为空，点击上方按钮添加商品</p>
                </div>
            }

            @if log != "" {
                <div class: log-area>
                    <p class: log-text>操作日志: {log}</p>
                </div>
            }
        </div>
    }
    style {
        .container { background: white; border: 1px solid #d4d4d8; border-radius: 12px; padding: 20px 16px; margin: 16px 0; }
        .title { font-size: 18px; color: #1f2937; margin: 0 0 4px 0; }
        .subtitle { font-size: 13px; color: #9ca3af; margin: 0 0 16px 0; }

        .status-area { padding: 12px; background: #f0f9ff; border: 1px solid #bae6fd; border-radius: 8px; margin-bottom: 12px; }
        .status-text { font-size: 14px; color: #075985; margin: 2px 0; }

        .btn-group { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 12px; }
        .btn { padding: 8px 14px; background: #6366f1; color: white; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .btn-danger { padding: 8px 14px; background: #f97316; color: white; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
        .btn-clear { padding: 8px 14px; background: #ef4444; color: white; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }

        .items-area { padding: 10px 12px; background: #f9fafb; border-radius: 8px; margin-bottom: 8px; }
        .items-title { font-size: 13px; color: #374151; margin: 0 0 6px 0; }
        .item-tag { display: inline-block; font-size: 12px; padding: 3px 8px; background: #e0e7ff; color: #3730a3; border-radius: 4px; margin-right: 4px; margin-bottom: 4px; }

        .empty-area { padding: 24px; text-align: center; }
        .empty-text { font-size: 14px; color: #9ca3af; margin: 0; }

        .log-area { padding: 8px 12px; background: #f3f4f6; border-radius: 6px; margin-top: 8px; }
        .log-text { font-size: 12px; color: #6b7280; margin: 0; }
    }
}
```

---

### Task 8: CSS 伪类选择器 — 更新 StyleAdvanced

**Files:**
- Modify: `test/projects/learn-omniview/src/components/StyleAdvanced.omni`

**What:** 在 style 块中新增带伪类的 CSS 规则组，view 中添加演示元素。

- [ ] 在 view 中 `<!-- 字符串转义展示 -->` section 之后，新增：
```omniview
<!-- CSS 伪类演示 -->
<div class: section>
    <p class: section-title>CSS pseudo-classes (:hover, :focus, :active, :disabled, :checked):</p>
    <div class: pseudo-grid>
        <button class: pseudo-btn hover-demo>Hover me (:hover)</button>
        <button class: pseudo-btn focus-demo>Focus me (:focus)</button>
        <button class: pseudo-btn active-demo>Press me (:active)</button>
        <button class: pseudo-btn disabled-demo" disabled>Disabled (:disabled)</button>
        <label class: pseudo-check>
            <input type="checkbox" checked />
            <span>Checked (:checked)</span>
        </label>
    </div>
</div>
```

- [ ] 在 style 块末尾（最后一个 `}` 之前）新增伪类规则：
```css
/* CSS 伪类演示 */
.pseudo-grid { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.pseudo-btn { padding: 10px 16px; border: 2px solid #d4d4d8; border-radius: 8px; font-size: 13px; cursor: pointer; background: white; color: #374151; transition: all 200ms ease; }
.hover-demo:hover { background: #6366f1; color: white; border-color: #6366f1; transform: translateY(-2px); box-shadow: 0 4px 12px rgba(99,102,241,0.3); }
.focus-demo:focus { outline: 3px solid #6366f1; outline-offset: 2px; border-color: #6366f1; }
.active-demo:active { background: #4338ca; color: white; transform: scale(0.95); }
.disabled-demo:disabled { background: #f3f4f6; color: #9ca3af; border-color: #e5e7eb; cursor: not-allowed; }
.pseudo-check { display: flex; align-items: center; gap: 6px; font-size: 14px; color: #374151; cursor: pointer; }
.pseudo-check input:checked + span { color: #6366f1; font-weight: 600; }
```

---

### Task 9: CSS 单位补全 (10 个) — 更新 CssUnitsDemo

**Files:**
- Modify: `test/projects/learn-omniview/src/components/CssUnitsDemo.omni`

**What:** 新增 10 个缺失的 CSS 单位演示盒子。

- [ ] 在 view 的 `.grid` 内最后一个 `<div>` 之后新增：
```omniview
<div class: box-s>1s</div>
<div class: box-vmax>5vmax</div>
<div class: box-svw>5svw</div>
<div class: box-lvw>5lvw</div>
<div class: box-dvw>5dvw</div>
<div class: box-cqh>5cqh</div>
<div class: box-cqb>5cqb</div>
<div class: box-cqmin>5cqmin</div>
<div class: box-cqmax>5cqmax</div>
<div class: box-rlh>2rlh</div>
```

- [ ] 更新 subtitle 文本：
```omniview
<p class: subtitle>28 个 CSS 单位全覆盖（含 10 个新增：s, vmax, svw, lvw, dvw, cqh, cqb, cqmin, cqmax, rlh）</p>
```

- [ ] 在 style 块末尾新增对应 CSS 规则：
```css
.box-s { animation-duration: 1s; background: #fecdd3; padding: 8px; font-size: 12px; border-radius: 6px; text-align: center; }
.box-vmax { width: 5vmax; height: 5vmax; background: #a7f3d0; padding: 4px; font-size: 10px; border-radius: 6px; text-align: center; }
.box-svw { width: 5svw; background: #c7d2fe; padding: 8px; font-size: 12px; border-radius: 6px; text-align: center; }
.box-lvw { width: 5lvw; background: #fde68a; padding: 8px; font-size: 12px; border-radius: 6px; text-align: center; }
.box-dvw { width: 5dvw; background: #ddd6fe; padding: 8px; font-size: 12px; border-radius: 6px; text-align: center; }
.box-cqh { height: 5cqh; background: #fbcfe8; padding: 8px; font-size: 12px; border-radius: 6px; text-align: center; }
.box-cqb { height: 5cqb; background: #a5f3fc; padding: 8px; font-size: 12px; border-radius: 6px; text-align: center; }
.box-cqmin { width: 5cqmin; height: 5cqmin; background: #fed7aa; padding: 4px; font-size: 10px; border-radius: 6px; text-align: center; }
.box-cqmax { width: 5cqmax; height: 5cqmax; background: #d9f99d; padding: 4px; font-size: 10px; border-radius: 6px; text-align: center; }
.box-rlh { line-height: 2rlh; background: #f3e8ff; padding: 8px; font-size: 12px; border-radius: 6px; text-align: center; }
```

---

### Task 10: http_send — 更新 lib/api.omni

**Files:**
- Modify: `test/projects/learn-omniview/src/lib/api.omni`
- Modify: `test/projects/learn-omniview/src/pages/DataFetching.omni` (可选，添加调用演示)

**What:** 在 api.omni 中新增使用 `http_send` 的函数。同时更新 DataFetching 展示。

- [ ] 在 api.omni 文件头部 import 区域新增 `http_send`：
```omniview
import { http_get, http_send } from "std/http";
```

- [ ] 在 api.omni 末尾新增函数：
```omniview
// ── 6. http_send —— 全参数 HTTP 请求（方法、头、体） ──

pub async fn send_custom_request(
    method: String,
    url: String,
    headers: Map<String, String>,
    body: String
) -> ApiResult {
    if url == "" {
        return Err { error: "url is required" };
    }
    let mut request: Map<String, String> = {};
    request.set("method", method);
    request.set("url", url);
    if headers.len() > 0 {
        request.set("headers", json_stringify(headers));
    }
    if body != "" {
        request.set("body", body);
    }
    let raw = await http_send(request);
    if raw == "" {
        return Err { error: "no response" };
    }
    return Ok { data: raw };
}

// ── 7. http_send 预配置请求（带自定义 Header） ──

pub async fn send_with_auth(url: String, token: String) -> ApiResult {
    let mut headers: Map<String, String> = {};
    headers.set("Authorization", "Bearer " + token);
    headers.set("Content-Type", "application/json");
    return await send_custom_request("GET", url, headers, "");
}

// ── 8. http_send 发送 JSON POST ──

pub async fn send_json_post(url: String, jsonBody: String) -> ApiResult {
    let mut headers: Map<String, String> = {};
    headers.set("Content-Type", "application/json");
    return await send_custom_request("POST", url, headers, jsonBody);
}
```

- [ ] 在 api.omni 文件头部新增 import：
```omniview
import { json_stringify } from "std/json";
```
（需要检查 api.omni 是否已有 json import，如果没有则添加）

- [ ] 在 DataFetching.omni 中新增 import 和 action：
```omniview
import { send_custom_request, send_with_auth, send_json_post } from "../lib/api";
```

- [ ] 在 DataFetching.omni 新增信号和 action：
```omniview
signal httpSendResult: String = "";

action testHttpSend() async {
    status = "loading";
    let result = await send_with_auth(
        "https://jsonplaceholder.typicode.com/posts/1",
        "demo-token-123"
    );
    match result {
        Ok { data } => {
            httpSendResult = data;
            status = "success";
        },
        Err { error } => {
            httpSendResult = error;
            status = "error";
        },
        Loading => {
            httpSendResult = "still loading";
        }
    }
}
```

- [ ] 在 DataFetching.omni view 中新增一个 section（放在现有 section 之后）：
```omniview
<div class: section>
    <h4 class: label>http_send (full HTTP client)</h4>
    <button class: btn-send @onPress={testHttpSend}>Send with Auth Header</button>
    @if httpSendResult != "" {
        <pre class: result-block>{httpSendResult}</pre>
    }
</div>
```

- [ ] style 中新增：
```css
.btn-send { padding: 8px 16px; background: #f59e0b; color: white; border: none; border-radius: 6px; font-size: 13px; cursor: pointer; }
.result-block { font-size: 11px; background: #f3f4f6; padding: 8px; border-radius: 4px; max-height: 120px; overflow-y: auto; margin-top: 8px; }
```

---

### Task 11: 注册新组件到 AdvancedFeatures 页面

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/AdvancedFeatures.omni`

**What:** 把新建的 SlotContentDemo 和 InlineStoreDemo 注册到聚合页面。

- [ ] 新增 import：
```omniview
import { SlotContentDemo } from "../components/SlotContentDemo";
import { InlineStoreDemo } from "../components/InlineStoreDemo";
```

- [ ] 在 view 中新增两个 section（放在最后一个 section 和 footer 之间）：
```omniview
<!-- Slot content projection -->
<section class: section>
    <SlotContentDemo />
</section>

<!-- Inline store -->
<section class: section>
    <InlineStoreDemo />
</section>
```

- [ ] 更新 footer-text 文本：
```omniview
<p class: footer-text>22 new syntax features · 3 new files · Phase 10 Gap Closure</p>
```

---

### Task 12: 注册新组件到 main.omni

**Files:**
- Modify: `test/projects/learn-omniview/src/main.omni`

**What:** 把新的聚合页面组件加到主入口。

- [ ] 检查 main.omni 是否已经导入 AdvancedFeatures，如果没有则新增 import：
```omniview
import { AdvancedFeatures } from "./pages/AdvancedFeatures";
```

- [ ] 在 view 中确保 `<AdvancedFeatures />` 被引用（如果已有则跳过）：

---

## Execution Order

```
Task 1  → NullishDemo (ternary + optional chain)
Task 2  → StringLiteralsDemo (template interpolation)
Task 3  → ViewAttributesDemo (class:"string")
Task 4  → FeatureList + DataTable (@for key)
Task 5  → SlotContentDemo (NEW - slot projection)
Task 6  → Counter (computed block-body)
Task 7  → InlineStoreDemo (NEW - inline store)
Task 8  → StyleAdvanced (CSS pseudo-classes)
Task 9  → CssUnitsDemo (CSS units completion)
Task 10 → api.omni + DataFetching (http_send)
Task 11 → AdvancedFeatures (register new components)
Task 12 → main.omni (final registration)

No dependencies between tasks — all can run in parallel.
```

## Coverage Summary

| # | Feature | Task | File(s) |
|---|---------|------|---------|
| 1 | Ternary `?:` | 1 | NullishDemo (modify) |
| 2 | Optional chain `?.` | 1 | NullishDemo (modify) |
| 3 | Template `${expr}` | 2 | StringLiteralsDemo (modify) |
| 4 | class:"string" literal | 3 | ViewAttributesDemo (modify) |
| 5 | @for key={expr} | 4 | FeatureList + DataTable (modify) |
| 6 | Slot content projection | 5 | SlotContentDemo (NEW) |
| 7 | computed block-body | 6 | Counter (modify) |
| 8 | Inline store | 7 | InlineStoreDemo (NEW) |
| 9 | CSS pseudo-classes | 8 | StyleAdvanced (modify) |
| 10 | CSS units (s, vmax, svw, lvw, dvw, cqh, cqb, cqmin, cqmax, rlh) | 9 | CssUnitsDemo (modify) |
| 11 | http_send | 10 | api.omni + DataFetching (modify) |
| — | Page registration | 11-12 | AdvancedFeatures + main (modify) |

**Files created:** 2 new (SlotContentDemo, InlineStoreDemo)
**Files modified:** 10 existing
**Total features covered:** 22 → all gaps closed
