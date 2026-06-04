import 'dart:io';

void main() {
    int i = 0;
    while ((i < 5)) {
        if ((i == 2)) {
            i = (i + 1);
            continue;
        }
        if ((i == 4)) {
            break;
        }
        stdout.writeln(i.toString());
        i = (i + 1);
    }
    int x = 0;
    int y = 10;
    while ((x < 3)) {
        int inner = 0;
        while ((inner < 2)) {
            y = (y + 1);
            inner = (inner + 1);
        }
        x = (x + 1);
    }
    stdout.writeln(y.toString());
}

