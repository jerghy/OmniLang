def main():
    arr = [10, 20, 30]
    for item in arr:
        print(str(item))
    empty = []
    print(str(len(empty)))
    sum = 0
    for n in arr:
        sum = (sum + n)
    print(str(sum))
    arr2 = [1, 2, 3, 4, 5]
    for x in arr2:
        if (x > 3):
            break
        print(str(x))


if __name__ == "__main__":
    main()
