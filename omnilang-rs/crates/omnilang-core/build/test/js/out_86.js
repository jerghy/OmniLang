function main() {
    const x = 10;
    if ((x > 5)) {
        console.log("big");
    } else if ((x == 0)) {
        console.log("zero");
    } else {
        console.log("small");
    }
    let i = 0;
    while ((i < 3)) {
        process.stdout.write(String(i));
        i = (i + 1);
    }
    console.log("done");
}

main();
