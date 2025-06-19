use std::time::{Duration, Instant};
use mini_tokio::{Delay, MiniTokio};

fn main() {
    let mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async {
        let when = Instant::now() + Duration::from_millis(10);
        let future = Delay { when };

        let out = future.await;
    });

    mini_tokio.run();
}

// #[tokio::main]
// async fn main() {
//     let when = Instant::now() + Duration::from_millis(10);
//     let future = Delay { when };
//     let _ = future.await;
// }
