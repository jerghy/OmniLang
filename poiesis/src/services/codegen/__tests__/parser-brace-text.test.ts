import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parser text content with braces", () => {
  it("preserves {expr} in element content text", () => {
    const source = `import omniview;

component Foo {
    view {
        <div>
            <p>Hello {name}</p>
        </div>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect(pEl).toBeDefined();
    expect("props" in pEl! && pEl.props.label).toBe("Hello {name}");
  });

  it("preserves plain text content", () => {
    const source = `import omniview;

component Foo {
    view {
        <div>
            <p>Hello World</p>
        </div>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect("props" in pEl! && pEl.props.label).toBe("Hello World");
  });

  it("preserves {expr} only as content", () => {
    const source = `import omniview;

component Foo {
    view {
        <div>
            <p>{name}</p>
        </div>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect("props" in pEl! && pEl.props.label).toBe("{name}");
  });

  it("preserves multiple {expr} in mixed text", () => {
    const source = `import omniview;

component Foo {
    view {
        <p>{count} items, {done} done</p>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect("props" in pEl! && pEl.props.label).toBe("{count} items, {done} done");
  });

  it("view 正文裸文本 {expr}：结构性 } 停止收集（651bae7a 状态机锁定）", () => {
    // 单行 view { Hello {name} }：{name} 插值括号配平收集，view 收尾的 } 不被吞进文本。
    // 修复前 collectTextWithSpacing 按行贪婪收集会吞掉结构性 BRACE_CLOSE → view 提前收尾，
    // 后续 signal 成员与下一组件整体被解析进 view 体（成员丢失、文本不保持）。
    const source = `import omniview;

component A {
    view { Hello {name} }
    signal s: i32 = 0;
}
component B {
    view { <div>World</div> }
}`;
    const result = parseOmniSource(source);
    expect(result.errors).toEqual([]);
    // view 在首个未配平 } 处正确收尾 → 后续 signal 成员未被吞进 view 体
    expect(result.signals?.map((s) => s.name)).toEqual(["s"]);
    // 后续组件正常解析，其文本保持（修复前 B 的 div 及其 label 整体丢失）
    const div = result.designElements?.find((e) => "tag" in e && e.tag === "div");
    expect(div).toBeDefined();
    expect("props" in div! && div.props.label).toBe("World");
  });

  it("单行 view 内元素文本 {expr} 保持（651bae7a 相邻路径）", () => {
    const source = `import omniview;

component Foo {
    view { <div>Hello {name}</div> }
}`;
    const result = parseOmniSource(source);
    expect(result.errors).toEqual([]);
    const div = result.designElements?.find((e) => "tag" in e && e.tag === "div");
    expect(div).toBeDefined();
    expect("props" in div! && div.props.label).toBe("Hello {name}");
  });
});
