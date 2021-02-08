//! Const evaluated utf8 to utf16 conversion functions.
//!
//! # Use
//!
//! ```
//! # #[macro_use]
//! # extern crate const_utf16;
//! # fn main() {}
//! const HELLO_WORLD_UTF16: &[u16] = const_utf16::encode!("Hello, world!");
//! ```
#![deny(missing_docs)]

/// Encode a &str as a utf16 buffer.
#[macro_export]
macro_rules! encode {
    ($s:expr) => {{
        $crate::encode!($s, non_null_terminated)
    }};
    ($s:expr, $null_terminated:ident) => {{
        const __STRING: &'static str = $s;
    const __EXTRA_BYTE: usize = $crate::encode!(@@ $null_terminated);
        const __STRING_LEN: usize = __STRING.len() + __EXTRA_BYTE;
        const __BUFFER_AND_LEN: (&[u16; __STRING_LEN], usize) = {
            let mut result = [0; __STRING_LEN];
            let mut utf16_offset = 0;

            let mut iterator = $crate::CodePointIterator::new(__STRING.as_bytes());
            while let Some((next, mut code)) = iterator.next() {
                iterator = next;
                if code == 0 && __EXTRA_BYTE == 1 {
                    #[allow(unconditional_panic)]
                    let _ =
                        ["Found a null byte in string which should have no null bytes"][usize::MAX];
                }
                if (code & 0xFFFF) == code {
                    result[utf16_offset] = code as u16;
                    utf16_offset += 1;
                } else {
                    // Supplementary planes break into surrogates.
                    code -= 0x1_0000;
                    result[utf16_offset] = 0xD800 | ((code >> 10) as u16);
                    result[utf16_offset + 1] = 0xDC00 | ((code as u16) & 0x3FF);
                    utf16_offset += 2;
                }
            }
            (&{ result }, utf16_offset + __EXTRA_BYTE)
        };
        const __OUT: &[u16; __BUFFER_AND_LEN.1] = unsafe {
            ::core::mem::transmute::<
                &'static &[u16; __STRING_LEN],
                &'static &[u16; __BUFFER_AND_LEN.1],
            >(&__BUFFER_AND_LEN.0)
        };
        __OUT
    }};
    (@@ null_terminated) => {
        1
    };
    (@@ non_null_terminated) => {
        0
    };
}

/// Encode a &str as a utf16 buffer with a terminating null byte
///
/// # Panics
///
/// This function panics if called with a string that contains any null bytes.
#[macro_export]
macro_rules! encode_null_terminated {
    ($s:expr) => {{
        $crate::encode!($s, null_terminated)
    }};
}

#[doc(hidden)]
pub struct CodePointIterator<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> CodePointIterator<'a> {
    #[doc(hidden)]
    pub const fn new(buffer: &'a [u8]) -> Self {
        Self::new_with_offset(buffer, 0)
    }

    #[doc(hidden)]
    pub const fn new_with_offset(buffer: &'a [u8], offset: usize) -> Self {
        Self { buffer, offset }
    }

    #[doc(hidden)]
    pub const fn next(self) -> Option<(Self, u32)> {
        if let Some((codepont, num_utf8_bytes)) = next_code_point(self.buffer, self.offset) {
            Some((
                Self::new_with_offset(self.buffer, self.offset + num_utf8_bytes),
                codepont,
            ))
        } else {
            None
        }
    }
}

/// Largely adapted from [Rust core](https://github.com/rust-lang/rust/blob/7e2032390cf34f3ffa726b7bd890141e2684ba63/library/core/src/str/validations.rs#L40-L68).
const fn next_code_point(bytes: &[u8], start: usize) -> Option<(u32, usize)> {
    if bytes.len() == start {
        return None;
    }
    let mut num_bytes = 1;
    let x = bytes[start + 0];
    if x < 128 {
        return Some((x as u32, num_bytes));
    }
    // Multibyte case follows
    // Decode from a byte combination out of: [[[x y] z] w]
    // NOTE: Performance is sensitive to the exact formulation here
    let init = utf8_first_byte(x, 2);
    let y = unwrap_or_0(bytes, start + 1);
    if y != 0 {
        num_bytes += 1;
    }
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        // [[x y z] w] case
        // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
        let z = unwrap_or_0(bytes, start + 2);
        if z != 0 {
            num_bytes += 1;
        }
        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            // [x y z w] case
            // use only the lower 3 bits of `init`
            let w = unwrap_or_0(bytes, start + 3);
            if w != 0 {
                num_bytes += 1;
            }
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }

    Some((ch, num_bytes))
}

/// Returns the initial codepoint accumulator for the first byte.
/// The first byte is special, only want bottom 5 bits for width 2, 4 bits
/// for width 3, and 3 bits for width 4.
const fn utf8_first_byte(byte: u8, width: u32) -> u32 {
    (byte & (0x7F >> width)) as u32
}

const fn unwrap_or_0(slice: &[u8], index: usize) -> u8 {
    if slice.len() > index {
        slice[index]
    } else {
        0
    }
}

const fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & CONT_MASK) as u32
}

/// Mask of the value bits of a continuation byte.
const CONT_MASK: u8 = 0b0011_1111;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encode_utf16_works() {
        const TEXT: &str = "Hello \0ä日本 語";
        let expected = TEXT.encode_utf16().collect::<Vec<_>>();
        const RESULT: &[u16] = encode!(TEXT);

        assert_eq!(RESULT, &expected[..]);
    }

    #[test]
    fn encode_utf16_with_null_byte_works() {
        const TEXT: &str = "Hello ä日本 語";
        let result = TEXT.encode_utf16().collect::<Vec<_>>();
        const RESULT: &[u16] = encode_null_terminated!(TEXT);

        assert_eq!(&RESULT[0..result.len()], &result[..]);
        assert_eq!(&RESULT[result.len()..result.len() + 1], &[0]);
    }
}
