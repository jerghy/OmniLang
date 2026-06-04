import 'dart:io';

void process(String s) {
    stdout.writeln(s);
}

void main() {
    final List<String> arr = ['a', 'b'];
    for (final item in arr) {
        final String owned = item;
        stdout.writeln(owned);
    }
}

