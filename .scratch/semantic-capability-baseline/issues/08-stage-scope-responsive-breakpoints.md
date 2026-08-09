# 08 — 批次 3 域裁决：响应式断点

Type: grilling
Status: resolved
Blocked by: 04, 05

## Question

**响应式与样式断点**能力的边界裁决与设计方向——按 05 定稿的裁决模板执行。

覆盖能力项：
- 断点条件（`@if bp("md")` 条件表达式或变体维度——2026-08-08 §4.4 草案：design 域 `breakpoint md: 768` token + 使用形态）
- 用户级 `@media` 断点样式（现状：仅主题生成 prefers-color-scheme，无用户级媒体查询——属样式域，跨域协调）
- 容器查询/自适应布局（研究 02 的样式生态全景输入）

## 决议内容

1. 逐项边界裁决（进语言/留生态/不支持 + 三判据证据）
2. 入语言项的 1-2 句设计方向（breakpoint token + `bp("md")` → React matchMedia/Flutter MediaQuery/OmniView matchMedia）
3. 与样式批次（车票 10）的边界划分（断点条件 vs @media 块样式谁管）
4. 本批次阶段 spec 的范围边界

## 解析即

决议记录 `## Answer` → `Status: resolved` → 地图 Decisions-so-far 追加一行 → 本批次即成为路线图一个阶段（后续独立 spec→plan→SDD）。

## Answer

**批次 3（响应式断点）域裁决定案**（2026-08-09，校准后标准——意图全进语言，机制语法面透传）。

| 能力项 | 结论 | 三目标映射方向 |
|---|---|---|
| 断点条件 bp("md") | 🔜 **进语言**（逻辑级断点条件——设计域 `breakpoint md: 768` token + `@if bp("md")` 使用形态） | React useMediaQuery/matchMedia hook / Flutter MediaQuery / OmniView matchMedia |
| breakpoint token | 🔜 **进语言**（design 域扩展——token 体系自然延伸） | 编译期 token 表 → 各目标断点常量 |
| 用户级 @media 样式 | 🏞 机制语法面（CSS 透传——web 表达通道）——**但响应式样式意图的 Flutter 路径存在**（MediaQuery + LayoutBuilder 分支），映射策略批次 spec 细化（编译器把断点条件样式映射为 MediaQuery 分支，或诚实降级注记） | web: @media 透传 / Flutter: MediaQuery 映射策略细化 |
| 容器查询 | 🔜 进语言候选（低优先——意图「组件自适应容器尺寸」） | web: CSS @container / Flutter: LayoutBuilder——批次 spec 细化形态（或编译器选路优化） |

**批次范围**：bp() 逻辑断点 + breakpoint token 为主体；@media 映射策略与样式批次（10）协调——bp() 是跨目标主通道，@media 是 web 透传通道。semantic-universe 域 12 响应式登记随批次 spec。虚拟化（@for 虚拟化属性）用户已拍板「先成熟后议」——保持后议。
