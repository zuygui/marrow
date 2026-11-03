package main

import "fmt"

func main() {
	var best, bestLen int64
	for start := int64(1); start < 1000000; start++ {
		x, n := start, int64(1)
		for x != 1 {
			if x%2 == 0 {
				x /= 2
			} else {
				x = 3*x + 1
			}
			n++
		}
		if n > bestLen {
			bestLen = n
			best = start
		}
	}
	fmt.Println(best)
	fmt.Println(bestLen)
}
