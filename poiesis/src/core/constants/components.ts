export const CATEGORY_LABELS: Record<string, string> = {
  Layout: "布局", Typography: "排版", Actions: "操作", Forms: "表单",
  Data: "数据", Media: "媒体", Feedback: "反馈", Navigation: "导航",
  Custom: "★ 自定义",
};
export const CATEGORY_ORDER = ["Custom", "Layout", "Typography", "Actions", "Forms", "Data", "Media", "Feedback", "Navigation"];
/** Icons keyed by OmniLang element tag name (not semantic alias). */
export const COMPONENT_ICONS: Record<string, string> = {
  div: "📦", span: "🔤", p: "📝",
  h1: "📰", h2: "📰", h3: "📰", h4: "📰", h5: "📰", h6: "📰",
  button: "🔘", input: "📋", textarea: "📄", select: "📌",
  ul: "📃", ol: "📋", li: "📎",
  img: "🖼",
  progress: "📊",
  a: "🔗", form: "📨",
  section: "🧱", article: "📄", header: "🧱", footer: "🧱", nav: "🧭", main: "🧱",
  label: "🏷",
};
