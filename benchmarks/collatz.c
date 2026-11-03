#include <stdio.h>
int main(void) {
    long best = 0, best_len = 0;
    for (long start = 1; start < 1000000; start++) {
        long x = start, len = 1;
        while (x != 1) {
            if (x % 2 == 0) x = x / 2; else x = 3 * x + 1;
            len++;
        }
        if (len > best_len) { best_len = len; best = start; }
    }
    printf("%ld\n%ld\n", best, best_len);
    return 0;
}
