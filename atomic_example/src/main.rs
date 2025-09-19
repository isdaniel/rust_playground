use std::{sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, thread, time::Instant};

use atomic_example::{FreeList, MutexLinkedList};

fn benchmark_spin_mutex(threads: usize, iterations: usize) -> u128 {
    let spin = Arc::new(SpinMutex::new());
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = Vec::new();

    let start = Instant::now();

    for _ in 0..threads {
        let spin = Arc::clone(&spin);
        let counter = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            for _ in 0..iterations {
                spin.lock();
                counter.fetch_add(1, Ordering::Relaxed);
                spin.unlock();
            }
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed().as_millis();

    let expected = threads * iterations;
    let result = counter.load(Ordering::Relaxed);
    assert_eq!(result, expected);

    elapsed
}

fn benchmark_std_mutex(threads: usize, iterations: usize) -> u128 {
    let mutex = Arc::new(Mutex::new(0usize));
    let mut handles = Vec::new();

    let start = Instant::now();

    for _ in 0..threads {
        let mutex = Arc::clone(&mutex);

        let handle = thread::spawn(move || {
            for _ in 0..iterations {
                let mut guard = mutex.lock().unwrap();
                *guard += 1;
            }
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed().as_millis();

    let result = *mutex.lock().unwrap();
    let expected = threads * iterations;
    assert_eq!(result, expected);

    elapsed
}

fn main() {
    const THREADS: usize = 8;
    const ITERATIONS: usize = 1_000_000;

    let spin_time = benchmark_spin_mutex(THREADS, ITERATIONS);
    let std_time = benchmark_std_mutex(THREADS, ITERATIONS);

    println!(
        "SpinMutex: {} ms, Std Mutex: {} ms ({} threads, {} iterations)",
        spin_time, std_time, THREADS, ITERATIONS
    );

    let mlist_ms = bench_freelock_linkedlist(THREADS, ITERATIONS);
    println!("FreeList: {} ms", mlist_ms);

    let mlist_ms = bench_mutex_linkedlist(THREADS, ITERATIONS);
    println!("MutexLinkedList (linked nodes + Mutex): {} ms", mlist_ms);
}

fn bench_freelock_linkedlist(thread_count: usize, iters: usize) -> u128 {
    let list = Arc::new(FreeList::new());

    let start = Instant::now();
    let mut handles = Vec::with_capacity(thread_count);
    for t in 0..thread_count {
        let s = list.clone();
        handles.push(thread::spawn(move || {
            for i in 0..iters {
                s.push(t * iters + i);
                let _ = s.pop();
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    start.elapsed().as_millis()
}

fn bench_mutex_linkedlist(thread_count: usize, iters: usize) -> u128 {
    let list = Arc::new(MutexLinkedList::<usize>::new());

    let start = Instant::now();
    let mut handles = Vec::with_capacity(thread_count);
    for t in 0..thread_count {
        let s = list.clone();
        handles.push(thread::spawn(move || {
            for i in 0..iters {
                s.push(t * iters + i);
                let _ = s.pop();
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    start.elapsed().as_millis()
}


struct SpinMutex {
    flag : AtomicBool,
}

impl SpinMutex {
    fn new() -> Self {
        SpinMutex {
            flag: AtomicBool::new(false),
        }
    }

    fn lock(&self) {
        let mut spin_count = 1;
        while self.flag.swap(true, Ordering::Acquire) {
            for _ in 0..spin_count {
                std::hint::spin_loop();
            }

            if spin_count < 64 {
                spin_count *= 2;
            }
        }
    }

    fn unlock(&self) {
        self.flag.store(false, Ordering::Release);
    }
}


