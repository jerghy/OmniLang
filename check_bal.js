const fs = require("fs");
const content = fs.readFileSync("D:/space/OmniLang/test/projects/learn-omniview/build/debug/index.html", "utf8");
const match = content.match(/<script>([\s\S]*)<\/script>/);
if (!match) { console.log("No script tag"); process.exit(1); }
const js = match[1];
let open = 0, close = 0;
let obrace = 0, cbrace = 0;
let inStr = false, inTmpl = false;
for (let i = 0; i < js.length; i++) {
  const c = js[i], p = i > 0 ? js[i-1] : "";
  if (c === "\"" && p !== "\\" && !inTmpl) inStr = !inStr;
  if (c === "`" && p !== "\\" && !inStr) inTmpl = !inTmpl;
  if (inStr || inTmpl) continue;
  if (c === "/" && p === "/") { while (i < js.length && js[i] !== "\n") i++; continue; }
  if (c === "*" && p === "/") { i += 2; while (i < js.length && !(js[i] === "/" && js[i-1] === "*")) i++; continue; }
  if (c === "(") open++;
  if (c === ")") close++;
  if (c === "{") obrace++;
  if (c === "}") cbrace++;
}
console.log("Open parens:", open, "Close parens:", close, "diff:", open-close);
console.log("Open braces:", obrace, "Close braces:", cbrace, "diff:", obrace-cbrace);
