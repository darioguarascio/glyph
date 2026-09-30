package main

import (
	"fmt"
	"os"
)

var mem [30000]int64

func sf(code []byte, i int64) int64 {
	d := int64(1)
	for d > 0 {
		i++
		x := code[i]
		if x == 91 {
			d++
		}
		if x == 93 {
			d--
		}
	}
	return i
}

func sb(code []byte, i int64) int64 {
	d := int64(1)
	for d > 0 {
		i--
		x := code[i]
		if x == 93 {
			d++
		}
		if x == 91 {
			d--
		}
	}
	return i
}

func run(code []byte) {
	dp, i := int64(0), int64(0)
	for i < int64(len(code)) {
		ch := code[i]
		switch ch {
		case 62:
			dp++
		case 60:
			dp--
		case 43:
			mem[dp]++
		case 45:
			mem[dp]--
		case 46:
			fmt.Printf("%c", mem[dp])
		case 44:
			var b [1]byte
			os.Stdin.Read(b[:])
			mem[dp] = int64(b[0])
		case 91:
			if mem[dp] == 0 {
				i = sf(code, i)
			}
		case 93:
			if mem[dp] != 0 {
				i = sb(code, i)
			}
		}
		i++
	}
}

func main() {
	if len(os.Args) < 2 {
		fmt.Println("usage: bf CODE")
		os.Exit(1)
	}
	run([]byte(os.Args[1]))
}
