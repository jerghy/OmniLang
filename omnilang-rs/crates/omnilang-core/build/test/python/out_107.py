def greet(name) -> str:
    return ("Hello, " + name)

def add(a, b) -> int:
    return (a + b)

def main():
    s = "Alice"
    msg = greet(s)
    print(msg)
    result = add(2, 4)
    print(str(result))


if __name__ == "__main__":
    main()
