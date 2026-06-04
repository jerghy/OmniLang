def main():
    i = 0
    while (i < 5):
        if (i == 2):
            i = (i + 1)
            continue
        if (i == 4):
            break
        print(str(i))
        i = (i + 1)
    x = 0
    y = 10
    while (x < 3):
        inner = 0
        while (inner < 2):
            y = (y + 1)
            inner = (inner + 1)
        x = (x + 1)
    print(str(y))


if __name__ == "__main__":
    main()
