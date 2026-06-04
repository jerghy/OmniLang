import 'dart:io';

void main() {
    final List<int> arr = [10, 20, 30];
    for (final item in arr) {
        stdout.writeln(item.toString());
    }
    final List<int> empty = [];
    stdout.writeln(empty.length.toString());
    int sum = 0;
    for (final n in arr) {
        sum = (sum + n);
    }
    stdout.writeln(sum.toString());
    final List<int> arr2 = [1, 2, 3, 4, 5];
    for (final x in arr2) {
        if ((x > 3)) {
            break;
        }
        stdout.writeln(x.toString());
    }
}

