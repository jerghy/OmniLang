def main():
    m = {"a": 1, "b": 2}
    print(str(len(m)))
    print(str(len(m) == 0).lower())
    v = m["a"]
    print(str(v))
    print(str("b" in m).lower())
    m2 = {1: "one"}
    m2[2] = "two"
    print(str(len(m2)))
    print(m2[2])
    m2[3] = "three"
    print(m2[3])


if __name__ == "__main__":
    main()
