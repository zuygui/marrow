fn main() {
    let n: usize = 5_000_000;
    let mut composite = vec![0u8; n + 1];
    let mut count: i64 = 0;
    for i in 2..=n {
        if composite[i] == 0 {
            count += 1;
            let mut j = i * i;
            while j <= n {
                composite[j] = 1;
                j += i;
            }
        }
    }
    println!("{}", count);
}
