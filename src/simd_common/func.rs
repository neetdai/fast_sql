use super::{SimdTrait, Sse};

pub(crate) fn find_consecutive_in_range(
    slice: &[u8],
    matches: (u8, u8),
    start_pos: usize,
) -> (usize, isize) {
    cfg_select! {
        target_feature = "sse2" => {
            find_consecutive_in_range_basic_sse(slice, matches, start_pos)
        }
        _ => {
            find_consecutive_in_range_basic(slice, matches, start_pos)
        }
    }
}

pub(crate) fn longest_consecutive_matching<const N: usize>(
    slice: &[u8],
    matches: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    cfg_select! {
        target_feature = "sse2" => {
            longest_consecutive_matching_basic_sse(slice, matches, start_pos)
        }
        _ => {
            longest_consecutive_matching_basic(slice, matches, start_pos)
        }
    }
}

pub(crate) fn mixed_match<const N1: usize, const N2: usize>(
    slice: &[u8],
    match_range: [(u8, u8); N1],
    matches2: [u8; N2],
    start_pos: usize,
) -> (usize, isize) {
    cfg_select! {
        target_feature = "sse2" => {
            mixed_match_sse(slice, match_range, matches2, start_pos)
        }
        _ => {
            mixed_match_basic(slice, match_range, matches2, start_pos)
        }
    }
}

pub(crate) fn skip_until_match<const N: usize>(
    slice: &[u8],
    matches: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    cfg_select! {
        target_feature = "sse2" => {
            skip_until_match_sse(slice, matches, start_pos)
        }
        _ => {
            skip_until_match_basic(slice, matches, start_pos)
        }
    }
}

pub(crate) fn skip_until_sequence<const N: usize>(
    slice: &[u8],
    sequence: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    cfg_select! {
        target_feature = "sse2" => {
            skip_until_sequence_sse(slice, sequence, start_pos)
        }
        _ => {
            skip_until_sequence_basic(slice, sequence, start_pos)
        }
    }
}

fn find_consecutive_in_range_basic(
    slice: &[u8],
    matches: (u8, u8),
    start_pos: usize,
) -> (usize, isize) {
    let len = slice.len();
    let mut end_pos = -1;

    for (index, c) in slice.iter().enumerate().take(len).skip(start_pos) {
        if *c >= matches.0 && *c <= matches.1 {
            end_pos = index.cast_signed();
        } else {
            break;
        }
    }
    (start_pos, end_pos)
}

#[cfg(target_feature = "sse2")]
fn find_consecutive_in_range_basic_sse(
    slice: &[u8],
    matches: (u8, u8),
    start_pos: usize,
) -> (usize, isize) {
    if start_pos >= slice.len() {
        return (start_pos, -1);
    }
    let mut end_pos = -1;
    let len = slice.len();

    let tmp_pos = if end_pos == -1 {
        start_pos
    } else {
        end_pos as usize
    };
    if len >= tmp_pos + Sse::LENGTH {
        (_, end_pos) = Sse::find_consecutive_in_range(slice, matches, tmp_pos);
    }

    let tmp_pos = if end_pos == -1 {
        start_pos
    } else {
        end_pos as usize
    };
    (_, end_pos) = find_consecutive_in_range_basic(slice, matches, tmp_pos);
    (start_pos, end_pos)
}

fn longest_consecutive_matching_basic<const N: usize>(
    slice: &[u8],
    matches: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    let len = slice.len();
    let mut end_pos = -1;

    for (index, c) in slice.iter().enumerate().take(len).skip(start_pos) {
        // dbg!(c);
        if matches.contains(c) {
            end_pos = index.cast_signed();
        } else {
            break;
        }
    }
    (start_pos, end_pos)
}

#[cfg(target_feature = "sse2")]
fn longest_consecutive_matching_basic_sse<const N: usize>(
    slice: &[u8],
    matches: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    if start_pos >= slice.len() {
        return (start_pos, -1);
    }
    let mut end_pos = -1;
    let len = slice.len();

    let tmp_pos = if end_pos == -1 {
        start_pos
    } else {
        end_pos as usize
    };
    if len >= tmp_pos + Sse::LENGTH {
        (_, end_pos) = Sse::longest_consecutive_matching(slice, matches, tmp_pos);
    }

    let tmp_pos = if end_pos == -1 {
        start_pos
    } else {
        end_pos as usize
    };
    (_, end_pos) = longest_consecutive_matching_basic(slice, matches, tmp_pos);
    (start_pos, end_pos)
}

