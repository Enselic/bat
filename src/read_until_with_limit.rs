//! This file is based on code from
//! <https://github.com/rust-lang/rust/blob/1.57.0/library/std/src/io/mod.rs>
//! licensed under <https://github.com/rust-lang/rust/blob/1.57.0/LICENSE-MIT>
//! which says:
//!
//! ```txt
//! Permission is hereby granted, free of charge, to any
//! person obtaining a copy of this software and associated
//! documentation files (the "Software"), to deal in the
//! Software without restriction, including without
//! limitation the rights to use, copy, modify, merge,
//! publish, distribute, sublicense, and/or sell copies of
//! the Software, and to permit persons to whom the Software
//! is furnished to do so, subject to the following
//! conditions:
//!
//! The above copyright notice and this permission notice
//! shall be included in all copies or substantial portions
//! of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
//! ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
//! TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
//! PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
//! SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
//! CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
//! OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
//! IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
//! DEALINGS IN THE SOFTWARE.
//! ```
//!
//! To see the changes that has been made to the file you can run `git diff
//! 766cfe1 -- src/read_until_with_limit.rs`.

use std::io::{BufRead, Error, ErrorKind, Result};

pub(crate) fn read_until_with_limit<R: BufRead + ?Sized>(
    r: &mut R,
    delim: u8,
    buf: &mut Vec<u8>,
    limit: usize,
) -> Result<usize> {
    let mut read = 0;
    loop {
        let (done, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            match available.iter().position(|b| *b == delim) {
                Some(i) => {
                    buf.extend_from_slice(&available[..=i]);
                    (true, i + 1)
                }
                None => {
                    buf.extend_from_slice(available);
                    (false, available.len())
                }
            }
        };
        r.consume(used);
        read += used;
        if read > limit {
            let bat_error = crate::error::Error::Msg(format!(
                "Lines longer than {} bytes are not supported. Try auto-formatting your file first.",
                limit
            ));
            return Err(Error::new(ErrorKind::Other, bat_error));
        }
        if done || used == 0 {
            return Ok(read);
        }
    }
}
