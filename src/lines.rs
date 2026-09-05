use std::io::{BufRead, ErrorKind, Result};

#[derive(Debug)]
/// `std::io::Lines` re-implementation to handle invalid utf-8 characters
pub struct LossyLines<B> {
    buf: B,
}

impl<B: BufRead> LossyLines<B> {
    pub fn new(buf: B) -> Self {
        Self { buf }
    }
}

impl<B: BufRead> Iterator for LossyLines<B> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Result<String>> {
        let mut buf = Vec::new();
        match read_until(&mut self.buf, b'\n', &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf.last().is_some_and(|b| *b == b'\n') {
                    buf.pop();
                    if buf.last().is_some_and(|b| *b == b'\r') {
                        buf.pop();
                    }
                }

                let s = String::from_utf8_lossy(&buf).into_owned();
                Some(Ok(s))
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
