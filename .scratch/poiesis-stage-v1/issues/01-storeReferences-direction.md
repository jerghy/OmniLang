---
name: storeReferences 方向反转
labels: needs-triage
severity: HIGH（潜伏）
found: M2 T2 质量审查（2026-08-12）
---

# storeReferences 方向反转（@store 接线后引用 store 源节点静默丢失）

## 规则/文件

- `poiesis/src/components/canvas/source-nodes.ts:71` — `storeReferences[componentName] ?? []`（读成 组件 → [store 名]）
- writer：`poiesis/src/store/slices/store-crud.ts:331-334` `registerStoreRef(componentName, storeName)` 存 `storeReferences[storeName] = [...existing, componentName]`（**store → [组件名]**，方向相反）
- 同款错误已复制于：`shared.ts`（useSignalItems）、`useHandlerItems.ts:49`、`FlowCanvas.tsx:734`（4 个消费者全反）
- 测试 fixture 同样编码错误假设（source-nodes.test.ts:12 `storeReferences: { Card: ["TodoStore"] }`）——所以测试全绿无法捕获

## 置信度

**CONFIRMED（源码级）**：唯一 writer 方向 = store → 组件列表；唯一活读者 flow-scope-hook.ts:18-22 用 `Object.entries(...).includes(componentName)`（正确读法）。

## 影响

**潜伏**：`registerStoreRef` 当前**零调用者**（map 运行时恒空）→ 今日不炸。但 @store 接线启用后（M3 数据绑定 / @store 声明加载），引用 store 的源节点将**静默不收集**（空数组漏，非崩溃）——比崩溃更难发现。

## 建议修复（择一，接线前必须）

1. 翻转消费者为 `Object.entries(storeReferences).filter(([, comps]) => comps.includes(componentName)).map(([storeName]) => storeName)`（4 处：source-nodes.ts / shared.ts / useHandlerItems.ts / FlowCanvas.tsx）
2. 或翻转 writer 键为 componentName（registerStoreRef + flow-scope-hook 同步）

修复时同步翻转测试 fixture 为 writer 真实形状。

## 提出修复

T2 提交保持原样（忠实复制 shared.ts 语义，审查 Approved）；本 issue 在 @store 接线任务前处理。
