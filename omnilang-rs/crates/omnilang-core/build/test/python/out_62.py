def fib(n) -> int:
    if (n <= 1):
        return n
    return (fib((n - 1)) + fib((n - 2)))

def main():
    result = fib(10)
    print(str(result))
    arr = [3, 1, 2]
    i = 0
    n = len(arr)
    while (i < n):
        j = 0
        while (j < ((n - i) - 1)):
            if (arr[j] > arr[(j + 1)]):
                tmp = arr[j]
                arr[j] = arr[(j + 1)]
                arr[(j + 1)] = tmp
            j = (j + 1)
        i = (i + 1)
    for v in arr:
        print(str(v))


if __name__ == "__main__":
    main()
