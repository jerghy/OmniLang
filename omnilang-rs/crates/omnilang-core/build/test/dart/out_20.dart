import 'dart:io';

void main() {
    final int x = 10;
    if ((x > 5)) {
        stdout.writeln('big');
    } else if ((x == 0)) {
        stdout.writeln('zero');
    } else {
        stdout.writeln('small');
    }
    int i = 0;
    while ((i < 3)) {
        stdout.write(i.toString());
        i = (i + 1);
    }
    stdout.writeln('done');
}

