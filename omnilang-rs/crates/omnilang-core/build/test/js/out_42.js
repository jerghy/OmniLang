function _arrlen(arr) { return arr.length; }
function _arrget(arr, i) {
    if (i < 0 || i >= arr.length) throw new Error("index out of bounds");
    return arr[i];
}

function fib(n) {
    if ((n <= 1)) {
        return n;
    }
    return (fib((n - 1)) + fib((n - 2)));
}

function main() {
    const result = fib(10);
    console.log(String(result));
    let arr = [3, 1, 2];
    let i = 0;
    const n = _arrlen(arr);
    while ((i < n)) {
        let j = 0;
        while ((j < ((n - i) - 1))) {
            if ((_arrget(arr, j) > _arrget(arr, (j + 1)))) {
                const tmp = _arrget(arr, j);
                arr[j] = _arrget(arr, (j + 1));
                arr[(j + 1)] = tmp;
            }
            j = (j + 1);
        }
        i = (i + 1);
    }
    for (const v of arr) {
        console.log(String(v));
    }
}

main();
