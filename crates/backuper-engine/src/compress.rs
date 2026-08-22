use backuper_core::error::BackuperError;
use std::path::Path;

pub async fn compress_file(src: &Path, dst: &Path) -> Result<(), BackuperError> {
    let src = src.to_path_buf();
    let dst = dst.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let input = std::fs::File::open(&src)?;
        let output = std::fs::File::create(&dst)?;
        zstd::stream::copy_encode(input, output, 3)?;
        Ok::<_, BackuperError>(())
    })
    .await
    .map_err(|e| BackuperError::Source(e.to_string()))??;
    Ok(())
}
