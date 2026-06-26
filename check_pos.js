const fs = require("fs");
const content = fs.readFileSync("D:/space/OmniLang/test/projects/learn-omniview/build/debug/index.html", "utf8");
const match = content.match(/<script>([\s\S]*)<\/script>/);
if (!match) { console.log("No script tag"); process.exit(1); }
const js = match[1];
let depth = 0;
let inStr = false, inTmpl = false, inLine = false, inBlock = false;
const lines = js.split("\n");
for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
  const line = lines[lineIdx];
  for (let i = 0; i < line.length; i++) {
    const c = line[i], p = i > 0 ? line[i-1] : "";
    if (inLine && c === "\n") { inLine = false; continue; }
    if (inBlock && c === "/" && p === "*") { inBlock = false; continue; }
    if (inBlock) continue;
    if (inLine) continue;
    if (c === "/" && p === "/") { inLine = true; continue; }
    if (c === "*" && p === "/") { inBlock = true; continue; }
    if (inTmpl && c === "`" && p !== "\\") { inTmpl = false; continue; }
    if (inTmpl) continue;
    if (inStr && c === '"' && p !== "\\") { inStr = false; continue; }
    if (inStr) continue;
    if (c === '"') { inStr = true; continue; }
    if (c === "`") { inTmpl = true; continue; }
    if (c === "{") depth++;
    if (c === "}") depth--;
    if (depth < 0) {
      console.log("Extra } at line", lineIdx + 1);
    }
  }
  if (depth !== 0) {
    console.log("Line", lineIdx + 1, "depth:", depth, ": " + line.substring(0, 100));
  }
}
console.log("Final depth:", depth);
