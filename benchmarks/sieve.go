package main

import "fmt"

func main() {
	n := 5000000
	composite := make([]byte, n+1)
	count := 0
	for i := 2; i <= n; i++ {
		if composite[i] == 0 {
			count++
			for j := i * i; j <= n; j += i {
				composite[j] = 1
			}
		}
	}
	fmt.Println(count)
}
