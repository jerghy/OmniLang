def main():
    arr = [1, 2, 3]
    print(str(len(arr)))
    print(str(len(arr) == 0).lower())
    arr2 = list(arr)
    arr2.append(4)
    arr2.append(5)
    print(str(arr2[0]))
    print(str(arr2[4]))
    print(str(len(arr2)))
    print(str(len(arr)))
    print(str(arr2.pop()))


if __name__ == "__main__":
    main()
