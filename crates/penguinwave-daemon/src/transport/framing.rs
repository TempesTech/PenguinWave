//! NDJSON framing.

use penguinwave_proto::MAX_FRAME_BYTES;
use std::io::{self, BufRead, Write};

#[derive(Debug)]
pub enum FrameError {
    Eof,
    /// Line exceeded [`MAX_FRAME_BYTES`]; the connection must close.
    TooLong,
    Io(#[allow(dead_code)] io::Error),
}

/// Read one line, refusing anything over the frame cap.
///
/// The cap is enforced while reading rather than after, so an endless line
/// cannot exhaust memory before it is rejected.
pub fn read_frame<R: BufRead>(reader: &mut R, buf: &mut Vec<u8>) -> Result<(), FrameError> {
    buf.clear();
    loop {
        let available = match reader.fill_buf() {
            Ok([]) => {
                return if buf.is_empty() {
                    Err(FrameError::Eof)
                } else {
                    Ok(())
                }
            }
            Ok(b) => b,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(FrameError::Io(e)),
        };

        match available.iter().position(|b| *b == b'\n') {
            Some(i) => {
                if buf.len() + i > MAX_FRAME_BYTES {
                    return Err(FrameError::TooLong);
                }
                buf.extend_from_slice(&available[..i]);
                reader.consume(i + 1);
                return Ok(());
            }
            None => {
                let n = available.len();
                if buf.len() + n > MAX_FRAME_BYTES {
                    return Err(FrameError::TooLong);
                }
                buf.extend_from_slice(available);
                reader.consume(n);
            }
        }
    }
}

/// Write one compact JSON value plus `\n`.
pub fn write_frame<W: Write, T: serde::Serialize>(writer: &mut W, value: &T) -> io::Result<()> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    writer.write_all(&line)?;
    writer.flush()
}
