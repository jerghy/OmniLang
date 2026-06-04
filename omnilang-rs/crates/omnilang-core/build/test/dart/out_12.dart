import 'dart:io';

void main() {
    final List<int> arr = [1, 2, 3];
    stdout.writeln(arr.length.toString());
    stdout.writeln(arr.isEmpty.toString());
    List<int> arr2 = List.of(arr);
    arr2.add(4);
    arr2.add(5);
    stdout.writeln((arr2[0] ?? (throw Exception('Key not found'))).toString());
    stdout.writeln((arr2[4] ?? (throw Exception('Key not found'))).toString());
    stdout.writeln(arr2.length.toString());
    stdout.writeln(arr.length.toString());
    stdout.writeln(arr2.removeLast().toString());
}

