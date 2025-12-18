use std::{pin::Pin, sync::{Arc, Mutex}, task::{Poll, Waker}, thread, time::Duration};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut handles = vec![];
    for i in 1..=5 {
        handles.push(SleepFuture::new(Duration::from_secs(i as u64)));
    }

    for handle in handles {
        let sec = handle.duration.as_secs();
        handle.await;
        println!("Slept! {}", sec);
    }

    Ok(())
}


struct SleepFuture {
    duration: Duration,
    state : Arc<Mutex<State>>,
}

impl SleepFuture {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            state: Arc::new(Mutex::new(State {
                inner_state: SleepState::Initial,
                waker: None,
            })),
        }
    }

}

struct State {
    inner_state: SleepState,
    waker: Option<Waker>,
}

#[derive(PartialEq, Eq)]
enum SleepState {
    Initial,
    Sleeping,
    Done,
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        println!("Polling...");
        let mut state = self.state.lock().unwrap();
        if state.inner_state == SleepState::Done {
            return Poll::Ready(());
        }

        if state.inner_state == SleepState::Initial {
            state.inner_state = SleepState::Sleeping;
            let waker = cx.waker().clone();
            state.waker = Some(waker);

            let duration = self.duration;
            let state_clone = self.state.clone();
            thread::spawn(move || {
                thread::sleep(duration);
                let mut state = state_clone.lock().unwrap();
                state.inner_state = SleepState::Done;
                if let Some(waker) = state.waker.take() {
                    println!("Waking up...");
                    waker.wake();
                }
            });
        }

        if !cx.waker().will_wake(cx.waker()) {
            println!("Waker will not wake");
            state.waker = Some(cx.waker().clone());
        }

        Poll::Pending
    }
}
