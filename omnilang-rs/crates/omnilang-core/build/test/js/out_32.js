function main() {
    let i = 0;
    while ((i < 5)) {
        if ((i == 2)) {
            i = (i + 1);
            continue;
        }
        if ((i == 4)) {
            break;
        }
        console.log(String(i));
        i = (i + 1);
    }
    let x = 0;
    let y = 10;
    while ((x < 3)) {
        let inner = 0;
        while ((inner < 2)) {
            y = (y + 1);
            inner = (inner + 1);
        }
        x = (x + 1);
    }
    console.log(String(y));
}

main();
