import 'dart:io';

void main() {
    final int a = 42;
    final double b = 3.14;
    final bool c = true;
    final String d = 'test';
    stdout.writeln(a.toString());
    stdout.writeln(b.toString());
    stdout.writeln(c.toString());
    stdout.writeln(d.toString());
    final List<bool> arr = [true, false];
    stdout.writeln(arr.length.toString());
    final Map<String, String> m = {'k': 'v'};
    stdout.writeln(m.length.toString());
}

