# Learn OmniView 语法补充实施计划

> 目标：将编译器支持的 117 项未覆盖语法构造全部纳入 `test/projects/learn-omniview` 项目
> 
> 现状：43 个 .omni 源文件，覆盖约 60% 编译器语法。剩余 117 项待补充。
>
> 原则：按主题聚合（非按语法类型），每个新增组件同时覆盖多个相关缺失项。

---

## 总览

| Phase | 优先级 | 主题 | 新增文件 | 修改文件 | 覆盖项数 |
|-------|--------|------|---------|---------|---------|
| 1 | **P0** | 错误处理 + 组件事件 | 3 | 1 | 8 |
| 2 | **P0** | 类型系统全覆盖 | 2 | 1 | 9 |
| 3 | **P0** | 信号方法 + 可选链 | 0 | 2 | 3 |
| 4 | **P1** | 标准库 — 新模块 | 3 | 0 | 3 模块 (26 函数) |
| 5 | **P1** | 标准库 — 补充函数 | 4 | 1 | 27 |
| 6 | **P1** | 视图属性 + 事件处理器 | 2 | 0 | 22 |
| 7 | **P2** | HTML 语义标签 + CSS 单位 | 3 | 0 | 49 |
| 8 | **P2** | 字符串字面量 + 预留指令 | 1 | 0 | 5 |
| — | **Skip** | @native / @fallback / @error | 0 | 0 | 3 (预留，无法实测) |

**总计：新增 18 文件，修改 5 文件，覆盖 114/117 项**

---

## Phase 1：错误处理 + 组件事件（P0，覆盖 #1 #2 #3 #7 #8）

### 新增 `src/lib/error_handling.omni`（纯逻辑文件）

覆盖项：#1 try/catch/finally, #2 throw, #7 Map<K,V>, #8 union type 注解

```omni
// 函数：try/catch/finally + throw
// pub fn safe_divide(a: f64, b: f64) -> f64 — throw when b==0
// pub fn parse_config(raw: String) -> Map<String, String> — 返回 Map<K,V> 类型
// pub fn get_or_throw(map: Map<String, String>, key: String) -> String — try/catch
// type ParseResult = Ok { data: Map<String, String> } | Err { msg: String } — union type
```

**覆盖明细：**
- `throw` 语句 → `safe_divide` 中除零抛出
- `try { } catch err { } finally { }` → `get_or_throw` 中使用
- `Map<K, V>` 类型注解 → `parse_config` 和 `get_or_throw` 的形参/返回值
- 联合类型注解 `String | i32` → 变量类型或函数参数中使用

### 新增 `src/components/ErrorHandlingDemo.omni`

覆盖项：#3 emit 语句, #7 Map<K,V>, #1 try/catch

```omni
// component ErrorHandlingDemo
//   event onError: String              ← 声明事件
//   event onRecovered: String
//   signal lastError: String = ""
//   
//   action triggerError(): void {
//       try {
//           let result = safe_divide(10.0, 0.0);   // 会 throw
//       } catch err { 
//           lastError = err;
//           emit onError(err);          ← emit 语句发射事件
//       } finally {
//           console_log("错误处理完成");
//       }
//   }
//   
//   action processConfig(): void {
//       let config: Map<String, String> = parse_config("{ ... }");
//       try { ... } catch err { emit onError(err); }
//   }
//
//   view { ... }  ← 展示错误状态 + 恢复按钮
//   style { ... }
```

**覆盖明细：**
- `emit eventName(args)` → 两个 action 中各使用一次
- `try/catch/finally` + `throw` 的组件级交互
- `Map<K,V>` 类型在 signal/let 中的实际使用

### 新增 `src/lib/union_types.omni`（纯逻辑文件）

覆盖项：#8 联合类型注解 `|`

```omni
// 类型层面的 union type（非 tagged union type）
// pub fn format_value(val: String | i32 | f64) -> String
// pub fn describe(input: String | bool) -> String
```

**覆盖明细：**
- `TypeAnnotation::Union` → `String | i32 | f64` 和 `String | bool` 两种联合

### 修改 `src/main.omni`

```diff
+ import { ErrorHandlingDemo } from "./components/ErrorHandlingDemo";
+ // 在 demo-section 中添加 <ErrorHandlingDemo />
```

---

## Phase 2：类型系统全覆盖（P0，覆盖 #9 #10 #11 #12）

