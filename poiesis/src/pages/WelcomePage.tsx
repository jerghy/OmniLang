// pages/WelcomePage.tsx
// Landing page shown when no project is open.
// Three entry points: open folder, new project, recent projects.

import { useState, useEffect, useCallback } from "react";
import { api } from "../commands";
import type { RecentProject } from "../commands/project-api";
import { createLogger } from "../lib/utils/logger";

const log = createLogger("WelcomePage");

interface WelcomePageProps {
  onProjectOpened: (projectPath: string, projectName: string) => void;
}

export function WelcomePage({ onProjectOpened }: WelcomePageProps) {
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const [showNewDialog, setShowNewDialog] = useState(false);
  const [newProjectParent, setNewProjectParent] = useState("");
  const [newProjectName, setNewProjectName] = useState("");
  const [error, setError] = useState("");

  const loadRecent = useCallback(async () => {
    const list = await api.project.getRecentProjects();
    setRecentProjects(list);
  }, []);

  useEffect(() => {
    loadRecent();
  }, [loadRecent]);

  const handleOpenProject = async () => {
    try {
      const folder = await api.project.openFolderDialog();
      if (!folder) return;
      const name = folder.replace(/\\/g, "/").split("/").pop() || folder;
      await api.project.setProjectDir(folder);
      await api.project.addRecentProject(name, folder);
      onProjectOpened(folder, name);
    } catch (e) {
      log.warn("Operation failed:", e);
      setError("操作失败，请重试");
    }
  };

  const handleOpenNewDialog = () => {
    setNewProjectParent("");
    setNewProjectName("");
    setError("");
    setShowNewDialog(true);
  };

  const handlePickLocation = async () => {
    const folder = await api.project.openFolderDialog();
    if (folder) setNewProjectParent(folder);
  };

  const handleCreateProject = async () => {
    try {
      if (!newProjectName.trim()) {
        setError("请输入项目名称");
        return;
      }
      const result = await api.project.createProject(newProjectParent, newProjectName.trim());
      if (!result.ok || !result.dir) {
        setError(result.error ?? "创建失败");
        return;
      }
      await api.project.addRecentProject(newProjectName.trim(), result.dir);
      setShowNewDialog(false);
      onProjectOpened(result.dir, newProjectName.trim());
    } catch (e) {
      log.warn("Operation failed:", e);
      setError("操作失败，请重试");
    }
  };

  const handleOpenRecent = async (proj: RecentProject) => {
    try {
      await api.project.setProjectDir(proj.path);
      await api.project.addRecentProject(proj.name, proj.path);
      onProjectOpened(proj.path, proj.name);
    } catch (e) {
      log.warn("Operation failed:", e);
      setError("操作失败，请重试");
    }
  };

  const handleRemoveRecent = async (e: React.MouseEvent, path: string) => {
    e.stopPropagation();
    await api.project.removeRecentProject(path);
    await loadRecent();
  };

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        {/* Logo / Hero */}
        <div style={styles.hero}>
          <div style={styles.logo}>✦</div>
          <h1 style={styles.title}>Poiesis</h1>
          <p style={styles.subtitle}>视觉化 OmniLang 开发环境</p>
        </div>

        {/* Action Cards */}
        <div style={styles.actions}>
          <button type="button" style={styles.card} onClick={handleOpenProject}>
            <span style={styles.cardIcon}>📂</span>
            <span style={styles.cardTitle}>打开项目</span>
            <span style={styles.cardDesc}>选择已有项目文件夹</span>
          </button>

          <button type="button" style={styles.card} onClick={handleOpenNewDialog}>
            <span style={styles.cardIcon}>✨</span>
            <span style={styles.cardTitle}>新建项目</span>
            <span style={styles.cardDesc}>创建全新的 OmniLang 项目</span>
          </button>
        </div>

        {/* Recent Projects */}
        {recentProjects.length > 0 && (
          <div style={styles.recentSection}>
            <div style={styles.recentHeader}>最近打开</div>
            <div style={styles.recentList}>
              {recentProjects.map((proj) => (
                <button type="button"
                  key={proj.path}
                  style={styles.recentItem}
                  onClick={(e) => { if ((e.target as HTMLElement).getAttribute('data-action') === 'remove-recent') { handleRemoveRecent(e as unknown as React.MouseEvent, proj.path); } else { handleOpenRecent(proj); } }}
                >
                  <span style={styles.recentIcon}>📁</span>
                  <div style={styles.recentInfo}>
                    <div style={styles.recentName}>{proj.name}</div>
                    <div style={styles.recentPath}>{proj.path}</div>
                  </div>
                  <span
                    style={styles.recentRemove}
                    data-action="remove-recent"
                    title="移除"
                  >
                    ×
                  </span>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* New Project Dialog (Modal) */}
      {showNewDialog && (
        <div style={styles.overlay} onClick={() => setShowNewDialog(false)}>
          <div style={styles.dialog} onClick={(e) => e.stopPropagation()}>
            <div style={styles.dialogTitle}>新建项目</div>

            {/* Location picker */}
            <div style={styles.dialogLabel}>项目位置</div>
            <div style={styles.dialogLocationRow}>
              <span style={styles.dialogLocationPath}>
                {newProjectParent || "未选择..."}
              </span>
              <button type="button" style={styles.dialogBrowseBtn} onClick={handlePickLocation}>
                选择文件夹
              </button>
            </div>

            {/* Name input */}
            <div style={styles.dialogLabel}>项目名称</div>
            <input
              style={styles.dialogInput}
              type="text"
              placeholder="例如：my-omnilang-app"
              aria-label="项目名称"
              value={newProjectName}
              onChange={(e) => {
                setNewProjectName(e.target.value);
                setError("");
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreateProject();
              }}
            />
            {error && <div style={styles.dialogError}>{error}</div>}
            <div style={styles.dialogButtons}>
              <button type="button" style={styles.dialogCancel} onClick={() => setShowNewDialog(false)}>
                取消
              </button>
              <button type="button"
                style={{
                  ...styles.dialogConfirm,
                  opacity: newProjectParent && newProjectName.trim() ? 1 : 0.5,
                  cursor: newProjectParent && newProjectName.trim() ? "pointer" : "not-allowed",
                }}
                onClick={handleCreateProject}
                disabled={!newProjectParent || !newProjectName.trim()}
              >
                创建
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Styles ──

const accent = "#6d28d9";

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    height: "100vh",
    background: "linear-gradient(135deg, #fafafa 0%, #f0f0f5 100%)",
    fontFamily: "system-ui, -apple-system, sans-serif",
  },
  content: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 32,
    maxWidth: 480,
    width: "100%",
    padding: "0 24px",
  },
  hero: {
    textAlign: "center" as const,
  },
  logo: {
    fontSize: 48,
    color: accent,
    marginBottom: 8,
  },
  title: {
    fontSize: 32,
    fontWeight: 700,
    color: "#1f2937",
    margin: 0,
    letterSpacing: -1,
  },
  subtitle: {
    fontSize: 14,
    color: "#9ca3af",
    margin: "8px 0 0",
  },
  actions: {
    display: "flex",
    gap: 12,
    width: "100%",
  },
  card: {
    flex: 1,
    display: "flex",
    flexDirection: "column" as const,
    alignItems: "center",
    gap: 6,
    padding: "24px 16px",
    background: "#fff",
    border: "1px solid #e5e7eb",
    borderRadius: 12,
    cursor: "pointer",
    transition: "border-color 150ms ease, background-color 150ms ease, opacity 150ms ease, 0.15s",
    boxShadow: "0 1px 2px rgba(0,0,0,0.04)",
    fontFamily: "inherit",
  },
  cardIcon: {
    fontSize: 28,
  },
  cardTitle: {
    fontSize: 15,
    fontWeight: 600,
    color: "#1f2937",
  },
  cardDesc: {
    fontSize: 12,
    color: "#9ca3af",
  },
  recentSection: {
    width: "100%",
  },
  recentHeader: {
    fontSize: 12,
    fontWeight: 600,
    color: "#9ca3af",
    textTransform: "uppercase" as const,
    letterSpacing: 1,
    marginBottom: 8,
  },
  recentList: {
    display: "flex",
    flexDirection: "column" as const,
    gap: 4,
  },
  recentItem: {
    display: "flex",
    alignItems: "center",
    gap: 10,
    padding: "10px 12px",
    background: "#fff",
    border: "1px solid #f3f4f6",
    borderRadius: 8,
    cursor: "pointer",
    fontFamily: "inherit",
    textAlign: "left" as const,
    width: "100%",
    transition: "background 0.1s",
  },
  recentIcon: {
    fontSize: 18,
    flexShrink: 0,
  },
  recentInfo: {
    flex: 1,
    minWidth: 0,
  },
  recentName: {
    fontSize: 13,
    fontWeight: 600,
    color: "#1f2937",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap" as const,
  },
  recentPath: {
    fontSize: 12,
    color: "#9ca3af",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap" as const,
    marginTop: 1,
  },
  recentRemove: {
    fontSize: 16,
    color: "#d1d5db",
    padding: "2px 6px",
    borderRadius: 4,
    flexShrink: 0,
    transition: "color 0.1s",
  },
  // Modal
  overlay: {
    position: "fixed",
    inset: 0,
    background: "rgba(0,0,0,0.3)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    zIndex: 200,
  },
  dialog: {
    background: "#fff",
    borderRadius: 16,
    padding: 28,
    width: 400,
    maxWidth: "90vw",
    boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
  },
  dialogTitle: {
    fontSize: 18,
    fontWeight: 700,
    color: "#1f2937",
    marginBottom: 16,
  },
  dialogLabel: {
    fontSize: 12,
    fontWeight: 600,
    color: "#6b7280",
    marginBottom: 6,
  },
  dialogLocationRow: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    marginBottom: 16,
  },
  dialogLocationPath: {
    flex: 1,
    padding: "8px 12px",
    fontSize: 12,
    color: "#9ca3af",
    background: "#f9fafb",
    border: "1px solid #e5e7eb",
    borderRadius: 6,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap" as const,
    minWidth: 0,
  },
  dialogBrowseBtn: {
    padding: "8px 16px",
    fontSize: 12,
    fontWeight: 500,
    border: "1px solid #d1d5db",
    borderRadius: 6,
    background: "#fff",
    cursor: "pointer",
    fontFamily: "inherit",
    color: "#374151",
    whiteSpace: "nowrap" as const,
    flexShrink: 0,
  },
  dialogInput: {
    width: "100%",
    padding: "10px 14px",
    fontSize: 14,
    border: "1px solid #e5e7eb",
    borderRadius: 8,
    outline: "2px solid transparent",
    fontFamily: "inherit",
    boxSizing: "border-box" as const,
  },
  dialogError: {
    fontSize: 12,
    color: "#ef4444",
    marginTop: 8,
  },
  dialogButtons: {
    display: "flex",
    justifyContent: "flex-end",
    gap: 8,
    marginTop: 20,
  },
  dialogCancel: {
    padding: "8px 20px",
    fontSize: 13,
    border: "1px solid #e5e7eb",
    borderRadius: 8,
    background: "#fff",
    cursor: "pointer",
    fontFamily: "inherit",
    color: "#6b7280",
  },
  dialogConfirm: {
    padding: "8px 20px",
    fontSize: 13,
    fontWeight: 600,
    border: "none",
    borderRadius: 8,
    background: accent,
    color: "#fff",
    cursor: "pointer",
    fontFamily: "inherit",
  },
};
