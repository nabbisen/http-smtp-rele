# RFC 304 — Sendmail Pipe Mode

**Status.** Implemented  
**Tracks.** SMTP / Platform  
**Touches.** `src/smtp.rs`, `src/security.rs`, `src/config.rs`

## Summary

Implement `smtp.mode = "pipe"`: submit mail by piping the message to a local MTA command
(`sendmail -t`) instead of a direct SMTP TCP connection.

## Design

```toml
[smtp]
mode = "smtp"   # or "pipe"
pipe_command = "/usr/sbin/sendmail"  # default; only used when mode = "pipe"
```

Submission:
```rust
use tokio::process::Command;
use tokio::io::AsyncWriteExt;

let mut child = Command::new(&config.pipe_command)
    .arg("-t")        // read recipients from headers
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::piped())
    .spawn()?;

child.stdin.take().unwrap()
    .write_all(message_bytes).await?;

let output = child.wait_with_output().await?;
if !output.status.success() { return Err(SmtpError::PipeFailed); }
```

## OpenBSD pledge update

When pipe mode is active, pledge must include `exec proc`:
```rust
// mode = "smtp":  pledge("stdio inet")
// mode = "pipe":  pledge("stdio exec proc")
//                 unveil("/usr/sbin/sendmail", "x")
```

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-304-01 | `mode = "pipe"` submits mail via the configured command. |
| AC-304-02 | Exit code != 0 from the pipe command returns 502. |
| AC-304-03 | OpenBSD pledge is `"stdio exec proc"` in pipe mode. |
| AC-304-04 | `mode = "smtp"` pledge remains `"stdio inet"`. |
