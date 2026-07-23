import { Component, type ReactNode } from "react";
import { Logger } from "../../lib/utils/logger";
import { showToast } from "../../lib/toast";

const log = new Logger({ prefix: "ErrorBoundary" });

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo): void {
    log.error("Uncaught error:", error, errorInfo);
    showToast("应用发生错误，请刷新页面重试", "error");
  }

  handleRefresh = (): void => {
    window.location.reload();
  };

  handleRetry = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    if (this.state.error) {
      return (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            height: "100vh",
            background: "#f8fafc",
            fontFamily: "system-ui, -apple-system, sans-serif",
          }}
        >
          <div
            style={{
              background: "#fff",
              borderRadius: 16,
              padding: "32px 40px",
              maxWidth: 480,
              width: "90%",
              boxShadow: "0 4px 24px rgba(0,0,0,0.1)",
              textAlign: "center",
            }}
          >
            <div style={{ fontSize: 48, marginBottom: 16 }}>
              ⚠
            </div>
            <h2
              style={{
                fontSize: 20,
                fontWeight: 700,
                color: "#1f2937",
                margin: "0 0 8px",
              }}
            >
              应用出现异常
            </h2>
            <p
              style={{
                fontSize: 14,
                color: "#6b7280",
                margin: "0 0 24px",
                lineHeight: 1.5,
              }}
            >
              抱歉，应用遇到了意外错误。请尝试刷新页面或重试。
            </p>
            <div
              style={{
                display: "flex",
                gap: 12,
                justifyContent: "center",
                marginBottom: 16,
              }}
            >
              <button type="button"
                onClick={this.handleRefresh}
                style={{
                  padding: "8px 20px",
                  fontSize: 14,
                  fontWeight: 600,
                  border: "none",
                  borderRadius: 8,
                  background: "#4F46E5",
                  color: "#fff",
                  cursor: "pointer",
                }}
              >
                刷新页面
              </button>
              <button type="button"
                onClick={this.handleRetry}
                style={{
                  padding: "8px 20px",
                  fontSize: 14,
                  fontWeight: 500,
                  border: "1px solid #d1d5db",
                  borderRadius: 8,
                  background: "#fff",
                  color: "#374151",
                  cursor: "pointer",
                }}
              >
                重试
              </button>
            </div>
            <details
              style={{
                fontSize: 12,
                color: "#9ca3af",
                textAlign: "left",
                marginTop: 8,
              }}
            >
              <summary style={{ cursor: "pointer", marginBottom: 8 }}>
                错误详情
              </summary>
              <pre
                style={{
                  background: "#f9fafb",
                  padding: 12,
                  borderRadius: 8,
                  overflow: "auto",
                  maxHeight: 200,
                  fontSize: 12,
                  lineHeight: 1.4,
                  color: "#dc2626",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-all",
                  margin: 0,
                }}
              >
                {this.state.error.message}
                {this.state.error.stack ? `\n\n${this.state.error.stack}` : ""}
              </pre>
            </details>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
