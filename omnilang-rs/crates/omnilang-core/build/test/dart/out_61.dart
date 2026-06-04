import 'dart:io';

void main() {
    final int x = 10;
    stdout.writeln(x.toString());
    double y = 3.14;
    stdout.writeln(y.toString());
    y = (y + 1);
    stdout.writeln(y.toString());
    final String s = 'hello';
    stdout.writeln(s);
    final String s2 = s;
    stdout.writeln(s2);
}

