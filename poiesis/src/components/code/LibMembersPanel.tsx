// components/code/LibMembersPanel.tsx
// Code tab 文件级成员面板（常驻左侧 280px，无按钮开关——用户已确认）。
// 三段：Imports（自由文本）/ Structs（pub/nominal + fields 行列表）/ Types（pub + 类型表达式文本）。
// 只改数据（setLibDef 不可变写回），发射由保存侧 emitLibFile 完成。
// S 样式对象照抄 StoreMembersPanel（panel/sectionHeader/addBtn/itemRow/itemHeader/
// input/select/typeSep/valueRow/removeBtn）；pid 生成与解析侧同源（struct_/type_ + uid，
// 见 services/codegen/parser.ts:419/460）。

import { usePoiesisStore } from "../../store";
import type { LibFileDef, StructDef, TypeDef } from "../../services/codegen/types";
import { uid } from "../../core/utils/id";

// ── Styles using --ol-* tokens to match Poiesis visual language ──

const S = {
  panel: {
    height: "100%",
    overflow: "auto",
    background: "var(--ol-gray-50)",
  } as React.CSSProperties,

  sectionHeader: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "8px 12px",
    cursor: "pointer",
    userSelect: "none",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    color: "var(--ol-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  addBtn: {
    padding: "1px 7px",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    background: "var(--ol-surface)",
    color: "var(--ol-text-secondary)",
    cursor: "pointer",
    lineHeight: "18px",
  } as React.CSSProperties,

  itemRow: {
    padding: "6px 12px",
    fontSize: "var(--ol-font-size-12)",
    borderBottom: "1px solid var(--ol-border-subtle)",
    background: "var(--ol-surface)",
  } as React.CSSProperties,

  itemHeader: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1)",
    marginBottom: 3,
  } as React.CSSProperties,

  input: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 6px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "2px solid transparent",
    width: "100%",
  } as React.CSSProperties,

  select: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 4px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "2px solid transparent",
    cursor: "pointer",
  } as React.CSSProperties,

  typeSep: {
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    fontFamily: "var(--ol-font-family-mono)",
  } as React.CSSProperties,

  valueRow: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1)",
  } as React.CSSProperties,

  removeBtn: {
    border: "none",
    background: "transparent",
    color: "var(--ol-text-muted)",
    cursor: "pointer",
    fontSize: "var(--ol-font-size-13)",
    padding: "0 2px",
    lineHeight: 1,
  } as React.CSSProperties,
};