### 新增 `src/lib/type_system.omni`（纯逻辑文件）

覆盖项：#9 fn 类型, #10 所有整数/浮点变体, #11 isize/usize, #12 Void

```omni
// ── 函数类型注解 (#9) ──
// pub fn apply_transform(val: i32, mapper: fn(i32) -> i32) -> i32
// pub fn create_formatter(prefix: String) -> fn(String) -> String
// let doubler: fn(i32) -> i32 = |x| x * 2;
// let greeter: fn(String) -> String = |name| "Hello " + name;

// ── 整数类型全覆盖 (#10, #11) ──
// pub fn demo_i8() -> i8 { return 127; }
// pub fn demo_i16() -> i16 { return 32767; }
// pub fn demo_i64() -> i64 { return 9223372036854775807; }
// pub fn demo_u8() -> u8 { return 255; }
// pub fn demo_u16() -> u16 { return 65535; }
// pub fn demo_u32() -> u32 { return 4294967295; }
// pub fn demo_u64() -> u64 { return 18446744073709551615; }
// pub fn demo_f32() -> f32 { return 3.14; }

// ── 平台相关类型 (#11) ──
// pub fn demo_isize() -> isize { return -42; }
// pub fn demo_usize() -> usize { return 100; }

// ── Void 类型 (#12) ──
// pub fn log_and_discard(msg: String) -> Void { console_log(msg); }
// 说明：Void 表示此函数无返回值（用于明确标注）

// ── 综合示例 ──
// pub fn type_system_summary() -> String { ... }
```

**覆盖明细：**
- `fn(Type) -> Type` 类型注解 → 函数参数类型和变量类型
- `i8, i16, i64, u8, u16, u32, u64, f32` → 各至少一个函数
- `isize, usize` → 各一个函数
- `Void` → 返回类型标注

### 新增 `src/components/TypeSystemDemo.omni`

```omni
// component TypeSystemDemo
//   signal allTypes: Array<String> = [
//       "i8: " + demo_i8().to_string(),
//       ...
//   ]
//   signal mapperResult: i32 = apply_transform(5, doubler)
//   signal formatResult: String = create_formatter("[Omni]")("Hello")
//   view { ... } ← 展示各类型返回值
```

### 修改 `src/lib.omni`

```diff
+ // Phase 2: 类型系统演示
+ export { demo_i8, demo_i16, ..., type_system_summary } from "./lib/type_system";
```

---

## Phase 3：信号快捷方法 + 可选链表达式（P0，覆盖 #4 #66 #67）

### 修改 `src/components/Counter.omni`（扩展）

覆盖项：#66 .inc()/.increment(), #67 .dec()/.decrement()

```diff
+ // 在已有 Counter 组件中添加信号快捷方法演示
+ signal altCount: i32 = 0;
+
+ // 在 view 中新增按钮：
+ <button class: btn @onPress={() => { altCount.inc(); }}>++ (via .inc())</button>
+ <button class: btn @onPress={() => { altCount.dec(); }}>-- (via .dec())</button>
+ <button class: btn @onPress={() => { altCount.increment(); }}>++ (via .increment())</button>
+ <button class: btn @onPress={() => { altCount.decrement(); }}>-- (via .decrement())</button>
+ 
+ <p class: count>altCount: {altCount}</p>
```

### 修改 `src/components/NullishDemo.omni`（扩展）

覆盖项：#4 `?.` 可选链表达式

```diff
+ // NullishDemo 注释中已写了"OptChain"但代码未实现，补上
+ signal nested: Map<String, String>? = null;
+ signal fallback: String = nested?.get("key") ?? "default value";
+ //                        ^^ ?? 已有，加上 ?. 即可
+
+ // 在 view 中展示
```

---

## Phase 4：标准库 — 全新模块（P1，覆盖 #88 #89 #90）

### 新增 `src/lib/regex_demo.omni`

覆盖项：#88 全部 std/regex 函数

```omni
// import { regex_match, regex_search, regex_replace } from "std/regex";
//
// pub fn test_regex_match() -> bool
//   → regex_match("hello@example.com", "^[\\w.]+@[\\w]+\\.\\w+$")
//
// pub fn test_regex_search() -> Array<String>
//   → regex_search("apple banana cherry apple", "apple")
//
// pub fn test_regex_replace() -> String
//   → regex_replace("Hello 2024 World 2024", "\\d{4}", "2025")
//
// pub fn validate_email(email: String) -> bool
// pub fn extract_numbers(text: String) -> Array<String>
// pub fn sanitize_input(text: String) -> String
```

