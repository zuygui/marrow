fn main() {
    let size: i64 = 400;
    let mut inside: i64 = 0;
    for py in 0..size {
        for px in 0..size {
            let cr = 2.0 * px as f64 / size as f64 - 1.5;
            let ci = 2.0 * py as f64 / size as f64 - 1.0;
            let (mut zr, mut zi) = (0.0f64, 0.0f64);
            let mut k: i64 = 0;
            while k < 50 && zr * zr + zi * zi <= 4.0 {
                let t = zr * zr - zi * zi + cr;
                zi = 2.0 * zr * zi + ci;
                zr = t;
                k += 1;
            }
            if k == 50 { inside += 1; }
        }
    }
    println!("{}", inside);
}
