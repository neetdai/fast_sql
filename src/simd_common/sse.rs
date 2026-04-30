use super::common::SimdTrait;
use std::arch::x86_64;

pub(crate) struct Sse;

impl SimdTrait for Sse {
    const LENGTH: usize = 16;

    fn find_consecutive_in_range(
        slice: &[u8],
        matches: (u8, u8),
        start_pos: usize,
    ) -> (usize, isize) {
        let mut end_pos = -1;
        let mut pos = start_pos;
        let len = slice.len();

        unsafe {
            let matches_range = {
                let lane_a = x86_64::_mm_set1_epi8(matches.0.cast_signed() - 1);
                let lane_b = x86_64::_mm_set1_epi8(matches.1.cast_signed() + 1);
                (lane_a, lane_b)
            };

            while pos + Self::LENGTH <= len {
                let ptr = slice.as_ptr().add(pos).cast();
                let ptr = x86_64::_mm_loadu_si128(ptr);

                let cmp_a = x86_64::_mm_cmpgt_epi8(ptr, matches_range.0);
                let cmp_b = x86_64::_mm_cmpgt_epi8(matches_range.1, ptr);
                let cmp = x86_64::_mm_and_si128(cmp_a, cmp_b);
                let mask = x86_64::_mm_movemask_epi8(cmp);

                if mask != (u16::MAX as i32) {
                    let trailing_ones = mask.trailing_ones();
                    pos += trailing_ones as usize;
                    break;
                } else {
                    pos += Self::LENGTH;
                }
            }
        }

        if start_pos < pos {
            end_pos = (pos - 1).cast_signed();
        }

        (start_pos, end_pos)
    }

    fn longest_consecutive_matching<const N: usize>(
        slice: &[u8],
        matches: [u8; N],
        start_pos: usize,
    ) -> (usize, isize) {
        let mut end_pos = -1;
        let mut pos = start_pos;
        let len = slice.len();

        unsafe {
            let match_lanes = matches.map(|m| x86_64::_mm_set1_epi8(m.cast_signed()));

            while pos + Self::LENGTH <= len {
                let ptr = slice.as_ptr().add(pos).cast();
                let ptr = x86_64::_mm_loadu_si128(ptr);

                let cmp = match_lanes
                    .iter()
                    .fold(x86_64::_mm_set1_epi8(0), |prev, &lane| {
                        let cmp = x86_64::_mm_cmpeq_epi8(ptr, lane);
                        x86_64::_mm_or_si128(prev, cmp)
                    });

                let mask = x86_64::_mm_movemask_epi8(cmp);

                if mask != (u16::MAX as i32) {
                    let trailing_ones = mask.trailing_ones();
                    pos += trailing_ones as usize;
                    break;
                } else {
                    pos += Self::LENGTH;
                }
            }
        }

        if start_pos < pos {
            end_pos = (pos - 1).cast_signed();
        }

        (start_pos, end_pos)
    }

