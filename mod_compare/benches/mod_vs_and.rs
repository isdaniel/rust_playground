use criterion::{black_box, criterion_group, criterion_main, Criterion};

//compare mod 2^n with bitwise &
//mod 2^n is slower than bitwise &
//this is because mod is a division operation, while bitwise & is a simple bit manipulation
//the performance difference is more pronounced with larger numbers
//this is a common optimization in performance-critical code
//especially in graphics programming, where you often need to wrap values around a power of two
//in this case, we are comparing modulo 1024 with bitwise AND with 1023 (1024 - 1)
//the bitwise operation is faster because it directly manipulates the bits of the number
//while the modulo operation involves division, which is more computationally expensive
//this benchmark will show the difference in performance between the two operations
fn bench_mod(c: &mut Criterion) {
    let n = 1_000_000u32;

    c.bench_function("modulo %", |b| {
        b.iter(|| {
            let mut sum = 0;
            for i in 0..n {
                sum += black_box(i % 1024);
            }
            sum
        })
    });

    c.bench_function("bitwise &", |b| {
        b.iter(|| {
            let mut sum = 0;
            for i in 0..n {
                sum += black_box(i & 1023); // 1024 - 1
            }
            sum
        })
    });
}

criterion_group!(benches, bench_mod);
criterion_main!(benches);
