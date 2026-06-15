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
