import 'dart:io';

String greet(String name) {
    return ('Hello, ' + name);
}

int add(int a, int b) {
    return (a + b);
}

void main() {
    final String s = 'Alice';
    final String msg = greet(s);
    stdout.writeln(msg);
    final int result = add(2, 4);
    stdout.writeln(result.toString());
}

