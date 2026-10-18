use crate::{Error, Result};
use serde::Serialize;
use std::io::{self, Write};
struct Counter<'a>(&'a mut usize);
impl Write for Counter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > *self.0 {
            return Err(io::Error::other("catalog budget"));
        }
        *self.0 -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(super) fn consume(value: &impl Serialize, remaining: &mut usize) -> Result<()> {
    serde_json::to_writer(Counter(remaining), value).map_err(|_| Error("api_catalog_too_large"))
}
