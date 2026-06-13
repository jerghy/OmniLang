function add(a, b) {
    return (a + b);
}

function greet(name) {
    return ("Hello, " + name);
}

function do_nothing() {
    return;
}

function main() {
    const sum = add(3, 4);
    console.log(String(sum));
    const msg = greet("World");
    console.log(msg);
    do_nothing();
    console.log("ok");
}

main();
