package main

import "fmt"

func main() {
	n := 200
	a := make([]float64, n*n)
	b := make([]float64, n*n)
	c := make([]float64, n*n)
	for i := 0; i < n*n; i++ {
		a[i] = float64(i%7) + 1.0
		b[i] = float64(i%5) + 1.0
	}
	for i := 0; i < n; i++ {
		for k := 0; k < n; k++ {
			aik := a[i*n+k]
			for j := 0; j < n; j++ {
				c[i*n+j] = c[i*n+j] + aik*b[k*n+j]
			}
		}
	}
	sum := 0.0
	for _, v := range c {
		sum = sum + v
	}
	fmt.Println(int64(sum))
}
