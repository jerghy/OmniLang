function _arrlen(arr) { return arr.length; }
function _arrClone(arr) { return arr.slice(); }

function main() {
    const s1 = "hello";
    const s2 = s1;
    console.log(s1);
    console.log(s2);
    const a1 = [1, 2];
    const a2 = _arrClone(a1);
    a2.push(3);
    console.log(String(_arrlen(a1)));
    console.log(String(_arrlen(a2)));
}

main();
