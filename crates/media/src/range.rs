/// A single byte range could not be selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RangeError {
    #[error("invalid byte range")]
    Invalid,
    #[error("byte range is unsatisfiable")]
    Unsatisfiable,
    #[error("multiple byte ranges are not supported")]
    MultipleRanges,
}

/// Select one inclusive byte interval, following HTTP range semantics.
///
/// Transport may ignore `MultipleRanges` and serve the complete representation.
/// Callers must emit `Content-Range: bytes */{size}` for a 416 response.
pub fn select_range(header: &str, size: u64) -> Result<(u64, u64), RangeError> {
    let (unit, value) = header.trim().split_once('=').ok_or(RangeError::Invalid)?;
    if !unit.eq_ignore_ascii_case("bytes") {
        return Err(RangeError::Invalid);
    }
    if value.contains(',') {
        return Err(RangeError::MultipleRanges);
    }
    let (start, end) = value.trim().split_once('-').ok_or(RangeError::Invalid)?;
    let number = |text: &str| {
        if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(RangeError::Invalid);
        }
        text.parse::<u64>().map_err(|_| RangeError::Invalid)
    };
    if size == 0 {
        return Err(RangeError::Unsatisfiable);
    }
    if start.is_empty() {
        let suffix = number(end)?;
        if suffix == 0 {
            return Err(RangeError::Unsatisfiable);
        }
        return Ok((size.saturating_sub(suffix), size - 1));
    }
    let start = number(start)?;
    let end = if end.is_empty() {
        size - 1
    } else {
        number(end)?
    };
    if start >= size || start > end {
        return Err(RangeError::Unsatisfiable);
    }
    Ok((start, end.min(size - 1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_open_suffix_and_clamped_ranges() {
        assert_eq!(select_range("bytes=0-0", 100), Ok((0, 0)));
        assert_eq!(select_range("bytes=10-19", 100), Ok((10, 19)));
        assert_eq!(select_range("bytes=90-", 100), Ok((90, 99)));
        assert_eq!(select_range("bytes=-10", 100), Ok((90, 99)));
        assert_eq!(select_range("bytes=-1000", 100), Ok((0, 99)));
        assert_eq!(select_range("bytes=80-200", 100), Ok((80, 99)));
        assert_eq!(select_range("BYTES=0-1", 100), Ok((0, 1)));
    }

    #[test]
    fn rejects_unsatisfiable_empty_and_reversed_ranges() {
        for (header, size) in [
            ("bytes=0-0", 0),
            ("bytes=100-", 100),
            ("bytes=10-9", 100),
            ("bytes=-0", 100),
        ] {
            assert_eq!(select_range(header, size), Err(RangeError::Unsatisfiable));
        }
    }

    #[test]
    fn rejects_malformed_and_overflowing_ranges() {
        for header in [
            "",
            "seconds=0-10",
            "bytes=",
            "bytes=-",
            "bytes=+1-2",
            "bytes=0--2",
            "bytes=1 -2",
            "bytes=0-18446744073709551616",
            "bytes=18446744073709551616-",
        ] {
            assert_eq!(
                select_range(header, 100),
                Err(RangeError::Invalid),
                "{header}"
            );
        }
        assert_eq!(
            select_range("bytes=0-1,4-5", 100),
            Err(RangeError::MultipleRanges)
        );
    }

    #[test]
    fn maximum_size_never_overflows() {
        assert_eq!(
            select_range("bytes=0-18446744073709551615", u64::MAX),
            Ok((0, u64::MAX - 1))
        );
        assert_eq!(
            select_range("bytes=-18446744073709551615", u64::MAX),
            Ok((0, u64::MAX - 1))
        );
        assert_eq!(
            select_range("bytes=18446744073709551614-", u64::MAX),
            Ok((u64::MAX - 1, u64::MAX - 1))
        );
    }
}
