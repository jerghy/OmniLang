/** 组件面板：按分类展示内置组件，支持拖拽到画布。
 *  META is the Component Protocol — the single source of truth for
 *  component metadata. Event ports in FlowCanvas are auto-generated
 *  from this data (no more hardcoded getPorts() event branches). */

import React from "react";

/** Per-event metadata. `id` is a stable UUID for protocol migration. */
export interface EventMeta {
  id: string;
  description: string;
  payload: string | null;
}

interface Meta {
  component: string; category: string; description: string;
  props: Record<string, { type: string; default?: unknown; values?: string[]; editor: string }>;
  events: Record<string, EventMeta>;
  styleProperties: string[]; children: boolean;
}

const META: Record<string, Meta> = {
  Container: { component:"Container",category:"Layout",description:"通用容器",props:{},events:{},styleProperties:["padding","margin","gap","background_color","border_radius","width","height"],children:true },
  Flex: { component:"Flex",category:"Layout",description:"弹性布局",props:{ direction:{type:"enum",values:["row","column"],default:"row",editor:"select"}, gap:{type:"string",default:"8px",editor:"text"}, align:{type:"enum",values:["start","center","end","stretch"],default:"start",editor:"select"}, justify:{type:"enum",values:["start","center","end","between","around"],default:"start",editor:"select"} },events:{},styleProperties:["padding","margin","width","height"],children:true },
  Grid: { component:"Grid",category:"Layout",description:"网格布局",props:{ columns:{type:"string",default:"2",editor:"text"}, gap:{type:"string",default:"16px",editor:"text"} },events:{},styleProperties:["padding","margin","width","height"],children:true },
  Card: { component:"Card",category:"Layout",description:"卡片容器",props:{},events:{},styleProperties:["padding","margin","border_radius","shadow","background_color","width","height"],children:true },
  Text: { component:"Text",category:"Typography",description:"文本",props:{ label:{type:"string",default:"Text",editor:"text"} },events:{},styleProperties:["font_size","font_weight","color","text_align"],children:false },
  Heading: { component:"Heading",category:"Typography",description:"标题",props:{ level:{type:"enum",values:["1","2","3","4"],default:"1",editor:"select"}, label:{type:"string",default:"Heading",editor:"text"} },events:{},styleProperties:["font_size","font_weight","color"],children:true },
  Button: { component:"Button",category:"Actions",description:"按钮",props:{ label:{type:"string",default:"Button",editor:"text"}, variant:{type:"enum",values:["primary","secondary","danger","ghost"],default:"primary",editor:"select"}, disabled:{type:"bool",default:false,editor:"checkbox"} },events:{ onPress:{id:"evt-btn-press-001",description:"点击",payload:"MouseEvent"}, onHover:{id:"evt-btn-hover-001",description:"悬停",payload:null}, onFocus:{id:"evt-btn-focus-001",description:"获得焦点",payload:null}, onLongPress:{id:"evt-btn-longpress-001",description:"长按",payload:null} },styleProperties:["width","padding","background_color","border_radius","font_size"],children:true },
  Input: { component:"Input",category:"Forms",description:"输入框",props:{ placeholder:{type:"string",default:"",editor:"text"}, type:{type:"enum",values:["text","password","email","number"],default:"text",editor:"select"} },events:{ onChange:{id:"evt-input-change-001",description:"值变化",payload:"String"} },styleProperties:["width","padding","border_radius","font_size"],children:false },
  TextArea: { component:"TextArea",category:"Forms",description:"多行输入",props:{ placeholder:{type:"string",default:"",editor:"text"}, rows:{type:"string",default:"3",editor:"text"} },events:{ onChange:{id:"evt-textarea-change-001",description:"值变化",payload:"String"} },styleProperties:["width","padding"],children:false },
  Select: { component:"Select",category:"Forms",description:"下拉选择",props:{},events:{ onChange:{id:"evt-select-change-001",description:"值变化",payload:"String"} },styleProperties:["width","padding"],children:false },
  Checkbox: { component:"Checkbox",category:"Forms",description:"复选框",props:{ label:{type:"string",default:"",editor:"text"}, checked:{type:"bool",default:false,editor:"checkbox"} },events:{ onChange:{id:"evt-checkbox-change-001",description:"值变化",payload:"bool"} },styleProperties:[],children:false },
  List: { component:"List",category:"Data",description:"列表",props:{},events:{},styleProperties:["padding","margin","gap"],children:true },
  ListItem: { component:"ListItem",category:"Data",description:"列表项",props:{},events:{},styleProperties:["padding","margin"],children:true },
  Image: { component:"Image",category:"Media",description:"图片",props:{ src:{type:"string",default:"",editor:"text"}, alt:{type:"string",default:"",editor:"text"} },events:{},styleProperties:["width","height","border_radius"],children:false },
  Spinner: { component:"Spinner",category:"Feedback",description:"加载动画",props:{},events:{},styleProperties:["width","height"],children:false },
  Progress: { component:"Progress",category:"Feedback",description:"进度条",props:{ value:{type:"string",default:"0",editor:"text"}, max:{type:"string",default:"100",editor:"text"} },events:{},styleProperties:["width"],children:false },
  Alert: { component:"Alert",category:"Feedback",description:"提示框",props:{ type:{type:"enum",values:["info","success","warning","error"],default:"info",editor:"select"}, label:{type:"string",default:"",editor:"text"} },events:{},styleProperties:["padding","border_radius"],children:true },
  Link: { component:"Link",category:"Navigation",description:"链接",props:{ href:{type:"string",default:"#",editor:"text"}, label:{type:"string",default:"Link",editor:"text"} },events:{},styleProperties:["color","font_size"],children:false },
  Form: { component:"Form",category:"Forms",description:"表单",props:{},events:{ onSubmit:{id:"evt-form-submit-001",description:"提交",payload:null} },styleProperties:["padding","gap","width"],children:true },
};

const CATEGORY_LABELS: Record<string, string> = {
  Layout: "布局", Typography: "排版", Actions: "操作", Forms: "表单",
  Data: "数据", Media: "媒体", Feedback: "反馈", Navigation: "导航",
};

export function ComponentPalette() {
  const cats = new Map<string, Meta[]>();
  for (const m of Object.values(META)) {
    const list = cats.get(m.category) || [];
    list.push(m);
    cats.set(m.category, list);
  }

  const handleDragStart = (e: React.DragEvent, tag: string) => {
    e.dataTransfer.setData("application/omni-component", tag);
    e.dataTransfer.effectAllowed = "copy";
  };

  return (
    <div style={{ padding: 8, fontSize: 12, userSelect: "none" }}>
      <div style={{ fontWeight: 600, marginBottom: 8, fontSize: 11, color: "#6b7280", textTransform: "uppercase" }}>
        组件面板
      </div>
      {Array.from(cats.entries()).map(([cat, items]) => (
        <div key={cat} style={{ marginBottom: 8 }}>
          <div style={{ fontSize: 10, color: "#9ca3af", marginBottom: 2, textTransform: "uppercase" }}>
            {CATEGORY_LABELS[cat] || cat}
          </div>
          {items.map((m) => (
            <div
              key={m.component}
              draggable
              onDragStart={(e) => handleDragStart(e, m.component)}
              title={m.description}
              style={{
                padding: "3px 8px", borderRadius: 4, cursor: "grab",
                fontSize: 12, marginBottom: 1,
                border: "1px solid transparent",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.borderColor = "#d1d5db")}
              onMouseLeave={(e) => (e.currentTarget.style.borderColor = "transparent")}
            >
              {m.component}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

export { META as COMPONENT_PROTOCOL };
