// editor/client/src/omni-gen/source-emitter.ts
// CodeWriter helper for generating indented source code.

export class CodeWriter {
  private lines: string[] = [];
  private _indent = 0;
  private readonly indentStep = "    "; // 4 spaces

  emit(line: string): void {
    this.lines.push(this.indentStep.repeat(this._indent) + line);
  }

  indent(): void { this._indent++; }
  dedent(): void { this._indent = Math.max(0, this._indent - 1); }
  blank(): void { this.lines.push(""); }
  toString(): string { return this.lines.join("\n") + "\n"; }
}
