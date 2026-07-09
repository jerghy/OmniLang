/** 组件协议 — 内置组件的单一真相源。
 *
 *  被 ComponentPalette, PropertyPanel, LayoutView, componentRegistry,
 *  connectionValidators, OmniDriver 消费。
 *
 *  自定义组件通过 componentRegistry 添加，不在本文件中。 */

export interface EventMeta {
  id: string;
  description: string;
  payload: string | null;
}

export interface PropMeta {
  type: string;
  default?: unknown;
  values?: string[];
  editor: string;
}

export interface ComponentProtocolEntry {
  component: string;
  category: string;
  description: string;
  props: Record<string, PropMeta>;
  events: Record<string, EventMeta>;
  styleProperties: string[];
  children: boolean;
  /** True for user-defined components (not built-in). */
  isCustom?: boolean;
  /** Recommended design system class ref for this component. */
  recommendedClass?: string;
  /** Recommended design system variant binding for this component. */
  recommendedVariant?: {
    designName: string;
    variantName: string;
    defaultParams: Record<string, string>;
  };
}

/**
 * OmniView 通用事件 — 所有组件都可以使用，无组件类型限制。
 *
 * 按 DOM 映射分类：点击 / 输入 / 焦点 / 键盘 / 表单 / 滚动 / 鼠标 / 拖拽 / 触摸 / 滚轮 / 媒体 / 动画 / 切换 / 剪贴板
 *
 * 注意：onCopy、onCut、onPaste 等虽然没有被显式列出，但编译器会在兜底规则中
 * strip "on" prefix 并转小写来匹配 DOM 事件。此处列出最常见的事件作为编辑器的可见选项。
 */
export const UNIVERSAL_EVENTS: Record<string, EventMeta> = {
  // ── 点击 ──
  onPress: { id: "evt-universal-press-001", description: "点击/按压", payload: "MouseEvent" },
  onClick: { id: "evt-universal-click-001", description: "点击", payload: "MouseEvent" },
  onTap: { id: "evt-universal-tap-001", description: "点击(触摸)", payload: "TouchEvent" },
  onDoubleClick: { id: "evt-universal-dblclick-001", description: "双击", payload: "MouseEvent" },
  onContextMenu: { id: "evt-universal-contextmenu-001", description: "右键菜单", payload: "MouseEvent" },
  // ── 输入 ──
  onChange: { id: "evt-universal-change-001", description: "输入变化", payload: "String" },
  onInput: { id: "evt-universal-input-001", description: "输入事件", payload: "InputEvent" },
  // ── 焦点 ──
  onFocus: { id: "evt-universal-focus-001", description: "获得焦点", payload: "FocusEvent" },
  onBlur: { id: "evt-universal-blur-001", description: "失去焦点", payload: "FocusEvent" },
  // ── 键盘 ──
  onKeyDown: { id: "evt-universal-keydown-001", description: "按键按下", payload: "KeyboardEvent" },
  onKeyUp: { id: "evt-universal-keyup-001", description: "按键释放", payload: "KeyboardEvent" },
  // ── 表单 ──
  onSubmit: { id: "evt-universal-submit-001", description: "表单提交", payload: null },
  // ── 滚动 ──
  onScroll: { id: "evt-universal-scroll-001", description: "滚动", payload: "WheelEvent" },
  // ── 鼠标 ──
  onMouseEnter: { id: "evt-universal-mouseenter-001", description: "鼠标进入", payload: null },
  onMouseLeave: { id: "evt-universal-mouseleave-001", description: "鼠标离开", payload: null },
  onMouseDown: { id: "evt-universal-mousedown-001", description: "鼠标按下", payload: "MouseEvent" },
  onMouseUp: { id: "evt-universal-mouseup-001", description: "鼠标释放", payload: "MouseEvent" },
  onMouseMove: { id: "evt-universal-mousemove-001", description: "鼠标移动", payload: "MouseEvent" },
  // ── 拖拽 ──
  onDragStart: { id: "evt-universal-dragstart-001", description: "拖拽开始", payload: null },
  onDragOver: { id: "evt-universal-dragover-001", description: "拖拽悬停", payload: null },
  onDragEnd: { id: "evt-universal-dragend-001", description: "拖拽结束", payload: null },
  onDrop: { id: "evt-universal-drop-001", description: "放下", payload: null },
  // ── 触摸 ──
  onTouchStart: { id: "evt-universal-touchstart-001", description: "触摸开始", payload: "TouchEvent" },
  onTouchEnd: { id: "evt-universal-touchend-001", description: "触摸结束", payload: "TouchEvent" },
  onTouchMove: { id: "evt-universal-touchmove-001", description: "触摸移动", payload: "TouchEvent" },
  // ── 滚轮 ──
  onWheel: { id: "evt-universal-wheel-001", description: "滚轮", payload: "WheelEvent" },
  // ── 媒体（Image / Video / Audio） ──
  onLoad: { id: "evt-universal-load-001", description: "加载完成", payload: null },
  onError: { id: "evt-universal-error-001", description: "加载错误", payload: "ErrorEvent" },
  onPlay: { id: "evt-universal-play-001", description: "播放", payload: null },
  onPause: { id: "evt-universal-pause-001", description: "暂停", payload: null },
  onEnded: { id: "evt-universal-ended-001", description: "播放结束", payload: null },
  // ── 动画 ──
  onAnimationStart: { id: "evt-universal-animationstart-001", description: "动画开始", payload: null },
  onAnimationEnd: { id: "evt-universal-animationend-001", description: "动画结束", payload: null },
  onAnimationIteration: { id: "evt-universal-animationiteration-001", description: "动画迭代", payload: null },
  onTransitionEnd: { id: "evt-universal-transitionend-001", description: "过渡结束", payload: null },
  // ── 切换 ──
  onToggle: { id: "evt-universal-toggle-001", description: "切换", payload: null },
  // ── 剪贴板 ──
  onCopy: { id: "evt-universal-copy-001", description: "复制", payload: "ClipboardEvent" },
  onCut: { id: "evt-universal-cut-001", description: "剪切", payload: "ClipboardEvent" },
  onPaste: { id: "evt-universal-paste-001", description: "粘贴", payload: "ClipboardEvent" },
};

