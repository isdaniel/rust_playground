use mini_redis::{client};
use tokio::sync::{oneshot,mpsc};
use bytes::Bytes;

#[derive(Debug)]
enum Commnad{
    Get {
        key: String,
        resp : Responder<Option<Bytes>>,
    },
    Set {
        key : String,
        val : Bytes,
        resp: Responder<()>,
    }
}

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[tokio::main]
async fn main()  {
    let (tx,mut rx) = mpsc::channel(32);
    let tx1 = tx.clone();
    let manager = tokio::spawn(async move{
        let mut client = client::connect("localhost:6379").await.unwrap();
        while let Some(cmd) = rx.recv().await {
            use Commnad::*;
            match cmd {
                Get {key,resp} =>{
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                },
                Set { key, val,resp} =>{
                    let res = client.set(&key, val).await;
                    let _ = resp.send(res);
                }
            }
        }
    });

    let get_task: tokio::task::JoinHandle<()> = tokio::spawn(async move{
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd  = Commnad::Get {
            key: "test".to_string(),
            resp : resp_tx
        };
        tx.send(cmd).await.unwrap();

        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    let set_task = tokio::spawn(async move{
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd  = Commnad::Set {
            key : "test".to_string(),
            val : "value_123".into(),
            resp : resp_tx
        };
        tx1.send(cmd).await.unwrap();

        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    get_task.await.unwrap();
    set_task.await.unwrap();
    manager.await.unwrap();
}
