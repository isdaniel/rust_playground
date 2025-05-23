use std::{pin::{pin, Pin}, time::Duration};

use trpl::{self, Either};
fn main() {
    trpl::run(async{
        let (tx,mut rx) = trpl::channel();

        let task1 = pin!(async move{
            let vals = vec![
                String::from("Hello"),
                String::from("World"),
                String::from("Rust!"),
            ];

            for val in vals {
                tx.send(val).unwrap();
                trpl::sleep(Duration::from_secs(1)).await;
            }
        });

        let task2 = pin!(async {
            while let Some(val) = rx.recv().await {
                println!("Got: {}", val);
            }
        });

        let tasks :Vec<Pin<&mut dyn Future<Output = ()>>> = vec![task1,task2];

        trpl::join_all(tasks).await;
    });

    trpl::run(async{
        match timeout( async {
            trpl::sleep(Duration::from_secs(2)).await;
            "Hello".to_string()
        } ,Duration::from_secs(1)).await {
            Ok(val) => println!("Got: {}", val),
            Err(timeout) => println!("over {} seconds, timeout",timeout.as_secs()),
        }
    });
}

async fn timeout<F:Future>(func : F, timeout_interval: Duration) -> Result<F::Output,Duration> {
    match trpl::race(func,trpl::sleep(timeout_interval)).await {
        Either::Left(val) => Ok(val),
        Either::Right(_) => Err(timeout_interval)
    }
}