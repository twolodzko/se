use std::io::{BufRead, ErrorKind, Result};

/// `std::io::Lines` re-implemented to return `Vec<u8>` instead of `String`
#[derive(Debug)]
pub struct ByteLines<B> {
    buf: B,
}

impl<B: BufRead> ByteLines<B> {
    pub fn new(buf: B) -> Self {
        Self { buf }
    }
}

impl<B: BufRead> Iterator for ByteLines<B> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match read_until(&mut self.buf, b'\n', &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if let Some(b'\n') = buf.last() {
                    buf.pop();
                    if let Some(b'\r') = buf.last() {
                        buf.pop();
                    }
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

// taken from std::io with modifications
fn read_until<R: BufRead + ?Sized>(r: &mut R, delim: u8, buf: &mut Vec<u8>) -> Result<usize> {
    let mut read = 0;
    loop {
        let (done, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            match memchr::memchr(delim, available) {
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
        if done || used == 0 {
            return Ok(read);
        }
    }
}
