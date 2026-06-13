function main() {
    const x = 1;
    console.log(String(x));
    if (true) {
        const x = 2;
        console.log(String(x));
    }
    console.log(String(x));
}

main();
