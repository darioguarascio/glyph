import sys

def sf(code, i):
    d = 1
    while d > 0:
        i += 1
        x = ord(code[i])
        if x == 91: d += 1
        if x == 93: d -= 1
    return i

def sb(code, i):
    d = 1
    while d > 0:
        i -= 1
        x = ord(code[i])
        if x == 93: d += 1
        if x == 91: d -= 1
    return i

def run(code):
    mem = [0] * 30000
    dp = i = 0
    while i < len(code):
        ch = ord(code[i])
        if ch == 62: dp += 1
        elif ch == 60: dp -= 1
        elif ch == 43: mem[dp] += 1
        elif ch == 45: mem[dp] -= 1
        elif ch == 46: sys.stdout.write(chr(mem[dp]))
        elif ch == 44: mem[dp] = ord(sys.stdin.read(1) or "\0")
        elif ch == 91 and mem[dp] == 0: i = sf(code, i)
        elif ch == 93 and mem[dp] != 0: i = sb(code, i)
        i += 1

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: bf CODE")
        sys.exit(1)
    run(sys.argv[1])
