# OmniView Cross-Framework Comparison

> 同一个 `.omni` 源文件 → React + Zustand vs Vue 3 + Pinia 生成代码对比

---

## 源文件: `02_Shop.omni`

```omni
store CartStore {
    signal item_count: i32 = 0;
    signal discount: i32 = 0;
    getter cart_total: i32 = discount;
    getter cart_count: i32 = item_count;
    action add_item() { item_count = item_count + 1; }
    action clear_cart() { item_count = 0; }
}

component ShopApp {
    @store CartStore;
    signal search: String = "";
    fn handle_add() { store.add_item(); }
    fn handle_clear() { store.clear_cart(); }
    view { /* Container, Flex, Grid, Heading, Text, Button, Input, Spacer */ }
    style { .product_card { padding: 16px; border_radius: 12px; shadow: md; } }
}
```

---

## 状态管理

### OmniView Store → Zustand (React)

```typescript
import { create } from "zustand";

interface CartStoreState {
  item_count: number;
  discount: number;
  cart_total: () => number;
  cart_count: () => number;
  add_item: () => void;
  clear_cart: () => void;
}

export const useCartStore = create<CartStoreState>()((set, get) => ({
  item_count: 0,
  discount: 0,
  cart_total: () => {
    const state = get();
    return state.discount;
  },
  cart_count: () => {
    const state = get();
    return state.item_count;
  },
  add_item: () => {
    set(state => ({ item_count: (state.item_count + 1) }));
  },
  clear_cart: () => {
    set(state => ({ item_count: 0 }));
  },
}));
```

### OmniView Store → Pinia (Vue)

```typescript
import { ref, computed } from "vue";
import { defineStore } from "pinia";

export const useCartStore = defineStore('CartStore', () => {
  const item_count = ref<number>(0);
  const discount = ref<number>(0);
  const cart_total = computed(() => discount.value);
  const cart_count = computed(() => item_count.value);

  function add_item() {
    item_count.value = (item_count.value + 1);
  }
  function clear_cart() {
    item_count.value = 0;
  }

  return { item_count, discount, cart_total, cart_count, add_item, clear_cart };
});
```

| 概念 | React (Zustand) | Vue (Pinia) |
|------|----------------|-------------|
| signal | `state.field` in `set()` | `ref<T>(init)` + `.value` |
| getter | `fn() => state.x` | `computed(() => x.value)` |
| action | `set(state => ({...}))` | function with `.value` mutations |

---

## 组件状态 & 方法

### React

```typescript
export function ShopApp() {
    const cart_count = useCartStore(s => s.cart_count());
    const cart_total = useCartStore(s => s.cart_total());
    const [search, setSearch] = useState<string>("");

    const handle_add = useCallback(() => {
        useCartStore.getState().add_item();
    }, []);
    const handle_clear = useCallback(() => {
        useCartStore.getState().clear_cart();
    }, []);
    // ...
}
```

### Vue

```vue
<script setup lang="ts">
const store = useCartStore();
const search = ref<string>("");

function handle_add() {
    store.add_item();
}
function handle_clear() {
    store.clear_cart();
}
</script>
```

| 概念 | React | Vue |
|------|-------|-----|
| 组件信号 | `useState<T>(init)` + setter | `ref<T>(init)` + `.value` |
| Store 读取 | `useStore(s => s.field())` (精确选择器) | `store.field` (自动解包) |
| Store 写入 | `useStore.getState().action()` | `store.action()` |
| 函数定义 | `useCallback(() => {...}, [deps])` | `function name() {...}` (天然稳定) |

---

## 视图模板

### 双向绑定

| OmniView | React | Vue |
|----------|-------|-----|
| `<Input bind:value={search} />` | `value={search} onChange={e => setSearch(e.target.value)}` | `v-model="search"` |

### 条件渲染

| OmniView | React | Vue |
|----------|-------|-----|
| `@if cond { <A/> }` | `{cond ? (<A/>) : null}` | `<template v-if="cond"><A/></template>` |

### 列表渲染

| OmniView | React | Vue |
|----------|-------|-----|
| `@for item in items { <Li key={item.id}/> }` | `{items.map(item => <li key={item.id}/>)}` | `<template v-for="item in items" :key="item.id"><li/></template>` |

### 事件处理

| OmniView | React | Vue |
|----------|-------|-----|
| `<Button onPress={fn}>` | `<button onClick={fn}>` | `<button @click="fn">` |

### 测试属性

| OmniView | React | Vue |
|----------|-------|-----|
| `test-id="btn"` | `data-testid="btn"` | `data-testid="btn"` |

---

## 样式

### OmniView

```omni
style {
    .product_card {
        layout: column;
        padding: 16px;
        border_radius: 12px;
        shadow: md;
    }
}
```

### React (CSS Modules)

```css
.product_card {
  display: flex;
  flex-direction: column;
  padding: 16px;
  border-radius: 12px;
  box-shadow: 0 4px 6px rgba(0,0,0,0.1);
}
```
引用: `className={styles.product_card}`

### Vue (Scoped CSS)

```css
.product_card {
  display: flex;
  flex-direction: column;
  padding: 16px;
  border-radius: 12px;
  box-shadow: 0 4px 6px rgba(0,0,0,0.1);
}
```
引用: `class="product_card"` (自动匹配)

---

## 完整编译结果对比

| 维度 | React | Vue |
|------|-------|-----|
| 输出格式 | `.tsx` + `.module.css` + `useStore.ts` | `.vue` SFC + `stores/useStore.ts` |
| 状态管理 | Zustand `create()` | Pinia `defineStore()` |
| 组件函数 | `export function Name()` | `<script setup>` |
| 响应式 | `useState` / `useMemo` | `ref()` / `computed()` |
| 副作用 | `useEffect(fn, [deps])` | `watch([deps], fn)` / `onMounted(fn)` |
| 函数缓存 | `useCallback(fn, deps)` | 不需要（Vue 天然稳定） |
| 双向绑定 | `value={x} onChange={e => setX(...)}` | `v-model="x"` |
| 条件渲染 | 三元表达式 `{cond ? ... : null}` | `v-if` / `v-else` 指令 |
| 列表渲染 | `.map()` 方法 | `v-for` 指令 |
| 事件绑定 | `onClick={fn}` | `@click="fn"` |
| 样式隔离 | CSS Modules | `<style scoped>` |
| 类型系统 | TypeScript | TypeScript |
| 构建工具 | Vite | Vite |

---

## 差异率评估

以 Shop 组件为基准：

| 目标 | 差异率 | 主要差异来源 |
|------|--------|------------|
| React | ~5% | snake_case 命名 vs camelCase |
| Vue | ~5% | snake_case 命名 vs camelCase |

差异率定义：生成的代码与同功能手写代码的结构差异比例。命名约定（snake_case vs camelCase）是主要差异源。函数结构、数据流、样式组织均符合各自框架的地道惯例。
