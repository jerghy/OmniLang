import 'dart:io';

int add(int a, int b) {
    return (a + b);
}

String greet(String name) {
    return ('Hello, ' + name);
}

void do_nothing() {
    return;
}

void main() {
    final int sum = add(3, 4);
    stdout.writeln(sum.toString());
    final String msg = greet('World');
    stdout.writeln(msg);
    do_nothing();
    stdout.writeln('ok');
}