    fn mixed_match<const N1: usize, const N2: usize>(
        slice: &[u8],
        match_range: [(u8, u8); N1],
        matches2: [u8; N2],
        start_pos: usize,
    ) -> (usize, isize) {
        let mut end_pos = -1;
        let mut pos = start_pos;
        let len = slice.len();

        unsafe {
            let matches_range = match_range.map(|(a, b)| {
                let lane_a = x86_64::_mm_set1_epi8(a.cast_signed() - 1);
                let lane_b = x86_64::_mm_set1_epi8(b.cast_signed() + 1);
                (lane_a, lane_b)
            });

            let match_lanes = matches2.map(|m| x86_64::_mm_set1_epi8(m.cast_signed()));

            while pos + Self::LENGTH <= len {
                // dbg!(&pos);
                let ptr = slice.as_ptr().add(pos).cast();
                let ptr = x86_64::_mm_loadu_si128(ptr);

                let range_cmp = matches_range.iter().fold(
                    x86_64::_mm_set1_epi8(0),
                    |prev, &(a_lane, b_lane)| {
                        let cmp_a = x86_64::_mm_cmpgt_epi8(ptr, a_lane);
                        let cmp_b = x86_64::_mm_cmpgt_epi8(b_lane, ptr);
                        let cmp = x86_64::_mm_and_si128(cmp_a, cmp_b);
                        x86_64::_mm_or_si128(prev, cmp)
                    },
                );

                let match_cmp = match_lanes
                    .iter()
                    .fold(x86_64::_mm_set1_epi8(0), |prev, &lane| {
                        let cmp = x86_64::_mm_cmpeq_epi8(ptr, lane);
                        x86_64::_mm_or_si128(prev, cmp)
                    });

                let cmp = x86_64::_mm_or_si128(range_cmp, match_cmp);
                let mask = x86_64::_mm_movemask_epi8(cmp);
                // dbg!(mask);

                if mask != (u16::MAX as i32) {
                    let trailing_ones = mask.trailing_ones();
                    pos += trailing_ones as usize;
                    break;
                } else {
                    pos += Self::LENGTH;
                }
            }
        }

        if start_pos < pos {
            end_pos = (pos - 1).cast_signed();
        }

        (start_pos, end_pos)
    }

    fn skip_until_match<const N: usize>(
        slice: &[u8],
        matches: [u8; N],
        start_pos: usize,
    ) -> (usize, isize) {
        if start_pos >= slice.len() {
            return (start_pos, -1);
        }
        let mut pos = start_pos;
        let len = slice.len();

        unsafe {
            let lanes = matches.map(|m| x86_64::_mm_set1_epi8(m.cast_signed()));

            while pos + Self::LENGTH <= len {
                let ptr = slice.as_ptr().add(pos).cast();
                let chunk = x86_64::_mm_loadu_si128(ptr);

                let mut all_mask: u32 = 0;
                for lane in lanes.iter().take(N) {
                    let eq = x86_64::_mm_cmpeq_epi8(chunk, *lane);
                    all_mask |= x86_64::_mm_movemask_epi8(eq) as u32;
                }

                if all_mask != 0 {
                    let offset = all_mask.trailing_zeros() as usize;
                    return (start_pos, (pos + offset) as isize);
                }
                pos += Self::LENGTH;
            }
        }

        // scalar tail
        while pos < len {
            if matches.contains(&slice[pos]) {
                return (start_pos, pos as isize);
            }
            pos += 1;
        }
        (start_pos, -1)
    }

    fn skip_until_sequence<const N: usize>(
        slice: &[u8],
        sequence: [u8; N],
        start_pos: usize,
    ) -> (usize, isize) {
        if start_pos + N > slice.len() {
            return (start_pos, -1);
        }
        let mut pos = start_pos;
        let len = slice.len();

        unsafe {
            let lanes = sequence.map(|m| x86_64::_mm_set1_epi8(m.cast_signed()));

            while pos + Self::LENGTH <= len {
                let ptr = slice.as_ptr().add(pos).cast();
                let chunk = x86_64::_mm_loadu_si128(ptr);

                let mut mask: u32 = 0xFFFF;
                for (i, lane) in lanes.iter().enumerate().take(N) {
                    let eq = x86_64::_mm_cmpeq_epi8(chunk, *lane);
                    mask &= (x86_64::_mm_movemask_epi8(eq) as u32) >> i;
                }

                if mask != 0 {
                    let offset = mask.trailing_zeros() as usize;
                    return (start_pos, (pos + offset) as isize);
                }
                pos += Self::LENGTH - (N - 1);
            }
        }

        // scalar tail
        let end = len.saturating_sub(N - 1);
        while pos < end {
            if slice[pos..pos + N] == sequence[..] {
                return (start_pos, pos as isize);
            }
            pos += 1;
        }
        (start_pos, -1)
    }
}

