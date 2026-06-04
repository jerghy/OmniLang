import 'dart:io';

void main() {
    final Map<String, int> m1 = {'alpha': 1};
    final String key = 'beta';
    m1[key] = 2;
    m1['gamma'] = 3;
    stdout.writeln((m1['alpha'] ?? (throw Exception('Key not found'))).toString());
    stdout.writeln((m1[key] ?? (throw Exception('Key not found'))).toString());
    stdout.writeln(m1.length.toString());
    final Map<int, String> m2 = {1: 'one', 2: 'two'};
    stdout.writeln((m2[1] ?? (throw Exception('Key not found'))));
    stdout.writeln(m2.containsKey(3).toString());
}

