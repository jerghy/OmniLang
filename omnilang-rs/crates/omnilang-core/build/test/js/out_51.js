function greet(name) {
    return ("Hello, " + name);
}

function add(a, b) {
    return (a + b);
}

function main() {
    const s = "Alice";
    const msg = greet(s);
    console.log(msg);
    const result = add(2, 4);
    console.log(String(result));
}

main();
