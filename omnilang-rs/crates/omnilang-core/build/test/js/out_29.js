function _arrlen(arr) { return arr.length; }

function main() {
    const arr = [10, 20, 30];
    for (const item of arr) {
        console.log(String(item));
    }
    const empty = [];
    console.log(String(_arrlen(empty)));
    let sum = 0;
    for (const n of arr) {
        sum = (sum + n);
    }
    console.log(String(sum));
    const arr2 = [1, 2, 3, 4, 5];
    for (const x of arr2) {
        if ((x > 3)) {
            break;
        }
        console.log(String(x));
    }
}

main();
