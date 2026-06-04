function _idiv(a, b) {
    if (b === 0) throw new Error("division by zero");
    const r = Math.trunc(a / b);
    return r === 0 ? 0 : r;
}
function _irem(a, b) {
    if (b === 0) throw new Error("division by zero");
    return a % b;
}

function main() {
    const a = 10;
    const b = 3;
    console.log(String((a + b)));
    console.log(String((a - b)));
    console.log(String((a * b)));
    console.log(String(_idiv(a, b)));
    console.log(String(_irem(a, b)));
}

main();