### 新增 `src/lib/datetime_demo.omni`

覆盖项：#89 全部 std/datetime 函数

```omni
// import { datetime_now, datetime_from_timestamp, datetime_format,
//          datetime_year, datetime_month, datetime_day,
//          datetime_hour, datetime_minute, datetime_second } from "std/datetime";
//
// pub fn now_formatted() -> String
//   → datetime_format(datetime_now(), "%Y-%m-%d %H:%M:%S")
//
// pub fn get_date_parts() -> Map<String, i32>
//   → { year: datetime_year(now), month: datetime_month(now), ... }
//
// pub fn timestamp_to_string(ts: i64) -> String
//   → datetime_format(datetime_from_timestamp(ts))
//
// pub fn all_extractors() -> Map<String, i32>
//   → 调用全部 datetime_year/month/day/hour/minute/second
```

### 新增 `src/lib/encode_demo.omni`

覆盖项：#90 全部 std/encode 函数

```omni
// import { encode_base64_encode, encode_base64_decode,
//          encode_hex_encode, encode_hex_decode,
//          encode_url_encode, encode_url_decode } from "std/encode";
//
// pub fn base64_roundtrip(original: String) -> bool
//   → encode_base64_decode(encode_base64_encode(original)) == original
//
// pub fn hex_demo() -> String
//   → encode_hex_encode("Hello")
//
// pub fn hex_roundtrip(original: String) -> bool
//   → encode_hex_decode(encode_hex_encode(original)) == original
//
// pub fn url_encode_demo(url: String) -> String
//   → encode_url_encode(url)
//
// pub fn url_decode_demo(encoded: String) -> String
//   → encode_url_decode(encoded)
```

### 新增 `src/pages/StdlibExtensions.omni`

覆盖项：上述三个模块的组件级集成展示

```omni
// component StdlibExtensions
//   直接调用 regex_demo / datetime_demo / encode_demo 中的 pub fn
//   view { ... } ← 分三个区域展示结果
//   style { ... }
```

### 修改 `src/main.omni`

```diff
+ import { StdlibExtensions } from "./pages/StdlibExtensions";
+ // 在 demo-section 中添加 <StdlibExtensions />
```

---

## Phase 5：标准库 — 已有模块的补充函数（P1，覆盖 #91~#117）

### 新增 `src/lib/math_ext.omni`

覆盖项：#97~#104（14 个 math 函数）

```omni
// import { math_min, math_max, math_pow, math_sin, math_cos, math_tan,
//          math_log, math_log2, math_log10, math_pi, math_e,
//          math_clamp, math_random_int, math_round } from "std/math";
//
// pub fn demo_math_min_max(a: f64, b: f64) -> Map<String, f64>
// pub fn demo_trig(angle: f64) -> Map<String, f64>  // sin/cos/tan
// pub fn demo_logs(x: f64) -> Map<String, f64>      // log/log2/log10
// pub fn demo_constants() -> Map<String, f64>        // pi, e
// pub fn demo_clamp(val: f64, min: f64, max: f64) -> f64
// pub fn demo_random_in_range(min: i32, max: i32) -> i32
// pub fn demo_pow(base: f64, exp: f64) -> f64
```

### 新增 `src/lib/string_ext.omni`

覆盖项：#105~#112（8 个 string 函数）

```omni
// import { string_concat, string_split, string_join, string_replace,
//          string_substring, string_to_upper, string_to_lower,
//          string_ends_with } from "std/string";
//
// pub fn demo_split_join(text: String, sep: String) -> String
//   → string_join(string_split(text, sep), " | ")
// pub fn demo_replace(text: String, from: String, to: String) -> String
// pub fn demo_substring(text: String, start: i32, end: i32) -> String
// pub fn demo_case(text: String) -> Map<String, String>
//   → { upper: string_to_upper(text), lower: string_to_lower(text) }
// pub fn demo_ends_with(text: String, suffix: String) -> bool
// pub fn demo_concat(a: String, b: String) -> String
```

### 新增 `src/lib/array_ext.omni`

覆盖项：#92~#96（5 个 array 函数，不含 array_get）

