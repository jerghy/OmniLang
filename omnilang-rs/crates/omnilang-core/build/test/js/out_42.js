function _maplen(map) { return map.size; }
function _mapget(map, key) {
    if (!map.has(key)) throw new Error("Key not found");
    return map.get(key);
}

function main() {
    const m1 = new Map([["alpha", 1]]);
    const key = "beta";
    m1.set(key, 2);
    m1.set("gamma", 3);
    console.log(String(_mapget(m1, "alpha")));
    console.log(String(_mapget(m1, key)));
    console.log(String(_maplen(m1)));
    const m2 = new Map([[1, "one"], [2, "two"]]);
    console.log(_mapget(m2, 1));
    console.log(String(m2.has(3)));
}

main();
