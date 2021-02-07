//! Const evaluated utf8 to utf16 conversion functions.
//!
//! # Use
//!
//! ```
//! const HELLO_WORLD_UTF16: const_utf16::Utf16Buffer = const_utf16::encode_utf16("Hello, world!");
//! ```
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// /// TODO
// pub const fn utf16_len(s: &str) -> usize {
//     s.len() // Obviously incorrect.
// }

// macro_rules! encode {
//     ($s:literal) => {{
//         const LEN: usize = $crate::utf16_len($s);

//         const fn utf16_encode() -> [u16; LEN] {
//             let mut buffer = [0; LEN];
//             let mut idx = 0;
//             // Obviously incorrect.
//             while idx < LEN {
//                 buffer[idx] = $s.as_bytes()[idx] as u16;
//                 idx += 1;
//             }
//             buffer
//         }
//         const BUFFER: [u16; LEN] = utf16_encode();
//         &BUFFER
//     }};
// }

/// Encode a &str as a utf16 buffer
///
/// # Panics
///
/// This function panics if if `string` encodes to a utf16 buffer bigger than [`BUFFER_SIZE`].
pub const fn encode_utf16(string: &str) -> Utf16Buffer {
    encode(string, false)
}

/// Encode a &str as a utf16 buffer with a terminating null byte
///
/// # Panics
///
/// This function panics if called with a string that contains any null bytes or
/// if `string` encodes to a utf16 buffer bigger than [`BUFFER_SIZE`].
pub const fn encode_utf16_with_terminating_null(string: &str) -> Utf16Buffer {
    let result = encode(string, true);
    result.push(0)
}

const fn encode(string: &str, ensure_no_nulls: bool) -> Utf16Buffer {
    let mut result = [0; BUFFER_SIZE];
    let bytes = string.as_bytes();
    let mut utf16_offset = 0;
    let mut iterator = CodePointIterator::new(bytes);
    while let Some((next, mut code)) = iterator.next() {
        iterator = next;
        if code == 0 && ensure_no_nulls {
            #[allow(unconditional_panic)]
            let _ = ["Found a null byte in string which should have no null bytes"][usize::MAX];
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

    Utf16Buffer {
        buffer: result,
        len: utf16_offset,
    }
}

struct CodePointIterator<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> CodePointIterator<'a> {
    const fn new(buffer: &'a [u8]) -> Self {
        Self::new_with_offset(buffer, 0)
    }

    const fn new_with_offset(buffer: &'a [u8], offset: usize) -> Self {
        Self { buffer, offset }
    }

    const fn next(self) -> Option<(Self, u32)> {
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

/// The size of the Utf16Buffer
pub const BUFFER_SIZE: usize = 1024;

/// A buffer of Utf16 encode bytes
pub struct Utf16Buffer {
    buffer: [u16; BUFFER_SIZE],
    len: usize,
}

impl Utf16Buffer {
    /// Get the buffer as a slice
    pub fn as_slice(&self) -> &[u16] {
        &self.buffer[0..self.len]
    }

    /// Push an item on to the buffer.
    ///
    /// Note: this takes `mut self` as `&mut self` is not allowed in const contexts
    const fn push(mut self, element: u16) -> Self {
        self.buffer[self.len] = element;
        self.len += 1;
        self
    }
}

impl std::ops::Deref for Utf16Buffer {
    type Target = [u16];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl std::fmt::Debug for Utf16Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_slice())
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
#[inline]
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
        const TEXT: &str = "Hello \0\0ä日本 語";
        let result = TEXT.encode_utf16().collect::<Vec<_>>();
        const RESULT: Utf16Buffer = encode_utf16(TEXT);

        assert_eq!(&RESULT.as_slice(), &result);
    }

    #[test]
    fn encode_utf16_with_null_byte_works() {
        const TEXT: &str = "Hello ä日本 語";
        let result = TEXT.encode_utf16().collect::<Vec<_>>();
        const RESULT: Utf16Buffer = encode_utf16_with_terminating_null(TEXT);

        assert_eq!(&RESULT.as_slice()[0..result.len()], &result);
        assert_eq!(&RESULT.as_slice()[result.len()..result.len() + 1], &[0]);
    }
}