```omni
// import { array_remove, array_map, array_find, array_contains } from "std/array";
//
// pub fn demo_remove(arr: Array<i32>, idx: i32) -> Array<i32>
//   → array_remove(arr, idx)
// pub fn demo_map(arr: Array<i32>) -> Array<i32>
//   → array_map(arr, |x| x * 2)
// pub fn demo_find(arr: Array<String>, target: String) -> i32
//   → array_find(arr, |s| s == target)
// pub fn demo_contains(arr: Array<String>, item: String) -> bool
//   → array_contains(arr, item)
```

### 新增 `src/lib/async_demo.omni`

覆盖项：#113~#117（全部 std/async 函数）

```omni
// import { async_sleep, async_spawn, async_all, async_race, async_timeout } from "std/async";
//
// pub async fn demo_sleep_and_return() -> String
//   → await async_sleep(10); return "slept 10ms"
//
// pub async fn demo_parallel() -> Array<String>
//   → let a = async_spawn(task_a()); let b = async_spawn(task_b());
//     return await async_all([a, b]);
//
// pub async fn demo_race() -> String
//   → let a = async_spawn(slow_task()); let b = async_spawn(fast_task());
//     return await async_race([a, b]);
//
// pub async fn demo_timeout() -> String
//   → let task = async_spawn(slow_task());
//     return await async_timeout(task, 50);  // 50ms 超时
```

### 修改 `src/lib/api.omni`

覆盖项：#91 http_send

```diff
+ // 新增 http_send(request) 演示
+ // pub async fn demo_http_send() -> String
+ //   let request = { method: "GET", url: "https://httpbin.org/get", headers: {} };
+ //   return await http_send(request);
```

### 新增 `src/components/StdlibFunctionsDemo.omni`

覆盖项：上述所有新增 lib 函数的组件级展示

```omni
// component StdlibFunctionsDemo
//   signal mathResults: Array<String> = [...]
//   signal stringResults: Array<String> = [...]
//   signal arrayResults: Array<String> = [...]
//   signal asyncResult: String = "..."
//   @mount { /* 调用 async demo，结果写入 signal */ }
//   view { ... } ← 分组展示
//   style { ... }
```

### 修改 `src/main.omni`

```diff
+ import { StdlibFunctionsDemo } from "./components/StdlibFunctionsDemo";
+ // 在 demo-section 中添加 <StdlibFunctionsDemo />
```

---

## Phase 6：视图属性 + 事件处理器（P1，覆盖 #13 #14 #18~#33）

### 新增 `src/components/ViewAttributesDemo.omni`

覆盖项：#13 testId, #14 class:"string", #18 bind:checked/bind:text

```omni
// component ViewAttributesDemo
//   signal isChecked: bool = false;
//   signal richText: String = "";
//
//   view {
//       <div class:"container">          ← ClassValue::StringLiteral
//           <h3 class:"section-title">View Attributes</h3>
//
//           <!-- testId 属性 -->
//           <div testId="attr-demo-container">  ← ElementProp::TestId
//               <p class:"info-text">此元素使用 testId 属性标记</p>
//           </div>
//
//           <!-- bind:checked (非 value) -->
//           <label class:"checkbox-label">
//               <input type="checkbox" bind:checked={isChecked} testId="agree-checkbox" />
//               <span>{"同意条款: " + isChecked.to_string()}</span>
//           </label>
//
//           <!-- bind:text (富文本输入) -->
//           <div testId="rich-text-area">
//               <textarea bind:value={richText} class:"rich-input" placeholder="输入文本..." />
//               <p class:"preview">预览: {richText}</p>
//           </div>
//       </div>
//   }
//
//   style {
//       ."container" { padding: 16px; border: 1px solid #ddd; }   ← 验证 class: "string" 生成的 CSS
//       ."section-title" { font-size: 18px; }
//       ."info-text" { font-size: 13px; color: #666; }
//       ."checkbox-label" { display: flex; gap: 8px; }
//       ."rich-input" { width: 100%; min-height: 60px; }
//       ."preview" { margin-top: 8px; background: #f0f0f0; padding: 8px; }
//   }
```

### 新增 `src/components/EventsDemo.omni`

覆盖项：#19~#33（15 个事件处理器），组合 32 个未使用的 HTML 标签

