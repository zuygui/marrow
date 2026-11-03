#include <stdio.h>
#include <stdlib.h>
int main(void) {
    long n = 5000000;
    unsigned char *composite = calloc(n + 1, 1);
    long count = 0;
    for (long i = 2; i <= n; i++) {
        if (composite[i] == 0) {
            count++;
            for (long j = i * i; j <= n; j += i) composite[j] = 1;
        }
    }
    printf("%ld\n", count);
    free(composite);
    return 0;
}
