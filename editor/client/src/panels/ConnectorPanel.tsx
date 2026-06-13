/**
 * 数据连接器面板 — 配置 REST / GraphQL / Firebase 连接器并生成脚手架代码。
 *
 * 工作流:
 *   1. 选择连接器类型 (REST / GraphQL / Firebase)
 *   2. 配置 endpoint / query / collection
 *   3. 粘贴 JSON 响应样本 → 自动推断类型
 *   4. 点击「生成」→ omni source 插入到光标位置
 */

import { useState } from "react";

interface EndpointForm {
  method: string;
  path: string;
  action: string;
  responseJson: string;
}

type ConnectorType = "rest" | "graphql" | "firebase";

interface ConnectorPanelProps {
  onInsertCode: (code: string) => void;
}

export function ConnectorPanel({ onInsertCode }: ConnectorPanelProps) {
  const [connType, setConnType] = useState<ConnectorType>("rest");
  const [name, setName] = useState("Api");
  const [baseUrl, setBaseUrl] = useState("https://api.example.com");
  const [endpoints, setEndpoints] = useState<EndpointForm[]>([
    { method: "GET", path: "/users", action: "fetchUsers", responseJson: "" },
  ]);
  const [query, setQuery] = useState("");
  const [collection, setCollection] = useState("items");
  const [generating, setGenerating] = useState(false);
  const [resultCode, setResultCode] = useState("");

  const handleGenerate = async () => {
    setGenerating(true);
    try {
      const res = await fetch("/api/connector/generate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          type: connType,
          name,
          baseUrl,
          endpoints: connType === "rest" ? endpoints : [],
          query: connType === "graphql" ? query : "",
          collection: connType === "firebase" ? collection : "",
        }),
      });
      const data = await res.json();
      if (data.success) {
        setResultCode(data.code);
      }
    } catch (e) {
      console.error("Connector generate failed:", e);
    } finally {
      setGenerating(false);
    }
  };

  const addEndpoint = () => {
    setEndpoints([...endpoints, { method: "GET", path: "", action: "", responseJson: "" }]);
  };

  const updateEndpoint = (i: number, field: keyof EndpointForm, value: string) => {
    const next = [...endpoints];
    next[i] = { ...next[i], [field]: value };
    setEndpoints(next);
  };

  const removeEndpoint = (i: number) => {
    setEndpoints(endpoints.filter((_, idx) => idx !== i));
  };

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", fontSize: 12 }}>
      {/* Header */}
      <div style={{ padding: "6px 8px", borderBottom: "1px solid #e5e7eb", fontWeight: 600, fontSize: 11, color: "#6B7280", textTransform: "uppercase" }}>
        🔌 数据连接器
      </div>

      {/* Type selector */}
      <div style={{ padding: "8px", borderBottom: "1px solid #f3f4f6" }}>
        <div style={{ display: "flex", gap: 2, marginBottom: 6 }}>
          {(["rest", "graphql", "firebase"] as ConnectorType[]).map((t) => (
            <button
              key={t}
              onClick={() => setConnType(t)}
              style={{
                flex: 1, padding: "3px 6px", fontSize: 11, border: "1px solid", borderRadius: 4,
                borderColor: connType === t ? "#4F46E5" : "#e5e7eb",
                background: connType === t ? "#EEF2FF" : "#fff",
                color: connType === t ? "#4F46E5" : "#6B7280",
                fontWeight: connType === t ? 600 : 400,
                cursor: "pointer",
              }}
            >{t.toUpperCase()}</button>
          ))}
        </div>

        <label style={{ display: "block", marginBottom: 4, fontSize: 10, color: "#9CA3AF" }}>Store 名称</label>
        <input
          value={name} onChange={(e) => setName(e.target.value)}
          style={{ width: "100%", padding: "3px 6px", fontSize: 11, border: "1px solid #e5e7eb", borderRadius: 4, boxSizing: "border-box", marginBottom: 6 }}
        />

        <label style={{ display: "block", marginBottom: 4, fontSize: 10, color: "#9CA3AF" }}>Base URL / Endpoint</label>
        <input
          value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)}
          placeholder="https://api.example.com"
          style={{ width: "100%", padding: "3px 6px", fontSize: 11, border: "1px solid #e5e7eb", borderRadius: 4, boxSizing: "border-box" }}
        />
      </div>

      {/* Connector-specific config */}
      <div style={{ flex: 1, overflow: "auto", padding: "8px" }}>
        {connType === "rest" && (
          <>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
              <span style={{ fontSize: 10, color: "#9CA3AF", fontWeight: 600 }}>接口列表</span>
              <button onClick={addEndpoint} style={{ fontSize: 11, padding: "1px 6px", border: "1px solid #e5e7eb", borderRadius: 3, background: "#fff", cursor: "pointer" }}>+ 添加</button>
            </div>
            {endpoints.map((ep, i) => (
              <div key={i} style={{ marginBottom: 8, padding: 6, border: "1px solid #f3f4f6", borderRadius: 4, background: "#fafafa", position: "relative" }}>
                {endpoints.length > 1 && (
                  <button onClick={() => removeEndpoint(i)}
                    style={{ position: "absolute", top: 2, right: 4, background: "none", border: "none", fontSize: 10, color: "#9CA3AF", cursor: "pointer" }}>✕</button>
                )}
                <div style={{ display: "flex", gap: 4, marginBottom: 3 }}>
                  <select value={ep.method} onChange={(e) => updateEndpoint(i, "method", e.target.value)}
                    style={{ padding: "2px 4px", fontSize: 10, border: "1px solid #e5e7eb", borderRadius: 3, width: 52 }}>
                    {["GET", "POST", "PUT", "DELETE"].map((m) => <option key={m}>{m}</option>)}
                  </select>
                  <input value={ep.action} onChange={(e) => updateEndpoint(i, "action", e.target.value)} placeholder="动作名"
                    style={{ flex: 1, padding: "2px 4px", fontSize: 10, border: "1px solid #e5e7eb", borderRadius: 3 }} />
                </div>
                <input value={ep.path} onChange={(e) => updateEndpoint(i, "path", e.target.value)} placeholder="/users/:id"
                  style={{ width: "100%", padding: "2px 4px", fontSize: 10, border: "1px solid #e5e7eb", borderRadius: 3, marginBottom: 3, boxSizing: "border-box" }} />
                <textarea
                  value={ep.responseJson}
                  onChange={(e) => updateEndpoint(i, "responseJson", e.target.value)}
                  placeholder='{ "id": 1, "name": "..." }'
                  rows={3}
                  style={{ width: "100%", padding: "2px 4px", fontSize: 10, border: "1px solid #e5e7eb", borderRadius: 3, resize: "vertical", boxSizing: "border-box", fontFamily: "monospace" }}
                />
              </div>
            ))}
          </>
        )}

        {connType === "graphql" && (
          <>
            <label style={{ display: "block", marginBottom: 4, fontSize: 10, color: "#9CA3AF" }}>GraphQL Query</label>
            <textarea
              value={query} onChange={(e) => setQuery(e.target.value)}
              placeholder={`query GetUsers { users { id name } }`}
              rows={6}
              style={{ width: "100%", padding: "4px 6px", fontSize: 11, border: "1px solid #e5e7eb", borderRadius: 4, resize: "vertical", boxSizing: "border-box", fontFamily: "monospace", marginBottom: 8 }}
            />
          </>
        )}

        {connType === "firebase" && (
          <>
            <label style={{ display: "block", marginBottom: 4, fontSize: 10, color: "#9CA3AF" }}>Firestore Collection</label>
            <input
              value={collection} onChange={(e) => setCollection(e.target.value)}
              placeholder="users"
              style={{ width: "100%", padding: "3px 6px", fontSize: 11, border: "1px solid #e5e7eb", borderRadius: 4, boxSizing: "border-box" }}
            />
          </>
        )}
      </div>

      {/* Generate button */}
      <div style={{ padding: "8px", borderTop: "1px solid #e5e7eb" }}>
        <button
          onClick={handleGenerate}
          disabled={generating}
          style={{
            width: "100%", padding: "6px 12px", fontSize: 12, fontWeight: 600,
            background: generating ? "#D1D5DB" : "#4F46E5", color: "#fff",
            border: "none", borderRadius: 6, cursor: generating ? "default" : "pointer",
          }}
        >
          {generating ? "生成中..." : "⚡ 生成代码"}
        </button>
      </div>

      {/* Result preview + insert */}
      {resultCode && (
        <div style={{ padding: "8px", borderTop: "1px solid #e5e7eb", background: "#f0fdf4" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
            <span style={{ fontSize: 10, color: "#059669", fontWeight: 600 }}>✅ 生成成功</span>
            <button
              onClick={() => { onInsertCode(resultCode); setResultCode(""); }}
              style={{ fontSize: 11, padding: "2px 8px", background: "#059669", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" }}
            >插入代码</button>
          </div>
          <pre style={{ margin: 0, fontSize: 10, maxHeight: 200, overflow: "auto", whiteSpace: "pre-wrap", color: "#374151" }}>
            {resultCode}
          </pre>
        </div>
      )}
    </div>
  );
}
