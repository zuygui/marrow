fn main() {
    let mut best: i64 = 0;
    let mut best_len: i64 = 0;
    for start in 1..1_000_000i64 {
        let mut x = start;
        let mut len: i64 = 1;
        while x != 1 {
            if x % 2 == 0 { x /= 2; } else { x = 3 * x + 1; }
            len += 1;
        }
        if len > best_len { best_len = len; best = start; }
    }
    println!("{}\n{}", best, best_len);
}
