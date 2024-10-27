def read_file(filename):
    with open(filename, 'r') as file:
        return set(file.read().splitlines())

def compare_files(old_file, new_file):
    old_lines = read_file(old_file)
    new_lines = read_file(new_file)

    unique_to_old = old_lines - new_lines
    unique_to_new = new_lines - old_lines

    print("Lines in old file but not in new file:")
    for line in unique_to_old:
        print(line)

    print("\nLines in new file but not in old file:")
    for line in unique_to_new:
        print(line)

if __name__ == "__main__":
    old_file = "original_output.txt"
    new_file = "new_output.txt"
    compare_files(old_file, new_file)
