import 'dart:io';

void main() {
    final String s1 = 'Hello, ';
    final String s2 = 'World!';
    final String msg = (s1 + s2);
    stdout.writeln(msg);
    String s3 = 'hello';
    s3 = (s3 + s3);
    stdout.writeln(s3);
}

