const fs = require("fs");
const content = fs.readFileSync("D:/space/OmniLang/test/projects/learn-omniview/build/debug/index.html", "utf8");
const match = content.match(/<script>([\s\S]*)<\/script>/);
if (!match) { console.log("No script"); process.exit(1); }
const js = match[1];
const lines = js.split("\n");

// Binary search to find the error line
let low = 0, high = lines.length;
while (low < high) {
  const mid = Math.floor((low + high) / 2);
  try {
    require("vm").runInThisContext(lines.slice(0, mid + 1).join("\n"), { filename: "inline.js" });
    low = mid + 1;
  } catch(e) {
    high = mid;
  }
}
console.log("Error at line:", low + 1);
console.log("Line content:", lines[low]);
console.log("Prev lines:");
for (let i = Math.max(0, low - 3); i <= low; i++) {
  console.log((i+1) + ": " + lines[i].substring(0, 200));
}
