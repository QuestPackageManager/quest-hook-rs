//! AOB (array-of-bytes) signature scanning, ported from beatsaber-hook's
//! `binary.cpp`.

use std::fmt;

/// A single byte in a parsed pattern: either a specific byte to match, or a
/// wildcard (`?`/`??`) that matches anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternByte {
    Exact(u8),
    Wildcard,
}

impl PatternByte {
    fn matches(self, byte: u8) -> bool {
        match self {
            Self::Exact(expected) => expected == byte,
            Self::Wildcard => true,
        }
    }
}

/// Why a pattern scan failed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindPatternError {
    /// A pattern token wasn't two hex digits, `?`, or `??`
    InvalidToken(String),
    /// The pattern did not match anywhere in the search space
    NotFound,
}

impl fmt::Display for FindPatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken(token) => write!(f, "invalid pattern token: {token:?}"),
            Self::NotFound => f.write_str("pattern not found"),
        }
    }
}

impl std::error::Error for FindPatternError {}

/// Parses a space-separated pattern string, e.g. `"48 8B ?? ?? 05"`
fn parse(pattern: &str) -> Result<Vec<PatternByte>, FindPatternError> {
    pattern
        .split_whitespace()
        .map(|token| match token {
            "?" | "??" => Ok(PatternByte::Wildcard),
            _ => u8::from_str_radix(token, 16)
                .map(PatternByte::Exact)
                .map_err(|_| FindPatternError::InvalidToken(token.to_owned())),
        })
        .collect()
}

/// Returns the starting offset of every occurrence of `pattern` in `haystack`
fn find_all(haystack: &[u8], pattern: &[PatternByte]) -> Vec<usize> {
    if pattern.is_empty() || haystack.len() < pattern.len() {
        return Vec::new();
    }

    haystack
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, window)| window.iter().zip(pattern).all(|(&b, p)| p.matches(b)))
        .map(|(i, _)| i)
        .collect()
}

/// Finds the first occurrence of `pattern` (e.g. `"48 8B ?? ?? 05"`, with
/// `?`/`??` tokens matching any byte) in `haystack`, returning its offset
pub fn find_pattern(haystack: &[u8], pattern: &str) -> Result<usize, FindPatternError> {
    let pattern = parse(pattern)?;
    find_all(haystack, &pattern)
        .into_iter()
        .next()
        .ok_or(FindPatternError::NotFound)
}

/// Like [`find_pattern`], but also notes (at debug level, if the `trace`
/// feature is enabled) when the pattern matches more than once - which
/// usually means it isn't specific enough to reliably identify a single
/// location. `label` identifies the pattern in that message.
pub fn find_unique_pattern(
    haystack: &[u8],
    pattern: &str,
    label: &str,
) -> Result<usize, FindPatternError> {
    let pattern = parse(pattern)?;
    let matches = find_all(haystack, &pattern);
    let &first = matches.first().ok_or(FindPatternError::NotFound)?;

    if matches.len() > 1 {
        debug!("multiple sig scan matches for \"{label}\"");
    }

    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_pattern_matches_an_exact_byte_sequence() {
        let haystack = [0x00, 0x11, 0x22, 0x33, 0x44];
        assert_eq!(find_pattern(&haystack, "22 33"), Ok(2));
    }

    #[test]
    fn find_pattern_treats_question_marks_as_wildcards() {
        let haystack = [0x00, 0x11, 0x22, 0x33, 0x44];
        assert_eq!(find_pattern(&haystack, "11 ? 33"), Ok(1));
        assert_eq!(find_pattern(&haystack, "11 ?? 33"), Ok(1));
    }

    #[test]
    fn find_pattern_reports_not_found() {
        let haystack = [0x00, 0x11, 0x22];
        assert_eq!(
            find_pattern(&haystack, "AA BB"),
            Err(FindPatternError::NotFound)
        );
    }

    #[test]
    fn find_pattern_rejects_an_invalid_token() {
        let haystack = [0x00, 0x11, 0x22];
        assert_eq!(
            find_pattern(&haystack, "ZZ"),
            Err(FindPatternError::InvalidToken("ZZ".to_owned()))
        );
    }

    #[test]
    fn find_unique_pattern_returns_the_first_match_even_when_there_are_several() {
        let haystack = [0xAA, 0x11, 0xAA, 0x11];
        assert_eq!(find_unique_pattern(&haystack, "AA 11", "test"), Ok(0));
    }
}
