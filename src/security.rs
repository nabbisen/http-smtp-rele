//! Platform security hardening.
//!
//! On OpenBSD: applies `pledge(2)` and `unveil(2)` restrictions.
//! On other platforms: no-op with a log warning.
//!
//! # Pledge sets by mode (RFC 091, RFC 304)
//!
//! | Mode       | pledge promises            | unveil                          |
//! |------------|----------------------------|---------------------------------|
//! | SmtpRelay  | `"stdio inet"`             | `unveil(NULL, NULL)`            |
//! | SendmailPipe | `"stdio exec proc"`      | `unveil("/usr/sbin/sendmail","x")` then lock |

use std::path::Path;

/// Runtime operation mode — selects the pledge promise set.
pub enum RuntimeMode {
    /// Direct SMTP relay over TCP. Requires `inet` only.
    SmtpRelay,
    /// Pipe mail through a sendmail-compatible command. Requires `exec proc`.
    SendmailPipe {
        /// Absolute path to the sendmail binary (e.g. `/usr/sbin/sendmail`).
        pipe_command: String,
    },
}

/// Apply filesystem restrictions before reading the config file.
pub fn apply_initial_restrictions(config_path: &Path) -> Result<(), String> {
    platform::apply_initial_restrictions(config_path)
}

/// Apply runtime restrictions after all initialization is complete.
pub fn apply_runtime_restrictions(mode: RuntimeMode) -> Result<(), String> {
    platform::apply_runtime_restrictions(mode)
}

// ---------------------------------------------------------------------------
// Platform-specific implementations
// ---------------------------------------------------------------------------

#[cfg(target_os = "openbsd")]
mod platform {
    use super::RuntimeMode;
    use std::path::Path;

    pub fn apply_initial_restrictions(config_path: &Path) -> Result<(), String> {
        unveil::unveil(config_path, "r")
            .map_err(|e| format!("unveil config path failed: {}", e))?;
        unveil::unveil("", "")
            .map_err(|e| format!("unveil lock failed: {}", e))?;
        pledge::pledge("stdio rpath inet", None)
            .map_err(|e| format!("initial pledge failed: {}", e))?;
        Ok(())
    }

    pub fn apply_runtime_restrictions(mode: RuntimeMode) -> Result<(), String> {
        match mode {
            RuntimeMode::SmtpRelay => {
                // No file access needed after config load.
                pledge::pledge("stdio inet", None)
                    .map_err(|e| format!("runtime pledge failed: {}", e))?;
            }
            RuntimeMode::SendmailPipe { ref pipe_command } => {
                // Unveil only the sendmail binary, then lock.
                unveil::unveil(pipe_command, "x")
                    .map_err(|e| format!("unveil pipe command failed: {}", e))?;
                unveil::unveil("", "")
                    .map_err(|e| format!("unveil lock failed: {}", e))?;
                // exec proc: needed to spawn the sendmail child process.
                // NOTE: stdio inet is NOT included — pipe mode does not open TCP connections.
                pledge::pledge("stdio exec proc", None)
                    .map_err(|e| format!("runtime pledge (pipe) failed: {}", e))?;
            }
        }
        Ok(())
    }
}

#[cfg(not(target_os = "openbsd"))]
mod platform {
    use super::RuntimeMode;
    use std::path::Path;

    pub fn apply_initial_restrictions(_config_path: &Path) -> Result<(), String> {
        tracing::warn!(
            "OpenBSD pledge/unveil are not available on this platform; \
             security restrictions are not applied"
        );
        Ok(())
    }

    pub fn apply_runtime_restrictions(_mode: RuntimeMode) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_restrictions_succeeds_on_non_openbsd() {
        let result = apply_initial_restrictions(Path::new("/etc/http-smtp-rele.toml"));
        assert!(result.is_ok());
    }

    #[test]
    fn apply_runtime_restrictions_smtp_relay_succeeds_on_non_openbsd() {
        let result = apply_runtime_restrictions(RuntimeMode::SmtpRelay);
        assert!(result.is_ok());
    }

    #[test]
    fn apply_runtime_restrictions_pipe_mode_succeeds_on_non_openbsd() {
        let result = apply_runtime_restrictions(RuntimeMode::SendmailPipe {
            pipe_command: "/usr/sbin/sendmail".into(),
        });
        assert!(result.is_ok());
    }
}