```omni
// component EventsDemo
//   signal log: Array<String> = [];
//   signal dragStatus: String = "idle";
//   signal wheelCount: i32 = 0;
//   signal copyCount: i32 = 0;
//   signal animStatus: String = "idle";
//   signal mediaStatus: String = "stopped";
//
//   fn addLog(msg: String): void { ... }
//
//   view {
//       <main>
//           <nav aria-label="Event Demo Navigation">
//               <h2>事件处理器全覆盖演示</h2>
//           </nav>
//
//           <!-- ═══ 输入事件 (#19, #20, #21, #32) ═══ -->
//           <article>
//               <h3>输入与键盘事件</h3>
//               <input placeholder="@onInput →" @onInput={addLog("input")}
//                      @onKeyUp={addLog("keyup")} @onKeyPress={addLog("keypress")}
//                      @onCopy={addLog("copy")} @onCut={addLog("cut")} @onPaste={addLog("paste")} />
//               <code>log: {lastLog}</code>
//           </article>
//
//           <!-- ═══ 双击 + 上下文菜单 (#22, #26) ═══ -->
//           <article>
//               <h3>双击与右键</h3>
//               <div @onDoubleClick={addLog("double-clicked")}
//                    @onContextMenu={addLog("right-clicked")}
//                    style: { background: "#eee", padding: "20px", cursor: "pointer" }>
//                   <strong>双击此区域</strong> 或 <em>右键点击</em>
//               </div>
//           </article>
//
//           <!-- ═══ 拖拽事件 (#23) ═══ -->
//           <article>
//               <h3>拖拽演示</h3>
//               <div @onDragStart={dragStatus = "dragging"}
//                    @onDragOver={dragStatus = "dragover"}
//                    @onDragEnd={dragStatus = "ended"}
//                    @onDrop={dragStatus = "dropped"}
//                    draggable="true"
//                    style: { background: "#cbd5e1", padding: "20px", margin: "8px 0" }>
//                   <small>拖拽状态: {dragStatus}</small>
//               </div>
//           </article>
//
//           <!-- ═══ 触摸事件 (#24) ═══ -->
//           <article>
//               <h3>触摸事件</h3>
//               <div @onTouchStart={addLog("touchstart")}
//                    @onTouchEnd={addLog("touchend")}
//                    @onTouchMove={addLog("touchmove")}
//                    style: { background: "#fed7aa", padding: "20px", userSelect: "none" }>
//                   <small>在移动设备上触摸此处</small>
//               </div>
//           </article>
//
//           <!-- ═══ 滚轮 (#25) ═══ -->
//           <article>
//               <h3>滚轮事件</h3>
//               <div @onWheel={wheelCount += 1}
//                    style: { background: "#ddd6fe", padding: "20px", overflow: "auto" }>
//                   <mark>滚轮计数: {wheelCount}</mark>
//               </div>
//           </article>
//
//           <!-- ═══ 媒体事件 (#27, #28, #29) ═══ -->
//           <article>
//               <h3>资源加载与媒体事件</h3>
//               <img src="about:blank" @onLoad={addLog("loaded")} @onError={addLog("error")}
//                    alt="test" style: { width: "1px", height: "1px" } />
//               <div @onPlay={mediaStatus = "playing"}
//                    @onPause={mediaStatus = "paused"}
//                    @onEnded={mediaStatus = "ended"}>
//                   <small>媒体状态: {mediaStatus}</small>
//               </div>
//           </article>
//
//           <!-- ═══ 动画 + 过渡事件 (#30, #31) ═══ -->
//           <article>
//               <h3>动画与过渡事件</h3>
//               <div @onAnimationStart={animStatus = "animation-start"}
//                    @onAnimationEnd={animStatus = "animation-end"}
//                    @onAnimationIteration={animStatus = "animation-iter"}
//                    @onTransitionEnd={animStatus = "transition-end"}
//                    class:"animated-box">
//                   <small>动画状态: {animStatus}</small>
//               </div>
//           </article>
//
//           <!-- ═══ Toggle 事件 (#33) ═══ -->
//           <details @onToggle={addLog("toggled")}>
//               <summary>点击展开/收起（@onToggle 事件）</summary>
//               <p>这是详细内容。<kbd>Esc</kbd> 关闭。</p>
//           </details>
//
//           <!-- ═══ 语义 HTML 标签一览 (#34~#65 在此组件中穿插使用) ═══ -->
//           <aside>
//               <h4>语义标签使用记录</h4>
//               <ol>
//                   <li><b>Bold</b> / <strong>Strong</strong> / <i>Italic</i> / <em>Emphasis</em></li>
//                   <li><mark>Marked</mark> / <small>Small</small> / <del>Deleted</del> / <ins>Inserted</ins></li>
//                   <li><code>code</code> / <kbd>kbd</kbd> / <samp>samp</samp> / <var>var</var></li>
//                   <li><sup>sup</sup> / <sub>sub</sub></li>
//                   <li><abbr title="HyperText Markup Language">HTML</abbr> / <dfn>definition</dfn></li>
//                   <li><blockquote cite="https://example.com">引用文本 <cite>— 来源</cite></blockquote></li>
//                   <li><q>内联引用</q></li>
//                   <li><time datetime="2025-06-20">2025年6月20日</time></li>
//                   <li><address>contact@example.com</address></li>
//                   <li><bdi>مرحبا</bdi> / <bdo dir="rtl">Right-to-left</bdo></li>
//                   <li>此处有<wbr>换行<wbr>机会</li>
//               </ol>
//           </aside>
//
//           <!-- ═══ 表单语义标签 ═══ -->
//           <fieldset>
//               <legend>表单分组（fieldset + legend）</legend>
//               <label>搜索: <input list="options" /></label>
//               <datalist id="options">
//                   <option value="Apple" />
//                   <option value="Banana" />
//               </datalist>
//               <select>
//                   <optgroup label="水果">
//                       <option>苹果</option>
//                   </optgroup>
//               </select>
//           </fieldset>
//
//           <!-- ═══ 媒体元素 ═══ -->
//           <figure>
//               <figcaption>媒体元素示例</figcaption>
//               <video controls style: { width: "200px" }>
//                   <source src="sample.mp4" type="video/mp4" />
//               </video>
//               <audio controls>
//                   <source src="sample.mp3" type="audio/mpeg" />
//               </audio>
//           </figure>
//
//           <!-- ═══ 画布 ═══ -->
//           <canvas style: { width: "100px", height: "50px", border: "1px solid #ccc" } />
//
//           <!-- ═══ 度量/进度 ═══ -->
//           <progress value="70" max="100">70%</progress>
//           <meter value="0.6" min="0" max="1">60%</meter>
//
//           <!-- ═══ 图片相关 ═══ -->
//           <picture>
//               <source srcset="large.jpg" media="(min-width: 800px)" />
//               <img src="fallback.jpg" alt="responsive" />
//           </picture>
//           <map name="demo-map">
//               <area shape="rect" coords="0,0,50,50" href="#area1" alt="Area 1" />
//           </map>
//
//           <!-- ═══ 注音 ═══ -->
//           <ruby>漢字<rt>かんじ</rt></ruby>
//       </main>
//   }
//
//   style {
//       ."animated-box" { 
//           width: 100px; height: 50px; background: #c084fc;
//           animation: pulse 2s infinite;
//       }
//       @keyframes pulse {
//           0% { opacity: 1; }
//           50% { opacity: 0.5; }
//           100% { opacity: 1; }
//       }
//   }
```

