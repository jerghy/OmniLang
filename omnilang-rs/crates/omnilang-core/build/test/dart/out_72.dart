import 'dart:io';

void main() {
    final int x = 10;
    final int result = ((x > 5) ? 1 : 2);
    stdout.writeln(result.toString());
    final bool y = true;
    final String msg = (y ? 'yes' : 'no');
    stdout.writeln(msg);
}