export function LibMembersPanel() {
  const libDefs = usePoiesisStore((s) => s.libDefs);
  const setLibDef = usePoiesisStore((s) => s.setLibDef);
  const activeTab = usePoiesisStore((s) => {
    const id = s.designPage.activeTabId;
    if (!id) return null;
    return s.designPage.openTabs.find((t) => t.id === id) ?? null;
  });
  const filePath = activeTab?.filePath ?? "";
  const def: LibFileDef = libDefs[filePath] ?? { imports: [], structs: [], types: [], fns: [] };

  const update = (fn: (d: LibFileDef) => LibFileDef) => {
    if (!filePath) return;
    setLibDef(filePath, fn(def));
  };

  const addImport = () => update((d) => ({ ...d, imports: [...d.imports, ""] }));
  const setImport = (i: number, v: string) =>
    update((d) => ({ ...d, imports: d.imports.map((x, idx) => (idx === i ? v : x)) }));
  const removeImport = (i: number) =>
    update((d) => ({ ...d, imports: d.imports.filter((_, idx) => idx !== i) }));

  const addStruct = () =>
    update((d) => ({ ...d, structs: [...d.structs, { pid: `struct_${uid()}`, name: "", pub: false, nominal: false, fields: [] }] }));
  const setStruct = (i: number, patch: Partial<StructDef>) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => (idx === i ? { ...s, ...patch } : s)) }));
  const removeStruct = (i: number) =>
    update((d) => ({ ...d, structs: d.structs.filter((_, idx) => idx !== i) }));
  const setStructField = (si: number, fi: number, patch: { name?: string; type?: string }) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => idx === si
      ? { ...s, fields: s.fields.map((f, fIdx) => (fIdx === fi ? { ...f, ...patch } : f)) } : s) }));
  const addStructField = (si: number) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => idx === si
      ? { ...s, fields: [...s.fields, { name: "", type: "" }] } : s) }));
  const removeStructField = (si: number, fi: number) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => idx === si
      ? { ...s, fields: s.fields.filter((_, fIdx) => fIdx !== fi) } : s) }));

  const addType = () =>
    update((d) => ({ ...d, types: [...d.types, { pid: `type_${uid()}`, name: "", pub: false, value: "" }] }));
  const setType = (i: number, patch: Partial<TypeDef>) =>
    update((d) => ({ ...d, types: d.types.map((t, idx) => (idx === i ? { ...t, ...patch } : t)) }));
  const removeType = (i: number) =>
    update((d) => ({ ...d, types: d.types.filter((_, idx) => idx !== i) }));

  return (
    <div style={S.panel}>
      {/* Imports 段 */}
      <div style={S.sectionHeader}>
        <span>Imports</span>
        <button type="button" style={S.addBtn} onClick={addImport}>+ Import</button>
      </div>
      {def.imports.map((imp, i) => (
        <div key={i} style={S.itemRow}>
          <input style={S.input} value={imp} placeholder="import ..."
            onChange={(e) => setImport(i, e.target.value)} />
          <button type="button" style={S.removeBtn} onClick={() => removeImport(i)}>×</button>
        </div>
      ))}
      {/* Structs 段 */}
      <div style={S.sectionHeader}>
        <span>Structs</span>
        <button type="button" style={S.addBtn} onClick={addStruct}>+ Struct</button>
      </div>
      {def.structs.map((s, si) => (
        <div key={s.pid} style={S.itemRow}>
          <input style={S.input} value={s.name} placeholder="struct name"
            onChange={(e) => setStruct(si, { name: e.target.value })} />
          <label style={{ fontSize: 12 }}>
            <input type="checkbox" checked={s.pub}
              onChange={(e) => setStruct(si, { pub: e.target.checked })} /> pub
          </label>
          <label style={{ fontSize: 12 }}>
            <input type="checkbox" checked={s.nominal}
              onChange={(e) => setStruct(si, { nominal: e.target.checked })} /> nominal
          </label>
          <button type="button" style={S.removeBtn} onClick={() => removeStruct(si)}>×</button>
          {s.fields.map((f, fi) => (
            <div key={fi} style={S.valueRow}>
              <input style={{ ...S.input, width: 72 }} value={f.name} placeholder="field"
                onChange={(e) => setStructField(si, fi, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <input style={S.input} value={f.type} placeholder="Type"
                onChange={(e) => setStructField(si, fi, { type: e.target.value })} />
              <button type="button" style={S.removeBtn} onClick={() => removeStructField(si, fi)}>×</button>
            </div>
          ))}
          <button type="button" style={S.addBtn} onClick={() => addStructField(si)}>+ Field</button>
        </div>
      ))}
      {/* Types 段 */}
      <div style={S.sectionHeader}>
        <span>Types</span>
        <button type="button" style={S.addBtn} onClick={addType}>+ Type</button>
      </div>
      {def.types.map((t, ti) => (
        <div key={t.pid} style={S.itemRow}>
          <input style={{ ...S.input, width: 96 }} value={t.name} placeholder="type name"
            onChange={(e) => setType(ti, { name: e.target.value })} />
          <label style={{ fontSize: 12 }}>
            <input type="checkbox" checked={t.pub}
              onChange={(e) => setType(ti, { pub: e.target.checked })} /> pub
          </label>
          <span style={S.typeSep}>=</span>
          <input style={S.input} value={t.value} placeholder="Ok | Err(String)"
            onChange={(e) => setType(ti, { value: e.target.value })} />
          <button type="button" style={S.removeBtn} onClick={() => removeType(ti)}>×</button>
        </div>
      ))}
    </div>
  );
}
