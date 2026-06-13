function _arrlen(arr) { return arr.length; }

function greet(name) {
    return ("Hi, " + name);
}

function process_arr(arr) {
    console.log(String(_arrlen(arr)));
}

function main() {
    const name = "Bob";
    const msg = greet(("Hello, " + name));
    console.log(msg);
}

main();
