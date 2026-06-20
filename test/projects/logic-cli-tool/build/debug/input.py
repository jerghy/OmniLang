# Generated from input.omni

def main():
    result = run('build', True)
    print(result)

def run(command: str, verbose: bool) -> str:
    if verbose:
        return 'Running in verbose mode: ' + command
    return command

if __name__ == "__main__":
    main()

