def main():
    m1 = {"alpha": 1}
    key = "beta"
    m1[key] = 2
    m1["gamma"] = 3
    print(str(m1["alpha"]))
    print(str(m1[key]))
    print(str(len(m1)))
    m2 = {1: "one", 2: "two"}
    print(m2[1])
    print(str(3 in m2).lower())


if __name__ == "__main__":
    main()
