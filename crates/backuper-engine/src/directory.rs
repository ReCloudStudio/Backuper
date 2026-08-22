use async_trait::async_trait;
use backuper_core::error::BackuperError;
use backuper_core::source::Source;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct DirectorySource {
    pub path: PathBuf,
}

impl DirectorySource {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl Source for DirectorySource {
    async fn backup(&self, target: &Path) -> Result<(), BackuperError> {
        let file = File::create(target)?;
        let encoder = zstd::stream::write::Encoder::new(file, 3)?;
        let mut builder = tar::Builder::new(encoder);
        builder.append_dir_all(".", &self.path)?;

        let encoder = builder.into_inner()?;
        encoder.finish()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn archives_directory() {
        let src = TempDir::new().unwrap();
        std::fs::write(src.path().join("hello.txt"), "world").unwrap();

        let dst = TempDir::new().unwrap();
        let archive = dst.path().join("backup.tar.zst");

        let source = DirectorySource::new(src.path().to_path_buf());
        source.backup(&archive).await.unwrap();

        let meta = std::fs::metadata(&archive).unwrap();
        assert!(meta.len() > 0);

        let file = std::fs::File::open(&archive).unwrap();
        let decoder = zstd::stream::read::Decoder::new(file).unwrap();
        let mut archive = tar::Archive::new(decoder);
        let entries: Vec<_> = archive
            .entries()
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.iter().any(|n| n.contains("hello.txt")));
    }
}
