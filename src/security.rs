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

/// Apply filesystem restrictions for the SQLite status store database (RFC 088).
///
/// On OpenBSD: calls `unveil(db_path, "rwc")` to allow read/write access.
/// Must be called BEFORE `apply_runtime_restrictions` (which locks unveil).
/// On other platforms: no-op.
pub fn apply_sqlite_restrictions(db_path: &std::path::Path) -> Result<(), String> {
    platform::apply_sqlite_restrictions(db_path)
}

/// Apply runtime restrictions after all initialization is complete.
///
/// `has_sqlite` controls whether `rpath wpath cpath` are added to the pledge set.
pub fn apply_runtime_restrictions(mode: RuntimeMode, has_sqlite: bool) -> Result<(), String> {
    platform::apply_runtime_restrictions(mode, has_sqlite)
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

    pub fn apply_runtime_restrictions(mode: RuntimeMode, has_sqlite: bool) -> Result<(), String> {
        match mode {
            RuntimeMode::SmtpRelay => {
                let file_prom = if has_sqlite { " rpath wpath cpath" } else { "" };
                pledge::pledge(&format!("stdio inet{file_prom}"), None)
                    .map_err(|e| format!("runtime pledge failed: {}", e))?;
            }
            RuntimeMode::SendmailPipe { ref pipe_command } => {
                // Unveil only the sendmail binary, then lock.
                unveil::unveil(pipe_command, "x")
                    .map_err(|e| format!("unveil pipe command failed: {}", e))?;
                unveil::unveil("", "")
                    .map_err(|e| format!("unveil lock failed: {}", e))?;
                let file_prom = if has_sqlite { " rpath wpath cpath" } else { "" };
                pledge::pledge(&format!("stdio exec proc{file_prom}"), None)
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

    pub fn apply_sqlite_restrictions(_db_path: &std::path::Path) -> Result<(), String> { Ok(()) }
    pub fn apply_runtime_restrictions(_mode: RuntimeMode, _has_sqlite: bool) -> Result<(), String> { Ok(()) }
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
        let result = apply_runtime_restrictions(RuntimeMode::SmtpRelay, false);
        assert!(result.is_ok());
    }

    #[test]
    fn apply_runtime_restrictions_pipe_mode_succeeds_on_non_openbsd() {
        let result = apply_runtime_restrictions(RuntimeMode::SendmailPipe {
            pipe_command: "/usr/sbin/sendmail".into(),
        }, false);
        assert!(result.is_ok());
    }
}
