import { Component, type ReactNode } from "react";
import { Logger } from "../../lib/utils/logger";
import { showToast } from "../../lib/toast";
import styles from "./ErrorBoundary.module.css";

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
        <div className={styles.wrapper}>
          <div className={styles.card}>
            <div className={styles.icon}>
              ⚠
            </div>
            <h2 className={styles.heading}>
              应用出现异常
            </h2>
            <p className={styles.message}>
              抱歉，应用遇到了意外错误。请尝试刷新页面或重试。
            </p>
            <div className={styles.actions}>
              <button type="button" onClick={this.handleRefresh} className={styles.btnPrimary}>
                刷新页面
              </button>
              <button type="button" onClick={this.handleRetry} className={styles.btnSecondary}>
                重试
              </button>
            </div>
            <details className={styles.details}>
              <summary className={styles.summary}>
                错误详情
              </summary>
              <pre className={styles.errorBlock}>
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
