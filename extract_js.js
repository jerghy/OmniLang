const fs = require("fs");
const content = fs.readFileSync("D:/space/OmniLang/test/projects/learn-omniview/build/debug/index.html", "utf8");
const start = content.indexOf("<script>") + 8;
const end = content.indexOf("</script>", start);
fs.writeFileSync("D:/space/OmniLang/generated.js", content.substring(start, end));
console.log("Extracted", end - start, "bytes");