#[cfg(target_feature = "sse2")]
fn mixed_match_sse<const N1: usize, const N2: usize>(
    slice: &[u8],
    match_range: [(u8, u8); N1],
    matches2: [u8; N2],
    start_pos: usize,
) -> (usize, isize) {
    if start_pos >= slice.len() {
        return (start_pos, -1);
    }
    let mut end_pos = -1;
    let len = slice.len();

    if is_x86_feature_detected!("sse2") {
        let tmp_pos = if end_pos == -1 {
            start_pos
        } else {
            end_pos as usize
        };
        if len >= tmp_pos + Sse::LENGTH {
            (_, end_pos) = Sse::mixed_match(slice, match_range, matches2, tmp_pos);
        }
    }

    let tmp_pos = if end_pos == -1 {
        start_pos
    } else {
        end_pos as usize
    };
    (_, end_pos) = mixed_match_basic(slice, match_range, matches2, tmp_pos);
    (start_pos, end_pos)
}

fn mixed_match_basic<const N1: usize, const N2: usize>(
    slice: &[u8],
    match_range: [(u8, u8); N1],
    matches2: [u8; N2],
    start_pos: usize,
) -> (usize, isize) {
    let mut end_pos = -1;
    let len = slice.len();
    // dbg!(start_pos);

    for (index, c) in slice.iter().enumerate().take(len).skip(start_pos) {
        // dbg!(c);
        let in_range = match_range.iter().any(|(a, b)| *c >= *a && *c <= *b);
        if in_range || matches2.contains(c) {
            end_pos = index.cast_signed();
        } else {
            break;
        }
    }
    (start_pos, end_pos)
}

fn skip_until_match_basic<const N: usize>(
    slice: &[u8],
    matches: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    if start_pos >= slice.len() {
        return (start_pos, -1);
    }
    for (i, item) in slice.iter().enumerate().skip(start_pos) {
        if matches.contains(item) {
            return (start_pos, i as isize);
        }
    }
    (start_pos, -1)
}

fn skip_until_sequence_basic<const N: usize>(
    slice: &[u8],
    sequence: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    if start_pos + N > slice.len() {
        return (start_pos, -1);
    }
    let end = slice.len().saturating_sub(N - 1);
    for i in start_pos..end {
        if slice[i..i + N] == sequence[..] {
            return (start_pos, i as isize);
        }
    }
    (start_pos, -1)
}

#[cfg(target_feature = "sse2")]
fn skip_until_match_sse<const N: usize>(
    slice: &[u8],
    matches: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    if start_pos >= slice.len() {
        return (start_pos, -1);
    }
    Sse::skip_until_match(slice, matches, start_pos)
}

#[cfg(target_feature = "sse2")]
fn skip_until_sequence_sse<const N: usize>(
    slice: &[u8],
    sequence: [u8; N],
    start_pos: usize,
) -> (usize, isize) {
    if start_pos >= slice.len() {
        return (start_pos, -1);
    }
    Sse::skip_until_sequence(slice, sequence, start_pos)
}

#[cfg(test)]
mod test {
    use super::{
        find_consecutive_in_range_basic, longest_consecutive_matching_basic, mixed_match_basic,
    };

    #[test]
    fn test_find_consecutive_in_range_basic() {
        let slice = b"123456789";
        let (start, end) = find_consecutive_in_range_basic(slice, (b'0', b'9'), 0);
        assert_eq!(start, 0);
        assert_eq!(end, 8);
    }

    #[test]
    fn test_longest_consecutive_matching_basic() {
        let slice = b"abcde12345fghij67890";
        let matches = [b'a', b'b', b'c', b'd', b'e'];
        let (start, end) = longest_consecutive_matching_basic(slice, matches, 0);
        assert_eq!(start, 0);
        assert_eq!(end, 4);

        let slice = b"                 
                ";
        let matches = [b' ', b'\t', b'\n', b'\r'];
        let (start, end) = longest_consecutive_matching_basic(slice, matches, 0);
        assert_eq!(start, 0);
        assert_eq!(end, 33);
    }

    #[test]
    fn test_mixed_match_basic() {
        let slice = b"abcde12345fghij67890";
        let match_range = [(b'a', b'e'), (b'0', b'9')];
        let matches2 = [b'f', b'g', b'h'];
        let (start, end) = mixed_match_basic(slice, match_range, matches2, 0);
        assert_eq!(start, 0);
        assert_eq!(end, 12);

        let slice = b"abcde12345fghij67890";
        let match_range = [(b'a', b'e'), (b'0', b'9')];
        let matches2 = [b'f', b'g', b'h'];
        let (start, end) = mixed_match_basic(slice, match_range, matches2, 10);
        assert_eq!(start, 10);
        assert_eq!(end, 12);
    }
}
