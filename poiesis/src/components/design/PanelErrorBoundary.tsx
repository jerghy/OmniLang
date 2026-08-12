// components/design/PanelErrorBoundary.tsx
// Lightweight error boundary: one panel crash must not blank the whole page.
// 页面级不变式「一个面板崩溃不整页白屏」——所有面板挂载点（左栏组件树、
// 组件库弹出面板等）都应包裹本边界。提取自 DesignPage 内部类（M2 T6 复审）。
import { Component, type ReactNode } from "react";

export class PanelErrorBoundary extends Component<{ children: ReactNode; name: string }, { error: Error | null }> {
  constructor(props: { children: ReactNode; name: string }) {
    super(props);
    this.state = { error: null };
  }
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 16, color: "var(--ol-color-danger-hover)", fontSize: 12, background: "var(--ol-color-danger-subtle)", border: "1px solid var(--ol-color-danger-muted)", borderRadius: 4, margin: 4 }}>
          <strong>⚠ {this.props.name}</strong>
          <div style={{ marginTop: 4, opacity: 0.7 }}>{this.state.error.message}</div>
        </div>
      );
    }
    return this.props.children;
  }
}