**覆盖明细（每项至少出现一次）：**
- #19~#33: 全部 15 个未使用事件处理器
- #34~#65: 全部 32 个未使用 HTML 语义标签
- #13: testId 专有属性
- #14: class:"string" 字面量语法（CSS 类名带引号）

### 修改 `src/main.omni`

```diff
+ import { ViewAttributesDemo } from "./components/ViewAttributesDemo";
+ import { EventsDemo } from "./components/EventsDemo";
+ // 在 demo-section 中添加 <ViewAttributesDemo /> 和 <EventsDemo />
```

---

## Phase 7：CSS 单位 + 字符串字面量（P2，覆盖 #5 #6 #18 #68~#87）

### 新增 `src/components/CssUnitsDemo.omni`

覆盖项：#68~#87（全部 20 个 CSS 单位）

```omni
// component CssUnitsDemo
//   view {
//       <div class: css-demo>
//           <h3>CSS 单位全覆盖</h3>
//
//           <div class: unit-box-vh>vh: 10vh</div>
//           <div class: unit-box-fr>fr（父级 grid）</div>
//           <div class: unit-box-pt>pt: 12pt</div>
//           <div class: unit-box-cm>cm: 1cm</div>
//           <div class: unit-box-mm>mm: 10mm</div>
//
//           <div class: unit-box-ms>transition: {ms}ms ({s}s)</div>
//           <div class: unit-box-deg>transform: rotate({deg}deg)</div>
//           <div class: unit-box-rad>transform: rotate({rad}rad)</div>
//           <div class: unit-box-grad>transform: rotate({grad}grad)</div>
//           <div class: unit-box-turn>transform: rotate({turn}turn)</div>
//           <div class: unit-box-vmin>vmin: 5vmin</div>
//           <div class: unit-box-svh>svh/svw: 5svh x 5svw</div>
//           <div class: unit-box-lvh>lvh/lvw: 5lvh x 5lvw</div>
//           <div class: unit-box-dvh>dvh/dvw: 5dvh x 5dvw</div>
//           <div class: unit-box-cq>cqw/cqh: 10cqw x 10cqh</div>
//           <div class: unit-box-cq2>cqi/cqb/cqmin/cqmax</div>
//           <div class: unit-box-ch>ch: 20ch</div>
//           <div class: unit-box-ex>ex: 10ex</div>
//           <div class: unit-box-ic>ic: 15ic</div>
//           <div class: unit-box-lh>lh/rlh: 2lh / 3rlh</div>
//       </div>
//   }
//   style {
//       .css-demo { container-type: inline-size; display: grid; grid-template-columns: 1fr 1fr 1fr; }
//       .unit-box-vh { height: 10vh; background: #fecaca; }
//       .unit-box-fr { background: #bbf7d0; }
//       .unit-box-pt { font-size: 12pt; background: #bfdbfe; }
//       .unit-box-cm { width: 1cm; background: #fef08a; }
//       .unit-box-mm { width: 10mm; background: #ddd6fe; }
//       .unit-box-ms { transition-duration: 150ms; background: #fed7aa; }
//       .unit-box-deg { transform: rotate(15deg); background: #fbcfe8; }
//       .unit-box-rad { transform: rotate(0.5rad); background: #a5f3fc; }
//       .unit-box-grad { transform: rotate(50grad); background: #d9f99d; }
//       .unit-box-turn { transform: rotate(0.25turn); background: #e9d5ff; }
//       .unit-box-vmin { width: 5vmin; height: 5vmin; background: #fde68a; }
//       .unit-box-svh { height: 5svh; background: #cffafe; }
//       .unit-box-lvh { height: 5lvh; background: #fce7f3; }
//       .unit-box-dvh { height: 5dvh; background: #d1fae5; }
//       .unit-box-cq { width: 10cqw; background: #ede9fe; }
//       .unit-box-cq2 { width: 5cqi; height: 5cqb; background: #ffedd5; }
//       .unit-box-ch { width: 20ch; background: #e0e7ff; }
//       .unit-box-ex { width: 10ex; background: #fee2e2; }
//       .unit-box-ic { width: 15ic; background: #dbeafe; }
//       .unit-box-lh { line-height: 2lh; background: #f3e8ff; }
//   }
```

