#include <stdio.h>
#include <stdlib.h>
static void quicksort(long *a, long lo, long hi) {
    while (lo < hi) {
        long pivot = a[(lo + hi) / 2], i = lo, j = hi;
        while (i <= j) {
            while (a[i] < pivot) i++;
            while (a[j] > pivot) j--;
            if (i <= j) { long t = a[i]; a[i] = a[j]; a[j] = t; i++; j--; }
        }
        if (j - lo < hi - i) { quicksort(a, lo, j); lo = i; }
        else { quicksort(a, i, hi); hi = j; }
    }
}
int main(void) {
    long n = 1000000, x = 12345;
    long *a = malloc(n * sizeof(long));
    for (long i = 0; i < n; i++) { x = (x * 1103515245 + 12345) % 2147483648; a[i] = x; }
    quicksort(a, 0, n - 1);
    long ok = 1;
    for (long i = 1; i < n; i++) if (a[i - 1] > a[i]) ok = 0;
    printf("%ld\n%ld\n%ld\n%ld\n", ok, a[0], a[n / 2], a[n - 1]);
    free(a);
    return 0;
}
