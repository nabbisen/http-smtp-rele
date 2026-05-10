//! Header injection detection and sanitization.
//!
//! Detects CR and LF characters in header-bound fields.
//! Policy: reject, never silently rewrite.
//!
//! RFC 050 — implementation pending.

/// Returns `true` if the string contains CR (`\r`) or LF (`\n`).
pub fn contains_header_injection(s: &str) -> bool {
    s.bytes().any(|b| b == b'\r' || b == b'\n')
}

/// Returns `true` if the string contains control characters other than tab.
pub fn contains_control_chars(s: &str) -> bool {
    s.bytes().any(|b| b < 0x20 && b != 0x09)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_string_passes() {
        assert!(!contains_header_injection("Hello World"));
        assert!(!contains_control_chars("Hello\tWorld"));
    }

    #[test]
    fn cr_detected() {
        assert!(contains_header_injection("Hello\rWorld"));
    }

    #[test]
    fn lf_detected() {
        assert!(contains_header_injection("Hello\nWorld"));
    }

    #[test]
    fn crlf_detected() {
        assert!(contains_header_injection("Hello\r\nWorld"));
    }

    #[test]
    fn null_byte_is_control_char() {
        assert!(contains_control_chars("Hello\x00World"));
    }
}

/// Rejects any string containing CR (`\r`) or LF (`\n`).
///
/// Returns a [`crate::error::AppError::Validation`] error naming the field.
/// This is the primary API used by the validation pipeline (RFC 051).
pub fn reject_header_crlf(field: &str, value: &str) -> Result<(), crate::error::AppError> {
    if contains_header_injection(value) {
        tracing::warn!(
            field = field,
            event = "header_injection_attempt",
            "CR/LF detected in header-bound field"
        );
        return Err(crate::error::AppError::Validation(format!(
            "{field}: CR or LF characters are not allowed"
        )));
    }
    Ok(())
}
