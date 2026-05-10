//! Minimal in-process SMTP stub server for integration tests.
//!
//! Implements RFC 101: accepts the minimum SMTP dialog required to receive
//! a message (`EHLO`, `MAIL FROM`, `RCPT TO`, `DATA`, `QUIT`), records
//! each received message, and makes the log available for test assertions.
//!
//! # Usage
//!
//! ```ignore
//! let stub = SmtpStub::start(0).await;          // port 0 → OS assigns
//! let port = stub.port();
//! // ... submit mail to 127.0.0.1:port ...
//! let msgs = stub.messages();
//! assert_eq!(msgs.len(), 1);
//! assert!(msgs[0].body.contains("Hello"));
//! stub.shutdown();
//! ```

use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single message received by the stub.
#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    pub envelope_from: String,
    pub envelope_to: String,
    pub body: String,
}

/// Configuration for the stub — allows injecting failures.
#[derive(Clone, Debug, Default)]
pub struct StubConfig {
    /// If true, respond 421 immediately on connect (simulates unreachable server).
    pub refuse_connect: bool,
    /// If true, respond 550 to MAIL FROM (simulates delivery rejection).
    pub reject_mail: bool,
}

/// An in-process SMTP stub server.
pub struct SmtpStub {
    port: u16,
    messages: Arc<Mutex<Vec<ReceivedMessage>>>,
    shutdown_tx: oneshot::Sender<()>,
    handle: JoinHandle<()>,
}

impl SmtpStub {
    /// Start the stub on the given port. Pass `0` to let the OS choose.
    pub async fn start(port: u16) -> Self {
        Self::start_with_config(port, StubConfig::default()).await
    }

    /// Start the stub with custom failure injection configuration.
    pub async fn start_with_config(port: u16, config: StubConfig) -> Self {
        let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("smtp stub: bind failed");
        let port = listener.local_addr().unwrap().port();
        let messages: Arc<Mutex<Vec<ReceivedMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let msgs_clone = messages.clone();
        let handle = tokio::spawn(async move {
            run_stub(listener, msgs_clone, config, shutdown_rx).await;
        });

        SmtpStub { port, messages, shutdown_tx, handle }
    }

    /// The port this stub is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// All messages received so far (cloned snapshot).
    pub fn messages(&self) -> Vec<ReceivedMessage> {
        self.messages.lock().unwrap().clone()
    }

    /// Assert that exactly `n` messages were received.
    pub fn assert_count(&self, n: usize) {
        let msgs = self.messages();
        assert_eq!(
            msgs.len(),
            n,
            "expected {n} message(s), got {}",
            msgs.len()
        );
    }

    /// Assert exactly one message was received and return it.
    pub fn assert_one(&self) -> ReceivedMessage {
        self.assert_count(1);
        self.messages().remove(0)
    }

    /// Signal the stub to stop accepting connections and wait for it to exit.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.handle.await;
    }
}

// ---------------------------------------------------------------------------
// Internal: accept loop
// ---------------------------------------------------------------------------

async fn run_stub(
    listener: TcpListener,
    messages: Arc<Mutex<Vec<ReceivedMessage>>>,
    config: StubConfig,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let msgs = messages.clone();
                        let cfg = config.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_session(stream, msgs, cfg).await {
                                eprintln!("smtp_stub: session error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("smtp_stub: accept error: {e}");
                        break;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal: SMTP session state machine
// ---------------------------------------------------------------------------

async fn handle_session(
    stream: TcpStream,
    messages: Arc<Mutex<Vec<ReceivedMessage>>>,
    config: StubConfig,
) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Greeting
    if config.refuse_connect {
        writer.write_all(b"421 Service unavailable\r\n").await?;
        return Ok(());
    }
    writer.write_all(b"220 localhost ESMTP smtp-stub\r\n").await?;

    let mut envelope_from = String::new();
    let mut envelope_to = String::new();
    let mut in_data = false;
    let mut body_lines: Vec<String> = Vec::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // connection closed
        }
        let cmd = line.trim_end();

        if in_data {
            if cmd == "." {
                // End of DATA
                in_data = false;
                let body = body_lines.join("\n");
                messages.lock().unwrap().push(ReceivedMessage {
                    envelope_from: envelope_from.clone(),
                    envelope_to: envelope_to.clone(),
                    body,
                });
                body_lines.clear();
                writer.write_all(b"250 OK: message accepted\r\n").await?;
            } else {
                // Strip leading dot (dot-stuffing per RFC 5321)
                let data_line = cmd.strip_prefix('.').unwrap_or(cmd);
                body_lines.push(data_line.to_string());
            }
            continue;
        }

        let upper = cmd.to_ascii_uppercase();

        if upper.starts_with("EHLO") || upper.starts_with("HELO") {
            writer.write_all(b"250-localhost\r\n250 OK\r\n").await?;
        } else if upper.starts_with("MAIL FROM:") {
            if config.reject_mail {
                writer.write_all(b"550 Rejected\r\n").await?;
            } else {
                envelope_from = extract_address(cmd);
                writer.write_all(b"250 OK\r\n").await?;
            }
        } else if upper.starts_with("RCPT TO:") {
            envelope_to = extract_address(cmd);
            writer.write_all(b"250 OK\r\n").await?;
        } else if upper == "DATA" {
            writer.write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n").await?;
            in_data = true;
        } else if upper == "QUIT" {
            writer.write_all(b"221 Bye\r\n").await?;
            break;
        } else if upper.starts_with("NOOP") {
            writer.write_all(b"250 OK\r\n").await?;
        } else {
            writer.write_all(b"502 Command not implemented\r\n").await?;
        }
    }

    Ok(())
}

