// ui/panels/StylePropertyEditor.tsx
// Refactored: replaces raw <input>/<select> with StyleValueEditor (fude).

import React, { useState, useEffect, useRef } from "react";
import type { StyleProperty } from "../../core/types/style";
import { StyleValueEditor } from "./StyleValueEditor";
import {
  isColorProperty,
  isCustomProperty,
  getUnitGroup,
  getDropdownOptions,
} from "../../constants/style-properties";

interface SignalOption {
  name: string;
  label: string;
}

interface Props {
  property: StyleProperty;
  onChange: (prop: StyleProperty) => void;
  onDelete: () => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];
}

// ── Property name search-select dropdown (unchanged from original) ──

const PROPERTY_CATEGORIES: { label: string; props: string[] }[] = [
  { label: "Layout", props: ["width","height","min-width","max-width","min-height","max-height","margin","margin-top","margin-right","margin-bottom","margin-left","padding","padding-top","padding-right","padding-bottom","padding-left","display","gap","overflow","overflow-x","overflow-y"] },
  { label: "Typography", props: ["font-size","font-weight","font-family","color","text-align","line-height","letter-spacing","text-decoration","text-transform","white-space","word-break"] },
  { label: "Background & Border", props: ["background-color","background-image","border","border-color","border-radius","box-shadow","opacity"] },
  { label: "Position", props: ["position","top","right","bottom","left","z-index"] },
  { label: "Flex & Grid", props: ["flex-direction","flex-wrap","flex-grow","flex-shrink","flex-basis","justify-content","align-items","align-self","grid-template-columns","grid-template-rows","grid-column","grid-row"] },
  { label: "Transform & Transition", props: ["transform","transition","transition-duration","transition-property","transition-timing-function","animation","animation-name","animation-duration","animation-iteration-count","animation-direction","animation-fill-mode","animation-timing-function","cursor"] },
];

const PropertyNameSelect: React.FC<{
  value: string;
  onChange: (name: string) => void;
  includeCustom?: boolean;
}> = ({ value, onChange, includeCustom }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [searchText, setSearchText] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;
    const close = () => { setIsOpen(false); setSearchText(""); };
    const mouseHandler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) close();
    };
    const keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("mousedown", mouseHandler);
    document.addEventListener("keydown", keyHandler);
    return () => {
      document.removeEventListener("mousedown", mouseHandler);
      document.removeEventListener("keydown", keyHandler);
    };
  }, [isOpen]);

  const filtered = PROPERTY_CATEGORIES
    .map(cat => ({ ...cat, props: cat.props.filter(p => p.toLowerCase().includes(searchText.toLowerCase())) }))
    .filter(cat => cat.props.length > 0);

  const showCustom = includeCustom && value.startsWith("--") && value.toLowerCase().includes(searchText.toLowerCase());

  return (
    <div ref={ref} style={{ position: "relative" }}>
      <div onClick={() => setIsOpen(!isOpen)} style={{ background:"#fff",color:"#374151",border:"1px solid #d1d5db",borderRadius:4,padding:"4px 8px",fontSize:12,cursor:"pointer",width:110,flex:"none" }}>
        {value || "select..."}
      </div>
      {isOpen && (
        <div style={{ position:"absolute",zIndex:10,background:"#fff",border:"1px solid #d1d5db",borderRadius:4,marginTop:2,maxHeight:250,overflow:"auto",width:200,boxShadow:"0 4px 12px rgba(0,0,0,0.1)" }}>
          <input autoFocus placeholder="搜索属性..." value={searchText} onChange={e => setSearchText(e.target.value)}
            style={{ width:"100%",boxSizing:"border-box",padding:"6px 8px",border:"none",borderBottom:"1px solid #e5e7eb",outline:"none",fontSize:12 }} />
          {filtered.map(cat => (
            <div key={cat.label}>
              <div style={{ padding:"4px 8px",fontSize:10,color:"#6B7280",fontWeight:600 }}>{cat.label}</div>
              {cat.props.map(p => (
                <div key={p} onClick={() => { onChange(p); setIsOpen(false); setSearchText(""); }}
                  style={{ padding:"4px 8px",cursor:"pointer",fontSize:12,background:p===value?"#EEF2FF":"transparent",color:p===value?"#4F46E5":"#374151" }}>
                  {p}
                </div>
              ))}
            </div>
          ))}
          {showCustom && (
            <div>
              <div style={{ padding:"4px 8px",fontSize:10,color:"#6B7280",fontWeight:600 }}>Custom</div>
              <div onClick={() => { onChange(value); setIsOpen(false); setSearchText(""); }}
                style={{ padding:"4px 8px",cursor:"pointer",fontSize:12,background:"#EEF2FF",color:"#4F46E5" }}>
                {value}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// ── Delete button ──
const DeleteBtn: React.FC<{ onClick: () => void }> = ({ onClick }) => (
  <button onClick={onClick} style={{ background:"none",border:"none",color:"#DC2626",cursor:"pointer",fontSize:14,padding:"0 4px" }} title="删除">×</button>
);

// ── Main component ──

const ROW = { display: "flex", gap: 6, alignItems: "center", marginBottom: 4 } as React.CSSProperties;

export function StylePropertyEditor({ property, onChange, onDelete, availableSignals, availableCssVars }: Props) {
  const isCustom = isCustomProperty(property.name);
  const color = isColorProperty(property.name);
  const dropdownOpts = getDropdownOptions(property.name);
  const unitGroup = getUnitGroup(property.name);

  // Extract text value from segments for color/unit/dropdown special rendering
  const textValue = property.segments.length === 1 && property.segments[0].type === "text"
    ? property.segments[0].value
    : "";

  // Color properties: color picker + fude editor
  if (color) {
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <input type="color" value={textValue.startsWith("#") ? textValue : "#000000"}
          onChange={(e) => onChange({ ...property, segments: [{ type: "text", value: e.target.value }] })}
          style={{ width: 28, height: 24, border: "none", cursor: "pointer", flex: "none" }} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Dropdown properties
  if (dropdownOpts !== null) {
    if (Array.isArray(dropdownOpts)) {
      return (
        <div style={ROW}>
          <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
          <select value={textValue} onChange={(e) => onChange({ ...property, segments: [{ type: "text", value: e.target.value }] })}
            style={{ background:"#fff",color:"#374151",border:"1px solid #d1d5db",borderRadius:4,padding:"4px 8px",fontSize:12,flex:1 }}>
            {dropdownOpts.map((opt) => (<option key={opt} value={opt}>{opt}</option>))}
          </select>
          <DeleteBtn onClick={onDelete} />
        </div>
      );
    }
    // Free text for transform, font-family, etc.
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Opacity: slider + number + fude
  if (property.name === "opacity") {
    const numVal = parseFloat(textValue);
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <input type="range" min="0" max="1" step="0.01" value={isNaN(numVal) ? 1 : numVal}
          onChange={(e) => onChange({ ...property, segments: [{ type: "text", value: e.target.value }] })}
          style={{ flex: 1 }} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Numeric + unit
  if (unitGroup) {
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Generic: fude editor
  return (
    <div style={ROW}>
      <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
      <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
      <DeleteBtn onClick={onDelete} />
    </div>
  );
}
