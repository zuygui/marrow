package main

import "fmt"

func quicksort(a []int64, lo, hi int64) {
	for lo < hi {
		pivot := a[(lo+hi)/2]
		i, j := lo, hi
		for i <= j {
			for a[i] < pivot {
				i++
			}
			for a[j] > pivot {
				j--
			}
			if i <= j {
				a[i], a[j] = a[j], a[i]
				i++
				j--
			}
		}
		if j-lo < hi-i {
			quicksort(a, lo, j)
			lo = i
		} else {
			quicksort(a, i, hi)
			hi = j
		}
	}
}

func main() {
	n := int64(1000000)
	x := int64(12345)
	a := make([]int64, n)
	for i := range a {
		x = (x*1103515245 + 12345) % 2147483648
		a[i] = x
	}
	quicksort(a, 0, n-1)
	ok := 1
	for i := int64(1); i < n; i++ {
		if a[i-1] > a[i] {
			ok = 0
		}
	}
	fmt.Println(ok)
	fmt.Println(a[0])
	fmt.Println(a[n/2])
	fmt.Println(a[n-1])
}
