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

export const COMPONENT_PROTOCOL: Record<string, ComponentProtocolEntry> = {
  Container: {
    component: "Container", category: "Layout", description: "通用容器",
    props: {}, events: {},
    styleProperties: ["padding", "margin", "gap", "background_color", "border_radius", "width", "height"],
    children: true,
  },
  Flex: {
    component: "Flex", category: "Layout", description: "弹性布局",
    props: {
      direction: { type: "enum", values: ["row", "column"], default: "row", editor: "select" },
      gap: { type: "string", default: "8px", editor: "text" },
      align: { type: "enum", values: ["start", "center", "end", "stretch"], default: "start", editor: "select" },
      justify: { type: "enum", values: ["start", "center", "end", "between", "around"], default: "start", editor: "select" },
    },
    events: {},
    styleProperties: ["padding", "margin", "width", "height"],
    children: true,
  },
  Grid: {
    component: "Grid", category: "Layout", description: "网格布局",
    props: { columns: { type: "string", default: "2", editor: "text" }, gap: { type: "string", default: "16px", editor: "text" } },
    events: {},
    styleProperties: ["padding", "margin", "width", "height"],
    children: true,
  },
  Card: {
    component: "Card", category: "Layout", description: "卡片容器",
    props: {}, events: {},
    styleProperties: ["padding", "margin", "border_radius", "shadow", "background_color", "width", "height"],
    children: true,
  },
  Text: {
    component: "Text", category: "Typography", description: "文本",
    props: { label: { type: "string", default: "Text", editor: "text" } },
    events: {},
    styleProperties: ["font_size", "font_weight", "color", "text_align"],
    children: false,
  },
  Heading: {
    component: "Heading", category: "Typography", description: "标题",
    props: {
      level: { type: "enum", values: ["1", "2", "3", "4"], default: "1", editor: "select" },
      label: { type: "string", default: "Heading", editor: "text" },
    },
    events: {},
    styleProperties: ["font_size", "font_weight", "color"],
    children: false,
  },
  Button: {
    component: "Button", category: "Actions", description: "按钮",
    props: {
      label: { type: "string", default: "Button", editor: "text" },
      variant: { type: "enum", values: ["primary", "secondary", "danger", "ghost", "outline"], default: "primary", editor: "select" },
      disabled: { type: "bool", default: false, editor: "checkbox" },
    },
    events: {
      onPress: { id: "evt-btn-press-001", description: "点击", payload: "MouseEvent" },
      onHover: { id: "evt-btn-hover-001", description: "悬停", payload: null },
      onFocus: { id: "evt-btn-focus-001", description: "获得焦点", payload: null },
      onLongPress: { id: "evt-btn-longpress-001", description: "长按", payload: null },
    },
    styleProperties: ["width", "padding", "background_color", "border_radius", "font_size"],
    children: true,
  },
  Input: {
    component: "Input", category: "Forms", description: "输入框",
    props: {
      placeholder: { type: "string", default: "", editor: "text" },
      type: { type: "enum", values: ["text", "password", "email", "number"], default: "text", editor: "select" },
    },
    events: {
      onChange: { id: "evt-input-change-001", description: "值变化", payload: "String" },
      onFocus: { id: "evt-input-focus-001", description: "获得焦点", payload: null },
      onBlur: { id: "evt-input-blur-001", description: "失去焦点", payload: null },
    },
    styleProperties: ["width", "padding", "border_radius", "font_size"],
    children: false,
  },
  TextArea: {
    component: "TextArea", category: "Forms", description: "多行输入",
    props: { placeholder: { type: "string", default: "", editor: "text" }, rows: { type: "string", default: "3", editor: "text" } },
    events: { onChange: { id: "evt-textarea-change-001", description: "值变化", payload: "String" } },
    styleProperties: ["width", "padding"],
    children: false,
  },
  Select: {
    component: "Select", category: "Forms", description: "下拉选择",
    props: { options: { type: "string", default: "", editor: "text" }, placeholder: { type: "string", default: "", editor: "text" } },
    events: { onChange: { id: "evt-select-change-001", description: "值变化", payload: "String" } },
    styleProperties: ["width", "padding"],
    children: false,
  },
  Checkbox: {
    component: "Checkbox", category: "Forms", description: "复选框",
    props: { label: { type: "string", default: "", editor: "text" }, checked: { type: "bool", default: false, editor: "checkbox" } },
    events: { onChange: { id: "evt-checkbox-change-001", description: "值变化", payload: "bool" } },
    styleProperties: [],
    children: false,
  },
  List: {
    component: "List", category: "Data", description: "列表",
    props: {}, events: {},
    styleProperties: ["padding", "margin", "gap"],
    children: true,
  },
  ListItem: {
    component: "ListItem", category: "Data", description: "列表项",
    props: {}, events: {},
    styleProperties: ["padding", "margin"],
    children: true,
  },
  Image: {
    component: "Image", category: "Media", description: "图片",
    props: { src: { type: "string", default: "", editor: "text" }, alt: { type: "string", default: "", editor: "text" } },
    events: {},
    styleProperties: ["width", "height", "border_radius"],
    children: false,
  },
  Spinner: {
    component: "Spinner", category: "Feedback", description: "加载动画",
    props: {}, events: {},
    styleProperties: ["width", "height"],
    children: false,
  },
  Progress: {
    component: "Progress", category: "Feedback", description: "进度条",
    props: { value: { type: "string", default: "0", editor: "text" }, max: { type: "string", default: "100", editor: "text" } },
    events: {},
    styleProperties: ["width"],
    children: false,
  },
  Alert: {
    component: "Alert", category: "Feedback", description: "提示框",
    props: { type: { type: "enum", values: ["info", "success", "warning", "error"], default: "info", editor: "select" }, label: { type: "string", default: "", editor: "text" } },
    events: {},
    styleProperties: ["padding", "border_radius"],
    children: true,
  },
  Link: {
    component: "Link", category: "Navigation", description: "链接",
    props: { href: { type: "string", default: "#", editor: "text" }, label: { type: "string", default: "Link", editor: "text" } },
    events: {},
    styleProperties: ["color", "font_size"],
    children: false,
  },
  Form: {
    component: "Form", category: "Forms", description: "表单",
    props: {}, events: { onSubmit: { id: "evt-form-submit-001", description: "提交", payload: null } },
    styleProperties: ["padding", "gap", "width"],
    children: true,
  },
};
