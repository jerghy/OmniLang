import 'dart:io';

void main() {
    final Map<String, int> m = {'a': 1, 'b': 2};
    stdout.writeln(m.length.toString());
    stdout.writeln(m.isEmpty.toString());
    final int v = (m['a'] ?? (throw Exception('Key not found')));
    stdout.writeln(v.toString());
    stdout.writeln(m.containsKey('b').toString());
    Map<int, String> m2 = {1: 'one'};
    m2[2] = 'two';
    stdout.writeln(m2.length.toString());
    stdout.writeln((m2[2] ?? (throw Exception('Key not found'))));
    m2[3] = 'three';
    stdout.writeln(m2[3]);
}