#[cfg(test)]
mod test {
    use super::Sse;
    use crate::simd_common::common::SimdTrait;

    #[test]
    fn sse_test1() {
        let slice = b"1234567890qwertyuiopasdfghjklzxcvbnm";
        let (start, end) = Sse::find_consecutive_in_range(slice, (b'0', b'9'), 0);
        assert_eq!(start, 0);
        assert_eq!(end, 9);

        let slice = b"12345678901234567890123456789012334567890";
        let (start, end) = Sse::find_consecutive_in_range(slice, (b'0', b'9'), 0);
        assert_eq!(start, 0);
        assert_eq!(end, 31);

        let slice = b"bqwertyuiopasdfghjklzxcvbnm";
        let (start, end) = Sse::find_consecutive_in_range(slice, (b'0', b'9'), 0);
        assert_eq!(start, 0);
        assert_eq!(end, -1);

        let slice = b"b023q2w142e245rtyuiopasdfghjklzxcvbnm";
        let (start, end) = Sse::find_consecutive_in_range(slice, (b'0', b'9'), 0);
        assert_eq!(start, 0);
        assert_eq!(end, -1);
    }

    #[test]
    fn sse_test2() {
        let slice = b"qwertyuiopasdfghjklzxcvbnm1234567890";
        let (start, end) = Sse::longest_consecutive_matching(slice, [b'q', b'w', b'e'], 0);
        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let slice = b"aaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbaaaaaaaaaaaaaaaaa";
        let (start, end) = Sse::longest_consecutive_matching(slice, [b'a', b'b', b'c'], 0);
        assert_eq!(start, 0);
        assert_eq!(end, 47);

        let slice = b"qwretyuiopasdfghjklzxcvbnm1234567890";
        let (start, end) = Sse::longest_consecutive_matching(slice, [b'q', b'w', b'e'], 0);
        assert_eq!(start, 0);
        assert_eq!(end, 1);
    }

    #[test]
    fn sse_test3() {
        let slice = b"1234567890qwertyuiopasdfghjklzxcvbnmQWERTYUIOIPASDFGHJKLZXCVBNM_";
        let (start, end) =
            Sse::mixed_match(slice, [(b'0', b'9'), (b'a', b'z'), (b'A', b'Z')], [b'_'], 0);
        assert_eq!(start, 0);
        assert_eq!(end, 63);
    }

    #[test]
    fn test_skip_until_match() {
        let slice = b"hello'world";
        let (start, end) = Sse::skip_until_match(slice, [b'\''], 0);
        assert_eq!(start, 0);
        assert_eq!(end, 5);

        let slice = b"no match";
        let (start, end) = Sse::skip_until_match(slice, [b'\''], 0);
        assert_eq!(end, -1);

        let slice = b"a\tb\nc\rd";
        let (start, end) = Sse::skip_until_match(slice, [b'\n', b'\r', b'\t'], 0);
        assert_eq!(start, 0);
        assert_eq!(end, 1); // \t is first

        // start at later position
        let slice = b"abc'def'ghi";
        let (start, end) = Sse::skip_until_match(slice, [b'\''], 4);
        assert_eq!(end, 7);
    }

    #[test]
    fn test_skip_until_sequence() {
        let slice = b"before */ after";
        let (start, end) = Sse::skip_until_sequence(slice, [b'*', b'/'], 0);
        assert_eq!(start, 0);
        assert_eq!(end, 7);

        let slice = b"no match";
        let (start, end) = Sse::skip_until_sequence(slice, [b'*', b'/'], 0);
        assert_eq!(end, -1);

        // 跨 SSE 16 字节边界
        let mut data = vec![b'a'; 15];
        data.extend_from_slice(b"*/");
        let (start, end) = Sse::skip_until_sequence(&data, [b'*', b'/'], 0);
        assert_eq!(end, 15);
    }
}
