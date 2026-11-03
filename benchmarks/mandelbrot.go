package main

import "fmt"

func main() {
	size, inside := int64(400), int64(0)
	for py := int64(0); py < size; py++ {
		for px := int64(0); px < size; px++ {
			cr := 2.0*float64(px)/float64(size) - 1.5
			ci := 2.0*float64(py)/float64(size) - 1.0
			zr, zi := 0.0, 0.0
			k := int64(0)
			for k < 50 && zr*zr+zi*zi <= 4.0 {
				t := zr*zr - zi*zi + cr
				zi = 2.0*zr*zi + ci
				zr = t
				k++
			}
			if k == 50 {
				inside++
			}
		}
	}
	fmt.Println(inside)
}
