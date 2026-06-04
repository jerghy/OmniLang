def main():
    s1 = "hello"
    s2 = s1
    print(s1)
    print(s2)
    a1 = [1, 2]
    a2 = list(a1)
    a2.append(3)
    print(str(len(a1)))
    print(str(len(a2)))


if __name__ == "__main__":
    main()
