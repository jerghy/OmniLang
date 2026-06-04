def _idiv(a, b):
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q

def _irem(a, b):
    return a - _idiv(a, b) * b

def main():
    a = 10
    b = 3
    print(str((a + b)))
    print(str((a - b)))
    print(str((a * b)))
    print(str(_idiv(a, b)))
    print(str(_irem(a, b)))


if __name__ == "__main__":
    main()
