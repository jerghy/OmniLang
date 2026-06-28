/** 项目欢迎页 — 创建新项目 / 打开已有项目 / 最近项目列表。 */

import { useState, useEffect, useCallback } from "react";
import { api } from "../../commands";

// ── Types ────────────────────────────────────────────

interface DirEntry {
  name: string;
  isDir: boolean;
}

interface RecentProject {
  name: string;
  path: string;
  lastOpened: string;
}

const RECENT_KEY = "omni_recent_projects";
const MAX_RECENT = 10;

function loadRecent(): RecentProject[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) || "[]");
  } catch {
    return [];
  }
}

function saveRecent(projects: RecentProject[]): void {
  localStorage.setItem(RECENT_KEY, JSON.stringify(projects.slice(0, MAX_RECENT)));
}

function addRecent(name: string, path: string): void {
  const list = loadRecent().filter((p) => p.path !== path);
  list.unshift({ name, path, lastOpened: new Date().toISOString() });
  saveRecent(list);
}

// ── Directory Browser ────────────────────────────────

function DirBrowser({ onSelectDir }: { onSelectDir: (absPath: string) => void }) {
  const [currentPath, setCurrentPath] = useState("");
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [pathInput, setPathInput] = useState("");

  const loadDir = useCallback(async (dirPath: string) => {
    setLoading(true);
    setError("");
    try {
      const res = await api.project.listDirectory(dirPath);
      if (res.ok && res.entries) {
        const abs = res.current || dirPath;
        setCurrentPath(abs);
        setPathInput(abs);
        setEntries(res.entries.filter((e) => e.isDir));
      } else {
        setError(res.error || "加载失败");
      }
    } catch {
      setError("无法连接到服务器");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- load directory listing on mount
    loadDir("/");
  }, [loadDir]);

  // Sync the current directory back to parent
  useEffect(() => {
    if (currentPath) onSelectDir(currentPath);
  }, [currentPath, onSelectDir]);

  const navigateTo = (name: string) => {
    loadDir(`${currentPath}/${name}`);
  };

  const goUp = () => {
    const normalized = currentPath.replace(/\\/g, "/").replace(/\/$/, "");
    const idx = normalized.lastIndexOf("/");
    if (idx > 0) loadDir(normalized.substring(0, idx));
  };

  return (
    <div style={{ fontSize: 12 }}>
      {/* Path input */}
      <div style={{ display: "flex", gap: 4, marginBottom: 6 }}>
        <input
          type="text"
          value={pathInput}
          onChange={(e) => setPathInput(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") loadDir(pathInput); }}
          placeholder="输入路径..."
          style={{ flex: 1, padding: "4px 6px", fontSize: 11, border: "1px solid #d1d5db", borderRadius: 4, outline: "none" }}
        />
        <button onClick={() => loadDir(pathInput)}
          style={{ padding: "3px 8px", fontSize: 11, border: "1px solid #d1d5db", borderRadius: 4, background: "#fff", cursor: "pointer" }}>
          跳转
        </button>
      </div>

      <div style={{ fontSize: 10, color: "#6B7280", marginBottom: 4, padding: "2px 4px", background: "#F3F4F6", borderRadius: 3, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        📁 {currentPath}
      </div>

      {loading ? (
        <div style={{ textAlign: "center", color: "#9CA3AF", padding: 8 }}>⏳ 加载中...</div>
      ) : error ? (
        <div style={{ color: "#DC2626", padding: 8 }}>❌ {error}</div>
      ) : (
        <div style={{ maxHeight: 180, overflow: "auto", border: "1px solid #e5e7eb", borderRadius: 4 }}>
          {currentPath.replace(/\\/g, "/") !== "/" && (
            <div onClick={goUp}
              style={{ padding: "4px 8px", cursor: "pointer", display: "flex", alignItems: "center", gap: 4, color: "#6B7280" }}
              onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }}
              onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}>
              📂 ..
            </div>
          )}
          {entries.map((e) => (
            <div key={e.name} onClick={() => navigateTo(e.name)}
              style={{ padding: "4px 8px", cursor: "pointer", display: "flex", alignItems: "center", gap: 4, color: "#374151" }}
              onMouseEnter={(e2) => { e2.currentTarget.style.background = "#F3F4F6"; }}
              onMouseLeave={(e2) => { e2.currentTarget.style.background = "transparent"; }}>
              📂 {e.name}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Project Welcome ──────────────────────────────────

interface ProjectWelcomeProps {
  onProjectReady: (dir: string, name: string) => void;
}

export function ProjectWelcome({ onProjectReady }: ProjectWelcomeProps) {
  const [recent, setRecent] = useState<RecentProject[]>(loadRecent);
  const [tab, setTab] = useState<"recent" | "create" | "open">("recent");

  // ── Create project ──
  const [createDir, setCreateDir] = useState("");
  const [createName, setCreateName] = useState("");
  const [creating, setCreating] = useState(false);
  const [createErr, setCreateErr] = useState("");

  const handleCreate = async () => {
    if (!createDir || !createName) { setCreateErr("请选择目录并填写项目名称"); return; }
    setCreating(true);
    setCreateErr("");
    try {
      const res = await api.project.createProject(createDir, createName);
      if (res.ok && res.dir) {
        addRecent(createName, res.dir);
        onProjectReady(res.dir, createName);
      } else {
        setCreateErr(res.error || "创建失败");
      }
    } catch {
      setCreateErr("无法连接到服务器");
    } finally {
      setCreating(false);
    }
  };

  // ── Open project ──
  const [openDir, setOpenDir] = useState("");
  const [opening, setOpening] = useState(false);
  const [openErr, setOpenErr] = useState("");

  const handleOpen = async (dir?: string) => {
    const target = dir || openDir;
    if (!target) { setOpenErr("请选择项目目录"); return; }
    setOpening(true);
    setOpenErr("");
    try {
      const res = await api.project.setProjectDir(target);
      if (res.ok && res.dir) {
        const name = res.dir.replace(/^.*[/\\]/, "") || "未命名项目";
        addRecent(name, res.dir);
        onProjectReady(res.dir, name);
      } else {
        setOpenErr(res.error || "打开失败");
      }
    } catch {
      setOpenErr("无法连接到服务器");
    } finally {
      setOpening(false);
    }
  };

  // ── Recent projects ──
  const handleRecentClick = async (p: RecentProject) => {
    setOpening(true);
    try {
      const res = await api.project.setProjectDir(p.path);
      if (res.ok && res.dir) {
        addRecent(p.name, res.dir);
        onProjectReady(res.dir, p.name);
      }
    } catch {
      // ignore
    } finally {
      setOpening(false);
    }
  };

  const clearRecent = () => {
    localStorage.removeItem(RECENT_KEY);
    setRecent([]);
  };

  const formatDate = (iso: string) => {
    try {
      return new Date(iso).toLocaleDateString("zh-CN", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
    } catch { return ""; }
  };

  return (
    <div style={{
      display: "flex", alignItems: "center", justifyContent: "center",
      height: "100vh", background: "linear-gradient(135deg, #f5f3ff 0%, #ede9fe 50%, #e0e7ff 100%)",
      fontFamily: "system-ui, sans-serif",
    }}>
      <div style={{
        background: "#fff", borderRadius: 16, padding: "32px 36px",
        boxShadow: "0 4px 24px rgba(0,0,0,0.1)", maxWidth: 500, width: "100%",
      }}>
        <div style={{ textAlign: "center", marginBottom: 24 }}>
          <div style={{ fontSize: 32, marginBottom: 4 }}>🖋️</div>
          <h1 style={{ fontSize: 22, fontWeight: 700, color: "#1F2937", margin: 0 }}>OmniLang Editor</h1>
          <p style={{ fontSize: 13, color: "#9CA3AF", margin: "4px 0 0" }}>选择项目开始编辑</p>
        </div>

        {/* Tabs */}
        <div style={{ display: "flex", gap: 0, marginBottom: 16, borderBottom: "2px solid #E5E7EB" }}>
          {([
            { key: "recent" as const, label: "📂 最近项目" },
            { key: "create" as const, label: "✨ 创建项目" },
            { key: "open" as const, label: "🔍 打开项目" },
          ]).map((t) => (
            <button key={t.key} onClick={() => setTab(t.key)} style={{
              flex: 1, padding: "8px 12px", fontSize: 12, fontWeight: 600,
              border: "none", borderBottom: tab === t.key ? "2px solid #4F46E5" : "2px solid transparent",
              marginBottom: -2, background: "transparent", cursor: "pointer",
              color: tab === t.key ? "#4F46E5" : "#9CA3AF",
            }}>
              {t.label}
            </button>
          ))}
        </div>

        {/* ── Recent ── */}
        {tab === "recent" && (
          <div>
            {recent.length === 0 ? (
              <div style={{ textAlign: "center", color: "#9CA3AF", fontSize: 13, padding: "24px 0" }}>
                还没有最近的项目<br />
                <span style={{ fontSize: 11 }}>创建或打开一个项目开始使用</span>
              </div>
            ) : (
              <>
                <div style={{ maxHeight: 240, overflow: "auto", marginBottom: 8 }}>
                  {recent.map((p) => (
                    <div key={p.path} onClick={() => handleRecentClick(p)} style={{
                      padding: "10px 12px", cursor: "pointer", borderRadius: 8,
                      display: "flex", alignItems: "center", gap: 10, marginBottom: 2,
                    }}
                      onMouseEnter={(e) => { e.currentTarget.style.background = "#F5F3FF"; }}
                      onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}>
                      <span style={{ fontSize: 20 }}>📁</span>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ fontSize: 13, fontWeight: 600, color: "#1F2937" }}>{p.name}</div>
                        <div style={{ fontSize: 11, color: "#9CA3AF", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.path}</div>
                      </div>
                      <div style={{ fontSize: 10, color: "#D1D5DB", whiteSpace: "nowrap" }}>{formatDate(p.lastOpened)}</div>
                    </div>
                  ))}
                </div>
                <button onClick={clearRecent} style={{ fontSize: 11, color: "#9CA3AF", background: "none", border: "none", cursor: "pointer", padding: 0 }}>清除历史</button>
              </>
            )}
          </div>
        )}

        {/* ── Create ── */}
        {tab === "create" && (
          <div>
            <div style={{ fontSize: 12, color: "#6B7280", marginBottom: 8 }}>选择父目录并输入项目名称</div>
            <DirBrowser onSelectDir={setCreateDir} />
            <input type="text" placeholder="项目名称" value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); }}
              style={{ width: "100%", padding: "6px 10px", fontSize: 13, border: "1px solid #d1d5db", borderRadius: 6, marginTop: 8, boxSizing: "border-box", outline: "none" }} />
            {createErr && <div style={{ color: "#DC2626", fontSize: 11, marginTop: 6 }}>{createErr}</div>}
            <button onClick={handleCreate} disabled={creating} style={{
              width: "100%", marginTop: 10, padding: "8px 16px", fontSize: 13, fontWeight: 600,
              border: "none", borderRadius: 8, background: "#4F46E5", color: "#fff",
              cursor: creating ? "wait" : "pointer", opacity: creating ? 0.7 : 1,
            }}>
              {creating ? "⏳ 创建中..." : "🚀 创建项目"}
            </button>
          </div>
        )}

        {/* ── Open ── */}
        {tab === "open" && (
          <div>
            <div style={{ fontSize: 12, color: "#6B7280", marginBottom: 8 }}>浏览到包含 .omni 文件的文件夹</div>
            <DirBrowser onSelectDir={setOpenDir} />
            {openErr && <div style={{ color: "#DC2626", fontSize: 11, marginTop: 6 }}>{openErr}</div>}
            {opening && <div style={{ textAlign: "center", color: "#6B7280", fontSize: 11, padding: 8 }}>⏳ 加载中...</div>}
            <button onClick={() => handleOpen()} disabled={opening || !openDir} style={{
              width: "100%", marginTop: 10, padding: "8px 16px", fontSize: 13, fontWeight: 600,
              border: "none", borderRadius: 8, background: "#4F46E5", color: "#fff",
              cursor: opening || !openDir ? "wait" : "pointer", opacity: opening || !openDir ? 0.5 : 1,
            }}>
              {opening ? "⏳ 加载中..." : "📂 选择此目录"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