/**
 * 获取组件的完整事件列表 = 通用事件 + 组件特殊事件。
 * 组件特殊事件会覆盖同名的通用事件定义（例如 Button 的 onPress 使用更具体的 evt-btn-press-001 ID）。
 */
export function getComponentEvents(componentName: string): Record<string, EventMeta> {
  const specialEvents = COMPONENT_PROTOCOL[componentName]?.events ?? {};
  return { ...UNIVERSAL_EVENTS, ...specialEvents };
}

/**
 * Built-in component protocol keyed by OmniLang tag name.
 *
 * These are the actual element names used in .omni source code, not semantic aliases.
 * When a user drags a component from the palette, the element's `tag` field is set
 * to the key below, ensuring the generated .omni code uses the correct tag.
 *
 * Custom components (PascalCase) are registered separately via componentDefs.
 */
export const COMPONENT_PROTOCOL: Record<string, ComponentProtocolEntry> = {
  div: {
    component: "div", category: "Layout", description: "通用容器 / 布局块",
    props: {},
    events: {},
    styleProperties: [],
    children: true,
    recommendedClass: "AppDesign.Card",
  },
  span: {
    component: "span", category: "Typography", description: "内联文本 / 行内元素",
    props: { label: { type: "string", default: "text", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  p: {
    component: "p", category: "Typography", description: "段落 / 块级文本",
    props: { label: { type: "string", default: "Text", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  h1: {
    component: "h1", category: "Typography", description: "一级标题",
    props: { label: { type: "string", default: "Heading", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  h2: {
    component: "h2", category: "Typography", description: "二级标题",
    props: { label: { type: "string", default: "Heading", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  h3: {
    component: "h3", category: "Typography", description: "三级标题",
    props: { label: { type: "string", default: "Heading", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  h4: {
    component: "h4", category: "Typography", description: "四级标题",
    props: { label: { type: "string", default: "Heading", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  h5: {
    component: "h5", category: "Typography", description: "五级标题",
    props: { label: { type: "string", default: "Heading", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  h6: {
    component: "h6", category: "Typography", description: "六级标题",
    props: { label: { type: "string", default: "Heading", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  button: {
    component: "button", category: "Actions", description: "按钮",
    props: {
      label: { type: "string", default: "Button", editor: "text" },
      disabled: { type: "bool", default: false, editor: "checkbox" },
    },
    events: {
      onPress: { id: "evt-btn-press-001", description: "点击", payload: "MouseEvent" },
      onHover: { id: "evt-btn-hover-001", description: "悬停", payload: null },
      onFocus: { id: "evt-btn-focus-001", description: "获得焦点", payload: null },
      onLongPress: { id: "evt-btn-longpress-001", description: "长按", payload: null },
    },
    styleProperties: [],
    children: true,
    recommendedVariant: { designName: "AppDesign", variantName: "Button", defaultParams: { color: "primary", size: "md" } },
  },
  input: {
    component: "input", category: "Forms", description: "输入框",
    props: {
      placeholder: { type: "string", default: "", editor: "text" },
      type: { type: "enum", values: ["text", "password", "email", "number", "checkbox"], default: "text", editor: "select" },
    },
    events: {
      onChange: { id: "evt-input-change-001", description: "值变化", payload: "String" },
      onFocus: { id: "evt-input-focus-001", description: "获得焦点", payload: null },
      onBlur: { id: "evt-input-blur-001", description: "失去焦点", payload: null },
    },
    styleProperties: [],
    children: false,
    recommendedClass: "AppDesign.Input",
  },
  textarea: {
    component: "textarea", category: "Forms", description: "多行输入",
    props: { placeholder: { type: "string", default: "", editor: "text" }, rows: { type: "string", default: "3", editor: "text" } },
    events: { onChange: { id: "evt-textarea-change-001", description: "值变化", payload: "String" } },
    styleProperties: [],
    children: false,
  },
  select: {
    component: "select", category: "Forms", description: "下拉选择",
    props: { options: { type: "string", default: "", editor: "text" }, placeholder: { type: "string", default: "", editor: "text" } },
    events: { onChange: { id: "evt-select-change-001", description: "值变化", payload: "String" } },
    styleProperties: [],
    children: false,
  },
  ul: {
    component: "ul", category: "Data", description: "无序列表",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  ol: {
    component: "ol", category: "Data", description: "有序列表",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  li: {
    component: "li", category: "Data", description: "列表项",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  img: {
    component: "img", category: "Media", description: "图片",
    props: { src: { type: "string", default: "", editor: "text" }, alt: { type: "string", default: "", editor: "text" } },
    events: {},
    styleProperties: [],
    children: false,
  },
  progress: {
    component: "progress", category: "Feedback", description: "进度条",
    props: { value: { type: "string", default: "50", editor: "text" } },
    events: {},
    styleProperties: [],
    children: false,
  },
  a: {
    component: "a", category: "Navigation", description: "链接 / 超链接",
    props: { href: { type: "string", default: "#", editor: "text" }, label: { type: "string", default: "Link", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
  form: {
    component: "form", category: "Forms", description: "表单",
    props: {}, events: { onSubmit: { id: "evt-form-submit-001", description: "提交", payload: null } },
    styleProperties: [],
    children: true,
  },
  section: {
    component: "section", category: "Layout", description: "页面分区",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  article: {
    component: "article", category: "Layout", description: "文章 / 独立内容",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  header: {
    component: "header", category: "Layout", description: "页头",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  footer: {
    component: "footer", category: "Layout", description: "页脚",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  nav: {
    component: "nav", category: "Navigation", description: "导航",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  main: {
    component: "main", category: "Layout", description: "主内容区",
    props: {}, events: {},
    styleProperties: [],
    children: true,
  },
  label: {
    component: "label", category: "Forms", description: "表单标签",
    props: { label: { type: "string", default: "Label", editor: "text" }, for: { type: "string", default: "", editor: "text" } },
    events: {},
    styleProperties: [],
    children: true,
  },
};
