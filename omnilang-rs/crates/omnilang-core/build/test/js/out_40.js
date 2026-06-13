function process(s) {
    console.log(s);
}

function main() {
    const arr = ["a", "b"];
    for (const item of arr) {
        const owned = item;
        console.log(owned);
    }
}

main();
