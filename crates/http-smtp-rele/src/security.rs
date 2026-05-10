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
mod openbsd_impl {
    /// Phase 1: unveil the config file path ONLY — do NOT lock unveil yet (RFC 820).
    /// Called before config is loaded so we can read the config file.
    pub fn unveil_config(config_path: &std::path::Path) -> Result<(), String> {
        unveil::unveil(config_path, "r")
            .map_err(|e| format!("unveil config failed: {e}"))
    }

    /// Phase 2: unveil all runtime paths, then lock unveil (RFC 820).
    /// Called after config is parsed — all needed paths are now known.
    pub fn unveil_runtime_paths(
        config_path: &std::path::Path,
        sqlite_path: Option<&std::path::Path>,
        tls_cert:    Option<&std::path::Path>,
        tls_key:     Option<&std::path::Path>,
    ) -> Result<(), String> {
        // Re-register config (idempotent)
        unveil::unveil(config_path, "r")
            .map_err(|e| format!("unveil config failed: {e}"))?;

        if let Some(p) = sqlite_path {
            // Ensure the parent dir is also accessible for db creation
            if let Some(parent) = p.parent() {
                let _ = unveil::unveil(parent, "rwc");
            }
            unveil::unveil(p, "rwc")
                .map_err(|e| format!("unveil sqlite failed: {e}"))?;
        }
        if let Some(p) = tls_cert {
            unveil::unveil(p, "r")
                .map_err(|e| format!("unveil tls_cert failed: {e}"))?;
        }
        if let Some(p) = tls_key {
            unveil::unveil(p, "r")
                .map_err(|e| format!("unveil tls_key failed: {e}"))?;
        }

        // Lock unveil table — no more paths can be added after this point.
        unveil::unveil("", "")
            .map_err(|e| format!("unveil lock failed: {e}"))
    }

    /// Phase 3: apply the final runtime pledge (RFC 820, RFC 721).
    ///
    /// `rpath` is always kept for SIGHUP config reload (RFC 721).
    pub fn apply_runtime_pledge(mode: super::RuntimeMode, has_sqlite: bool) -> Result<(), String> {
        let extra = if has_sqlite { " wpath cpath" } else { "" };
        match mode {
            super::RuntimeMode::SmtpRelay => {
                pledge::pledge(&format!("stdio inet rpath{extra}"), None)
                    .map_err(|e| format!("runtime pledge failed: {e}"))
            }
            super::RuntimeMode::SendmailPipe { .. } => {
                // Pipe mode rejected at config validation (RFC 815)
                Err("pipe mode is not supported in v0.15".into())
            }
        }
    }
}

#[cfg(target_os = "openbsd")]
pub use openbsd_impl::{unveil_config, unveil_runtime_paths, apply_runtime_pledge};

/// Compatibility shim: old callers get phase-1 unveil only.
#[cfg(target_os = "openbsd")]
pub fn apply_initial_restrictions(config_path: &std::path::Path) -> Result<(), String> {
    openbsd_impl::unveil_config(config_path)
}

#[cfg(target_os = "openbsd")]
pub fn apply_runtime_restrictions(mode: RuntimeMode, has_sqlite: bool) -> Result<(), String> {
    openbsd_impl::apply_runtime_pledge(mode, has_sqlite)
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
    #[allow(dead_code)]
    pub fn apply_tls_file_restrictions(_cert: &std::path::Path, _key: &std::path::Path) -> Result<(), String> { Ok(()) }
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
