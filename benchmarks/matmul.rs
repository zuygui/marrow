fn main() {
    let n: usize = 200;
    let mut a = vec![0.0f64; n * n];
    let mut b = vec![0.0f64; n * n];
    let mut c = vec![0.0f64; n * n];
    for i in 0..n * n {
        a[i] = (i % 7) as f64 + 1.0;
        b[i] = (i % 5) as f64 + 1.0;
    }
    for i in 0..n {
        for k in 0..n {
            let aik = a[i * n + k];
            for j in 0..n {
                c[i * n + j] = c[i * n + j] + aik * b[k * n + j];
            }
        }
    }
    let mut sum = 0.0f64;
    for v in &c { sum = sum + v; }
    println!("{}", sum as i64);
}
