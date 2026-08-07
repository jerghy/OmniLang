// poiesis/src/components/code/__tests__/LibMembersPanel.test.tsx
// Task 10: LibMembersPanel（imports/structs/types 常驻左面板）三段渲染 + 增删写回。
// 相对 brief 的修正（如实验证，详见 task-10-report.md）：
// 1) toBeInTheDocument 非项目既有 matcher（未装 @testing-library/jest-dom）——
//    本地 expect.extend 注册（ComponentDefPanel.test.tsx 的 toHaveAttribute 同款先例）；
// 2) openTabs 条目按真实 DesignTab 类型（id/filePath/componentName/isDirty/pidMap/tabType，
//    无 label 字段）——brief 用例用 as never 兜底，改为类型正确的最小 stub；
// 3) 添加 import 后既有行与新行均含 placeholder "import ..."（getByPlaceholderText
//    多命中会抛错）——用 getAllByPlaceholderText()[1]（DOM 序 = 数组序，新行追加末尾）。
import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { LibMembersPanel } from "../LibMembersPanel";
import { usePoiesisStore } from "../../../store";

declare module "vitest" {
  // 泛型默认值必须与 vitest 官方 Assertion<T = any> 声明同形（否则 tsc 报
  // "All declarations must have identical type parameters"）；any 仅为类型参数
  // 默认值故局部豁免。不写 this: T——实验证（matcher-probe 已删）：
  // 声明 this: T 时调用侧必然 TS2684（Assertion<T> 不可赋值给 T）；NodeCandidateMenu
  // 的 toHaveAttribute 先例即无 this 注解的纯声明，模块增强全局合并后解析到它。
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  interface Assertion<T = any> {
    toBeInTheDocument(): void;
  }
  interface AsymmetricMatchersContaining {
    toBeInTheDocument(): void;
  }
}

expect.extend({
  toBeInTheDocument(received: unknown) {
    const el = received as Element | null;
    const pass = !!el && !!el.ownerDocument && el.ownerDocument.contains(el);
    return {
      pass,
      message: () =>
        `expected element to ${pass ? "not " : ""}be in the document`,
    };
  },
});

const stubDef = {
  imports: [`import{sleep}from"std/async";`],
  structs: [{ pid: "s1", name: "Point", pub: true, nominal: false, fields: [{ name: "x", type: "i32" }] }],
  types: [{ pid: "t1", name: "Score", pub: false, value: "i32" }],
  fns: [],
};

describe("LibMembersPanel", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      libDefs: { "src/lib/a.omni": stubDef },
      designPage: {
        ...usePoiesisStore.getState().designPage,
        activeTabId: "code-a",
        openTabs: [{
          id: "code-a",
          tabType: "code",
          filePath: "src/lib/a.omni",
          componentName: "a",
          isDirty: false,
          pidMap: { status: "ok", entries: {}, lineToPid: {} },
        }],
      },
    });
  });

  it("三段渲染：Imports/Structs/Types", () => {
    render(<LibMembersPanel />);
    expect(screen.getByText("Imports")).toBeInTheDocument();
    expect(screen.getByText("Structs")).toBeInTheDocument();
    expect(screen.getByText("Types")).toBeInTheDocument();
    expect(screen.getByDisplayValue(`import{sleep}from"std/async";`)).toBeInTheDocument();
    expect(screen.getByDisplayValue("Point")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Score")).toBeInTheDocument();
  });

  it("添加 import 写回 libDefs", () => {
    render(<LibMembersPanel />);
    fireEvent.click(screen.getByText("+ Import"));
    const input = screen.getAllByPlaceholderText("import ...")[1];
    fireEvent.change(input, { target: { value: `import{other}from"std/x";` } });
    expect(usePoiesisStore.getState().libDefs["src/lib/a.omni"].imports).toContain(`import{other}from"std/x";`);
  });

  it("struct pub 开关切换写回", () => {
    render(<LibMembersPanel />);
    fireEvent.click(screen.getAllByRole("checkbox")[0]);
    expect(usePoiesisStore.getState().libDefs["src/lib/a.omni"].structs[0].pub).toBe(false);
  });
});