### 新增 `src/components/StringLiteralsDemo.omni`

覆盖项：#5 反引号字符串, #6 单引号字符串

```omni
// component StringLiteralsDemo
//   signal backtickResult: String = `Hello "OmniView"`
//   signal singleQuoteResult: String = 'Single-quoted text'
//   signal mixed: String = `Backtick with 'single' and "double" quotes`
//   view { ... } ← 展示各种字符串字面量
```

### 修改 `src/main.omni`

```diff
+ import { CssUnitsDemo } from "./components/CssUnitsDemo";
+ import { StringLiteralsDemo } from "./components/StringLiteralsDemo";
+ // 在 demo-section 中添加 <CssUnitsDemo /> 和 <StringLiteralsDemo />
```

---

## 跳过项：@native / @fallback / @error（#15 #16 #17）

这三个指令是编译器解析器中的**预留占位符**，标记为 "P2 计划"。它们目前没有实际的语义分析和代码生成实现，无法产生有意义的编译输出。因此：

- **不在 learn-omniview 中添加示例**
- 在 `src/main.omni` 注释中注明这些指令为预留项
- 当编译器实际支持时再补充演示

---

## 构建与验证

### 构建命令

```bash
cd D:\space\OmniLang
cargo run -- target js --project test/projects/learn-omniview/build test/projects/learn-omniview/src/main.omni
```

