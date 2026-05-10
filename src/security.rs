//! Platform security hardening.
//!
//! On OpenBSD: applies `pledge(2)` and `unveil(2)` restrictions.
//! On other platforms: no-op with a log warning.

use std::path::Path;

/// Runtime operation mode, used to select the appropriate pledge promise set.
pub enum RuntimeMode {
    SmtpRelay,
}

/// Apply filesystem restrictions before reading the config file.
///
/// On OpenBSD: unveils only the config path with read permission, then
/// locks unveil. Applies initial pledge (`stdio rpath inet`).
///
/// On other platforms: logs a warning and returns `Ok(())`.
pub fn apply_initial_restrictions(config_path: &Path) -> Result<(), String> {
    platform::apply_initial_restrictions(config_path)
}

/// Apply runtime restrictions after all initialization is complete.
///
/// On OpenBSD: drops file-read permission, tightens pledge to
/// `stdio inet` (SMTP relay mode).
///
/// On other platforms: returns `Ok(())`.
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
        // Lock unveil — no further unveil calls allowed
        unveil::unveil("", "")
            .map_err(|e| format!("unveil lock failed: {}", e))?;
        // Initial pledge: read config, set up network
        pledge::pledge("stdio rpath inet", None)
            .map_err(|e| format!("initial pledge failed: {}", e))?;
        Ok(())
    }

    pub fn apply_runtime_restrictions(mode: RuntimeMode) -> Result<(), String> {
        let promises = match mode {
            RuntimeMode::SmtpRelay => "stdio inet",
        };
        // Drop rpath — config already loaded, no more file reads needed
        pledge::pledge(promises, None)
            .map_err(|e| format!("runtime pledge failed: {}", e))?;
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
        // On non-OpenBSD, this must be a no-op that succeeds.
        let result = apply_initial_restrictions(Path::new("/etc/http-smtp-rele.toml"));
        assert!(result.is_ok());
    }

    #[test]
    fn apply_runtime_restrictions_succeeds_on_non_openbsd() {
        let result = apply_runtime_restrictions(RuntimeMode::SmtpRelay);
        assert!(result.is_ok());
    }
}
