#include <stdio.h>
int main(void) {
    long size = 400, inside = 0;
    for (long py = 0; py < size; py++) {
        for (long px = 0; px < size; px++) {
            double cr = 2.0 * (double)px / (double)size - 1.5;
            double ci = 2.0 * (double)py / (double)size - 1.0;
            double zr = 0.0, zi = 0.0;
            long k = 0;
            while (k < 50 && zr * zr + zi * zi <= 4.0) {
                double t = zr * zr - zi * zi + cr;
                zi = 2.0 * zr * zi + ci;
                zr = t;
                k++;
            }
            if (k == 50) inside++;
        }
    }
    printf("%ld\n", inside);
    return 0;
}
