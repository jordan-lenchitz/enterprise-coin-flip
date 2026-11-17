use std::io::{self, BufWriter, Write};
use std::sync::{mpsc, OnceLock};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[derive(Clone)]
struct NonBlockingStdout {
    sender: mpsc::SyncSender<Vec<u8>>,
}

struct ChannelWriter {
    sender: mpsc::SyncSender<Vec<u8>>,
    buf: Vec<u8>,
}

impl Write for ChannelWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            let chunk = std::mem::take(&mut self.buf);
            let _ = self.sender.try_send(chunk);
        }
        Ok(())
    }
}

impl Drop for ChannelWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl<'a> MakeWriter<'a> for NonBlockingStdout {
    type Writer = ChannelWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ChannelWriter {
            sender: self.sender.clone(),
            buf: Vec::with_capacity(256),
        }
    }
}

fn non_blocking_stdout() -> NonBlockingStdout {
    static SENDER: OnceLock<mpsc::SyncSender<Vec<u8>>> = OnceLock::new();
    let sender = SENDER
        .get_or_init(|| {
            let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(8192);
            std::thread::Builder::new()
                .name("telemetry-stdout".to_string())
                .spawn(move || {
                    let stdout = io::stdout();
                    let mut writer = BufWriter::with_capacity(64 * 1024, stdout.lock());
                    while let Ok(chunk) = rx.recv() {
                        let _ = writer.write_all(&chunk);
                        // Drain bursts before flushing to coalesce stdout syscalls.
                        while let Ok(more) = rx.try_recv() {
                            let _ = writer.write_all(&more);
                        }
                        let _ = writer.flush();
                    }
                })
                .expect("failed to spawn telemetry writer thread");
            tx
        })
        .clone();
    NonBlockingStdout { sender }
}

pub fn init_telemetry() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,enterprise_coin_flip=info,actix_web=info"));

    let formatting_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking_stdout())
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::NONE);

    let _ = Registry::default()
        .with(env_filter)
        .with(formatting_layer)
        .try_init();

    tracing::info!("Enterprise telemetry pipeline initialized successfully");
}
