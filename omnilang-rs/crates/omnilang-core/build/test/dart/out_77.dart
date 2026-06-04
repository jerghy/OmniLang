import 'dart:io';

String greet(String name) {
    return ('Hi, ' + name);
}

void process_arr(List<int> arr) {
    stdout.writeln(arr.length.toString());
}

void main() {
    final String name = 'Bob';
    final String msg = greet(('Hello, ' + name));
    stdout.writeln(msg);
}

