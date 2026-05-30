use regex_lite::Regex;

use crate::error::EvalError;

/// Build a `Regex` from a pattern with optional FHIRPath 3.0 flags (`i`, `m`, `s`).
pub(super) fn build_regex(pattern: &str, flags: &str, context: &str) -> Result<Regex, EvalError> {
    let prefix: String = flags
        .chars()
        .filter(|c| matches!(c, 'i' | 'm' | 's'))
        .map(|c| format!("(?{c})"))
        .collect();
    let full = if prefix.is_empty() {
        pattern.to_owned()
    } else {
        format!("{prefix}{pattern}")
    };
    Regex::new(&full).map_err(|e| EvalError::Type(format!("{context}: invalid pattern: {e}")))
}

pub(super) fn regex_matches(text: &str, pattern: &str, flags: &str) -> Result<bool, EvalError> {
    build_regex(pattern, flags, "matches()").map(|re| re.is_match(text))
}

pub(super) fn regex_matches_full(
    text: &str,
    pattern: &str,
    flags: &str,
) -> Result<bool, EvalError> {
    let anchored = format!("^(?:{pattern})$");
    build_regex(&anchored, flags, "matchesFull()").map(|re| re.is_match(text))
}

pub(super) fn regex_replace_all(
    text: &str,
    pattern: &str,
    replacement: &str,
    flags: &str,
) -> Result<String, EvalError> {
    build_regex(pattern, flags, "replaceMatches()")
        .map(|re| re.replace_all(text, replacement).into_owned())
}
