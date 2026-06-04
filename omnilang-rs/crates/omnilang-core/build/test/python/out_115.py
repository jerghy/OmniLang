def add(a, b) -> int:
    return (a + b)

def greet(name) -> str:
    return ("Hello, " + name)

def do_nothing():
    return

def main():
    sum = add(3, 4)
    print(str(sum))
    msg = greet("World")
    print(msg)
    do_nothing()
    print("ok")


if __name__ == "__main__":
    main()
