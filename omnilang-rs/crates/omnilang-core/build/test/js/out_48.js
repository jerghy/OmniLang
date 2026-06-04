function _arrlen(arr) { return arr.length; }
function _arrget(arr, i) {
    if (i < 0 || i >= arr.length) throw new Error("index out of bounds");
    return arr[i];
}
function _arrpop(arr) {
    if (arr.length === 0) throw new Error("pop from empty array");
    return arr.pop();
}
function _arrClone(arr) { return arr.slice(); }

function main() {
    const arr = [1, 2, 3];
    console.log(String(_arrlen(arr)));
    console.log(String((arr.length === 0)));
    let arr2 = _arrClone(arr);
    arr2.push(4);
    arr2.push(5);
    console.log(String(_arrget(arr2, 0)));
    console.log(String(_arrget(arr2, 4)));
    console.log(String(_arrlen(arr2)));
    console.log(String(_arrlen(arr)));
    console.log(String(_arrpop(arr2)));
}

main();