fn extract_address(line: &str) -> String {
    // Extract address from "MAIL FROM:<addr>" or "RCPT TO:<addr>"
    let inner = line
        .splitn(2, ':')
        .nth(1)
        .unwrap_or("")
        .trim();
    inner
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string()
}

// ---------------------------------------------------------------------------
// Tests for the stub itself
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_accepts_smtp_session() {
        let stub = SmtpStub::start(0).await;
        let port = stub.port();

        // Connect and send a minimal SMTP session
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buf = vec![0u8; 256];

        // Greeting
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("220"));

        // Dialog
        stream.write_all(b"EHLO test\r\n").await.unwrap();
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("250"));

        stream.write_all(b"MAIL FROM:<sender@example.com>\r\n").await.unwrap();
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("250"));

        stream.write_all(b"RCPT TO:<recipient@example.com>\r\n").await.unwrap();
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("250"));

        stream.write_all(b"DATA\r\n").await.unwrap();
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("354"));

        stream.write_all(b"Subject: Hello\r\n\r\nTest body.\r\n.\r\n").await.unwrap();
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("250"));

        stream.write_all(b"QUIT\r\n").await.unwrap();
        let _ = stream.read(&mut buf).await;

        stub.shutdown().await;
    }

    #[tokio::test]
    async fn stub_records_one_message() {
        let stub = SmtpStub::start(0).await;
        let port = stub.port();

        // Use a line-buffered client to match the stub's line-by-line protocol
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let (r, mut w) = stream.into_split();
        let mut reader = BufReader::new(r);
        let mut line = String::new();

        // Greeting
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("220"), "bad greeting: {line}");

        // One command at a time
        let exchanges: &[(&[u8], &str)] = &[
            (b"EHLO test\r\n",              "250"),
            (b"MAIL FROM:<a@b.com>\r\n",    "250"),
            (b"RCPT TO:<c@d.com>\r\n",      "250"),
            (b"DATA\r\n",                   "354"),
        ];
        for (cmd, expected) in exchanges {
            w.write_all(cmd).await.unwrap();
            // Read until we get a non-continuation response (no dash after code)
            loop {
                line.clear();
                reader.read_line(&mut line).await.unwrap();
                // Multi-line responses: "250-..." continues, "250 ..." ends
                let is_continuation = line.len() > 3 && line.as_bytes()[3] == b'-';
                if !is_continuation {
                    break;
                }
            }
            assert!(line.starts_with(expected), "cmd={cmd:?} expected {expected} got {line}");
        }

        // Body
        w.write_all(b"Subject: Hi\r\n\r\nHello world\r\n.\r\n").await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("250"), "data accepted: {line}");

        // Quit
        w.write_all(b"QUIT\r\n").await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("221"), "quit: {line}");

        stub.assert_count(1);
        let msg = stub.assert_one();
        assert!(msg.envelope_from.contains("a@b.com"));
        assert!(msg.envelope_to.contains("c@d.com"));
        assert!(msg.body.contains("Hello world"));

        stub.shutdown().await;
    }

    #[tokio::test]
    async fn stub_refuse_connect_returns_421() {
        let cfg = StubConfig { refuse_connect: true, ..Default::default() };
        let stub = SmtpStub::start_with_config(0, cfg).await;
        let port = stub.port();

        use tokio::io::AsyncReadExt;
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let mut buf = vec![0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("421"));

        stub.shutdown().await;
    }

    #[tokio::test]
    async fn stub_reject_mail_returns_550() {
        let cfg = StubConfig { reject_mail: true, ..Default::default() };
        let stub = SmtpStub::start_with_config(0, cfg).await;
        let port = stub.port();

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let mut buf = vec![0u8; 256];
        stream.read(&mut buf).await.unwrap(); // greeting
        stream.write_all(b"EHLO x\r\n").await.unwrap();
        stream.read(&mut buf).await.unwrap(); // 250
        stream.write_all(b"MAIL FROM:<a@b.com>\r\n").await.unwrap();
        let n = stream.read(&mut buf).await.unwrap();
        assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("550"));

        stub.shutdown().await;
    }
}
