fn quicksort(a: &mut [i64], mut lo: i64, mut hi: i64) {
    while lo < hi {
        let pivot = a[((lo + hi) / 2) as usize];
        let (mut i, mut j) = (lo, hi);
        while i <= j {
            while a[i as usize] < pivot { i += 1; }
            while a[j as usize] > pivot { j -= 1; }
            if i <= j {
                a.swap(i as usize, j as usize);
                i += 1;
                j -= 1;
            }
        }
        if j - lo < hi - i { quicksort(a, lo, j); lo = i; } else { quicksort(a, i, hi); hi = j; }
    }
}

fn main() {
    let n: usize = 1_000_000;
    let mut x: i64 = 12345;
    let mut a = vec![0i64; n];
    for v in a.iter_mut() {
        x = (x * 1103515245 + 12345) % 2147483648;
        *v = x;
    }
    quicksort(&mut a, 0, n as i64 - 1);
    let ok = a.windows(2).all(|w| w[0] <= w[1]) as i64;
    println!("{}\n{}\n{}\n{}", ok, a[0], a[n / 2], a[n - 1]);
}
