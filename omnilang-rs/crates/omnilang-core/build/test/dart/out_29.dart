import 'dart:io';

void main() {
    final String s1 = 'hello';
    final String s2 = s1;
    stdout.writeln(s1);
    stdout.writeln(s2);
    final List<int> a1 = [1, 2];
    final List<int> a2 = List.of(a1);
    a2.add(3);
    stdout.writeln(a1.length.toString());
    stdout.writeln(a2.length.toString());
}

