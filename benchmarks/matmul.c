#include <stdio.h>
#include <stdlib.h>
int main(void) {
    long n = 200;
    double *a = malloc(n * n * 8), *b = malloc(n * n * 8), *c = malloc(n * n * 8);
    for (long i = 0; i < n * n; i++) { a[i] = (double)(i % 7) + 1.0; b[i] = (double)(i % 5) + 1.0; c[i] = 0.0; }
    for (long i = 0; i < n; i++)
        for (long k = 0; k < n; k++) {
            double aik = a[i * n + k];
            for (long j = 0; j < n; j++) c[i * n + j] = c[i * n + j] + aik * b[k * n + j];
        }
    double sum = 0.0;
    for (long i = 0; i < n * n; i++) sum = sum + c[i];
    printf("%ld\n", (long)sum);
    free(a); free(b); free(c);
    return 0;
}
