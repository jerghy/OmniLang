function _arrget(arr, i) {
    if (i < 0 || i >= arr.length) throw new Error("index out of bounds");
    return arr[i];
}
function _maplen(map) { return map.size; }
function _mapget(map, key) {
    if (!map.has(key)) throw new Error("Key not found");
    return map.get(key);
}

function main() {
    const m = new Map([["a", 1], ["b", 2]]);
    console.log(String(_maplen(m)));
    console.log(String(_maplen(m) === 0));
    const v = _mapget(m, "a");
    console.log(String(v));
    console.log(String(m.has("b")));
    let m2 = new Map([[1, "one"]]);
    m2.set(2, "two");
    console.log(String(_maplen(m2)));
    console.log(_mapget(m2, 2));
    m2[3] = "three";
    console.log(_arrget(m2, 3));
}

main();
