import 'dart:io';

void main() {
    final int a = 10;
    final int b = 3;
    stdout.writeln((a + b).toString());
    stdout.writeln((a - b).toString());
    stdout.writeln((a * b).toString());
    stdout.writeln((a ~/ b).toString());
    stdout.writeln((a % b).toString());
}

