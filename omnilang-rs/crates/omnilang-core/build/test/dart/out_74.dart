import 'dart:io';

int fib(int n) {
    if ((n <= 1)) {
        return n;
    }
    return (fib((n - 1)) + fib((n - 2)));
}

void main() {
    final int result = fib(10);
    stdout.writeln(result.toString());
    List<int> arr = [3, 1, 2];
    int i = 0;
    final int n = arr.length;
    while ((i < n)) {
        int j = 0;
        while ((j < ((n - i) - 1))) {
            if ((arr[j] > arr[(j + 1)])) {
                final int tmp = arr[j];
                arr[j] = arr[(j + 1)];
                arr[(j + 1)] = tmp;
            }
            j = (j + 1);
        }
        i = (i + 1);
    }
    for (final v in arr) {
        stdout.writeln(v.toString());
    }
}

