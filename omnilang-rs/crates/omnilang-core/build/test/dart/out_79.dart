import 'dart:io';

void main() {
    final int x = 100;
    final int y = 200;
    final int sum = (x + y);
    final int diff = (y - x);
    final int prod = (x * 10);
    stdout.writeln(sum.toString());
    stdout.writeln(diff.toString());
    stdout.writeln(prod.toString());
    final String s1 = 'hello';
    final String s2 = 'world';
    final String msg = (s1 + s2);
    stdout.writeln(msg);
}

