# Generated from input.omni

from dataclasses import dataclass

@dataclass
class Counter:
    value: int

def add(a: int, b: int) -> int:
    return a + b

def main():
    value = 0
    result = add(value, 5)
    print(result)

if __name__ == "__main__":
    main()

