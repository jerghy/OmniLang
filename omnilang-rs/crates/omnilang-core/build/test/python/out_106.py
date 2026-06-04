def main():
    x = 10
    if (x > 5):
        print("big")
    elif (x == 0):
        print("zero")
    else:
        print("small")
    i = 0
    while (i < 3):
        print(str(i), end='')
        i = (i + 1)
    print("done")


if __name__ == "__main__":
    main()
