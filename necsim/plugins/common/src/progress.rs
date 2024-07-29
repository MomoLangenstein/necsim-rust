use std::{
    fmt,
    io::{self, Write},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Sender, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use necsim_core::{impl_report, reporter::Reporter};

struct ProgressUpdater {
    thread: JoinHandle<()>,
    sender: Sender<()>,
}

#[expect(clippy::module_name_repetitions)]
pub struct ProgressReporter {
    updater: Option<ProgressUpdater>,
    last_remaining: Arc<AtomicU64>,
    last_total: Arc<AtomicU64>,
}

impl fmt::Debug for ProgressReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct Progress(u64, u64);

        impl fmt::Debug for Progress {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "{} / {}", self.0, self.1)
            }
        }

        let total = self.last_total.load(Ordering::Acquire);
        let remaining = self.last_remaining.load(Ordering::Acquire).min(total);

        fmt.debug_struct(stringify!(ProgressReporter))
            .field("progress", &Progress(total - remaining, total))
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for ProgressReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

impl<'de> serde::Deserialize<'de> for ProgressReporter {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::default())
    }
}

impl Reporter for ProgressReporter {
    impl_report!(speciation(&mut self, _speciation: Ignored) {});

    impl_report!(dispersal(&mut self, _dispersal: Ignored) {});

    impl_report!(progress(&mut self, remaining: Used) {
        let last_remaining = self.last_remaining.swap(*remaining, Ordering::AcqRel);

        // Update the progress total in case of regression
        if last_remaining < *remaining {
            self.last_total
                .fetch_add(remaining - last_remaining, Ordering::AcqRel);
        }

        if last_remaining > 0 && *remaining == 0 {
            let total = self.last_total.load(Ordering::Acquire);

            display_progress(total, self.last_remaining.load(Ordering::Acquire).min(total));

            // Flush stdout to update the progress bar
            std::mem::drop(io::stdout().flush());
        }
    });

    fn initialise(&mut self) -> Result<(), String> {
        if self.updater.is_some() {
            return Ok(());
        }

        let remaining = Arc::clone(&self.last_remaining);
        let total = Arc::clone(&self.last_total);

        let (sender, receiver) = mpsc::channel();

        let thread = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(100));

                match receiver.try_recv() {
                    Ok(()) | Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {},
                }

                let total = total.load(Ordering::Acquire);

                if total > 0 {
                    display_progress(total, remaining.load(Ordering::Acquire).min(total));

                    // Flush stdout to update the progress bar
                    std::mem::drop(io::stdout().flush());
                }
            }
        });

        self.updater = Some(ProgressUpdater { thread, sender });

        Ok(())
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        if let Some(updater) = self.updater.take() {
            if updater.sender.send(()).is_ok() {
                std::mem::drop(updater.thread.join());
            }
        }
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        let last_remaining = Arc::new(AtomicU64::new(0_u64));
        let last_total = Arc::new(AtomicU64::new(0_u64));

        Self {
            updater: None,
            last_remaining,
            last_total,
        }
    }
}

fn display_progress(total: u64, remaining: u64) {
    const UPDATE_PRECISION: usize = 50;

    #[expect(clippy::cast_possible_truncation)]
    let display_progress =
        ((total - remaining) * (UPDATE_PRECISION as u64) / total.max(1)) as usize;

    // Display a simple progress bar to stdout
    print!("\r{:>13} [", total - remaining);
    if display_progress == 0 {
        print!("{:>UPDATE_PRECISION$}", "");
    } else if remaining > 0 {
        print!(
            "{:=<progress$}>{:>rest$}",
            "",
            "",
            progress = (display_progress - 1),
            rest = (UPDATE_PRECISION - display_progress)
        );
    } else {
        print!("{:=<UPDATE_PRECISION$}", "");
    }
    print!("] {total:<13}");
}
