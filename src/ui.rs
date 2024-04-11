use futures::{Future, FutureExt};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;
use std::{cell::RefCell, collections::HashMap};
use tokio::task::{spawn_blocking, JoinHandle};

#[derive(Clone)]
pub struct Ui {
    tx: Sender<Signal>,
}

pub struct UiThreadHandle(JoinHandle<()>);

impl Future for UiThreadHandle {
    type Output = std::result::Result<(), tokio::task::JoinError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx)
    }
}

struct UiThread {
    progress: MultiProgress,
    style: ProgressStyle,
    state: RefCell<HashMap<String, ProgressBar>>,
}

#[derive(Debug)]
enum Signal {
    Update(String, u64, u64),
}

impl Ui {
    pub fn new() -> (Self, UiThreadHandle) {
        let sty =
            ProgressStyle::with_template("[{elapsed_precise}] {bar:50.cyan/blue} {pos:>6}/{len:6}% {spinner} {msg}")
                .unwrap()
                .progress_chars("●●·");

        let (tx, rx) = channel::<Signal>();

        let mut inner = UiThread {
            progress: MultiProgress::with_draw_target(ProgressDrawTarget::stdout_with_hz(60)),
            style: sty,
            state: RefCell::new(HashMap::new()),
        };

        let thread_handle = spawn_blocking(move || {
            while let Ok(msg) = rx.recv() {
                match msg {
                    Signal::Update(msg, current, total) => inner.handle_update(&msg, current, total),
                }
            }
        });

        (Ui { tx }, UiThreadHandle(thread_handle))
    }

    pub fn update(&self, msg: &str, current: u64, total: u64) {
        self.tx
            .send(Signal::Update(msg.to_string(), current, total))
            .expect("Failed to dispatch signal");
    }
}

impl UiThread {
    fn handle_update(&mut self, msg: &str, current: u64, total: u64) {
        assert!(current <= total, "current={} total={}", current, total);
        assert!(total > 0, "total={}", total);

        let mut progress_bars = self.state.borrow_mut();

        let progress_bar = progress_bars.entry(msg.to_string()).or_insert_with(|| {
            let progress_bar = self.progress.add(ProgressBar::new(total));
            progress_bar.set_style(self.style.clone());
            progress_bar.set_message(msg.to_string());
            progress_bar.enable_steady_tick(Duration::from_secs_f64(16.666666666667 / 1000.0));
            progress_bar
        });

        let pos = progress_bar.position();
        if current == pos {
            return;
        }

        progress_bar.set_position(current);

        if current == total && current != pos {
            progress_bar.finish();
        }
    }
}
