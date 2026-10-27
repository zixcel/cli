use crate::{Error, Result};
use tokio::io::{AsyncRead, AsyncReadExt};
pub(crate) async fn bounded(reader: impl AsyncRead + Unpin, maximum: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Error("provider_io_failed"))?;
    if bytes.len() > maximum {
        return Err(Error("provider_output_limit"));
    }
    Ok(bytes)
}