### 分阶段验证

每个 Phase 完成后执行：
1. `cargo build` — 确认编译器无编译错误
2. `cargo run -- target js --project ...` — 确认 .omni → .js 编译成功
3. 检查 `test/projects/learn-omniview/build/index.js` 是否包含新增代码
4. 运行已有测试 `cargo test` — 确保 380+ 测试仍然通过

### 最终验证

全部 Phase 完成后：
1. 编译 learn-omniview 项目
2. 运行 `cargo test` — 全部测试通过
3. 如有 E2E 测试，运行 Playwright 测试

---

## 文件清单汇总

### 新增文件（18 个）

| # | 文件路径 | Phase | 类型 |
|---|---------|-------|------|
| 1 | `src/lib/error_handling.omni` | 1 | 纯逻辑 |
| 2 | `src/lib/union_types.omni` | 1 | 纯逻辑 |
| 3 | `src/components/ErrorHandlingDemo.omni` | 1 | 组件 |
| 4 | `src/lib/type_system.omni` | 2 | 纯逻辑 |
| 5 | `src/components/TypeSystemDemo.omni` | 2 | 组件 |
| 6 | `src/lib/regex_demo.omni` | 4 | 纯逻辑 |
| 7 | `src/lib/datetime_demo.omni` | 4 | 纯逻辑 |
| 8 | `src/lib/encode_demo.omni` | 4 | 纯逻辑 |
| 9 | `src/pages/StdlibExtensions.omni` | 4 | 页面 |
| 10 | `src/lib/math_ext.omni` | 5 | 纯逻辑 |
| 11 | `src/lib/string_ext.omni` | 5 | 纯逻辑 |
| 12 | `src/lib/array_ext.omni` | 5 | 纯逻辑 |
| 13 | `src/lib/async_demo.omni` | 5 | 纯逻辑 |
| 14 | `src/components/StdlibFunctionsDemo.omni` | 5 | 组件 |
| 15 | `src/components/ViewAttributesDemo.omni` | 6 | 组件 |
| 16 | `src/components/EventsDemo.omni` | 6 | 组件 |
| 17 | `src/components/CssUnitsDemo.omni` | 7 | 组件 |
| 18 | `src/components/StringLiteralsDemo.omni` | 7 | 组件 |

### 修改文件（5 个）

| # | 文件路径 | Phase | 修改内容 |
|---|---------|-------|---------|
| 1 | `src/main.omni` | 1–7 | 导入新组件、添加 Phase 10 section |
| 2 | `src/lib.omni` | 2 | 导出 type_system 函数 |
| 3 | `src/components/Counter.omni` | 3 | 添加 .inc()/.dec() 演示 |
| 4 | `src/components/NullishDemo.omni` | 3 | 添加 ?. 可选链演示 |
| 5 | `src/lib/api.omni` | 5 | 添加 http_send 演示 |

---

## 覆盖率总结

| 类别 | 总数 | 已覆盖 | 本次补充 | 最终 |
|------|------|--------|---------|------|
| 语句级构造 | 3 | 0 | 3 | 3 |
| 表达式级构造 | 3 | 0 | 3 | 3 |
| 类型注解构造 | 6 | 0 | 6 | 6 |
| 视图属性/指令 | 6 | 0 | 3 | 3 (3 预留) |
| 事件处理器 | 15 | 0 | 15 | 15 |
| HTML 标签 | 32 | 0 | 32 | 32 |
| 信号方法 | 2 | 0 | 2 | 2 |
| CSS 单位 | 20 | 0 | 20 | 20 |
| 标准库模块 | 3 | 0 | 3 | 3 |
| 标准库函数 | 27 | 0 | 27 | 27 |
| **合计** | **117** | **0** | **114** | **114** |

---

## 实施顺序建议

```
Phase 1 ──→ Phase 2 ──→ Phase 3
(error)     (types)     (signal+optional)

   ↓                      ↓
Phase 4 ──→ Phase 5 ──→ Phase 6 ──→ Phase 7
(stdlib)    (ext lib)   (view)      (css+string)
```

- Phase 1–3 可以部分并行（互不依赖）
- Phase 4–5 依赖 Phase 1–2 完成（共用 lib 模式）
- Phase 6–7 完全独立，可在 Phase 1 之后任意时间开始
- 每个 Phase 完成后独立构建验证
