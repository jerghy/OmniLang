function _arrlen(arr) { return arr.length; }
function _maplen(map) { return map.size; }

function main() {
    const a = 42;
    const b = 3.14;
    const c = true;
    const d = "test";
    console.log(String(a));
    console.log(String(b));
    console.log(String(c));
    console.log(String(d));
    const arr = [true, false];
    console.log(String(_arrlen(arr)));
    const m = new Map([["k", "v"]]);
    console.log(String(_maplen(m)));
}

main();
