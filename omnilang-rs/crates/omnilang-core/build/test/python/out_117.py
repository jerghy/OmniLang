def greet(name) -> str:
    return ("Hi, " + name)

def process_arr(arr):
    print(str(len(arr)))

def main():
    name = "Bob"
    msg = greet(("Hello, " + name))
    print(msg)


if __name__ == "__main__":
    main()
