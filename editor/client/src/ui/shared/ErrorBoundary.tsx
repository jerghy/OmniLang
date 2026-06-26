/** 应用级 ErrorBoundary。
 *  捕获渲染错误，显示恢复 UI，保留 store 状态避免工作丢失。 */

import { Component, type ReactNode, type ErrorInfo } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("[ErrorBoundary] 捕获渲染错误:", error.message, info.componentStack);
    // 不清理 store 状态 —— 用户的工作保留在内存中
  }

  handleReset = (): void => {
    this.setState({ hasError: false, error: null });
  };

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div
          style={{
            padding: 40,
            textAlign: "center",
            fontFamily: "system-ui, sans-serif",
            maxWidth: 600,
            margin: "80px auto",
          }}
        >
          <h2 style={{ color: "#DC2626", marginBottom: 12 }}>渲染错误</h2>
          <pre
            style={{
              fontSize: 12,
              color: "#6B7280",
              background: "#F3F4F6",
              padding: 12,
              borderRadius: 8,
              marginBottom: 16,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {this.state.error?.message}
          </pre>
          <div style={{ color: "#9CA3AF", fontSize: 13, marginBottom: 20 }}>
            您的工作已保存。您可以安全地重试或重新加载页面。
          </div>
          <div style={{ display: "flex", gap: 12, justifyContent: "center" }}>
            <button
              onClick={this.handleReset}
              style={{
                padding: "8px 20px",
                background: "#2563EB",
                color: "#fff",
                border: "none",
                borderRadius: 6,
                cursor: "pointer",
                fontSize: 14,
              }}
            >
              重试
            </button>
            <button
              onClick={() => window.location.reload()}
              style={{
                padding: "8px 20px",
                background: "#E5E7EB",
                color: "#374151",
                border: "none",
                borderRadius: 6,
                cursor: "pointer",
                fontSize: 14,
              }}
            >
              重新加载
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
