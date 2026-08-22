use async_trait::async_trait;
use backuper_core::error::BackuperError;
use backuper_core::storage::StorageBackend;
use russh::client::{self, Handler};
use russh::keys::{PrivateKeyWithHashAlg, PublicKey, load_secret_key};
use russh_sftp::client::SftpSession;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

pub struct SshStorage {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub key: Option<PathBuf>,
    pub path: PathBuf,
}

impl SshStorage {
    pub fn new(
        host: String,
        port: u16,
        username: String,
        key: Option<PathBuf>,
        path: PathBuf,
    ) -> Self {
        Self {
            host,
            port,
            username,
            key,
            path,
        }
    }

    fn key_path(&self) -> Result<PathBuf, BackuperError> {
        if let Some(key) = self.key.as_ref() {
            return Ok(key.clone());
        }

        let home = std::env::var("HOME")
            .map_err(|_| BackuperError::Storage("无法获取 HOME 目录".to_string()))?;
        let ssh_dir = PathBuf::from(home).join(".ssh");
        for name in ["id_ed25519", "id_rsa"] {
            let candidate = ssh_dir.join(name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(BackuperError::Storage(
            "未配置 SSH 私钥且找不到默认密钥".to_string(),
        ))
    }

    fn remote_dest(&self, dest: &Path) -> String {
        format!("{}@{}:{}", self.username, self.host, dest.to_string_lossy())
    }

    fn ssh_common_options(&self, key: &Path) -> Vec<String> {
        vec![
            "-i".to_string(),
            key.display().to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
        ]
    }

    async fn ensure_remote_dir(&self, dest: &Path) -> Result<(), BackuperError> {
        let Some(parent) = dest.parent() else {
            return Ok(());
        };
        let parent = parent.to_string_lossy();
        if parent.is_empty() {
            return Ok(());
        }

        let key = self.key_path()?;
        let target = format!("{}@{}", self.username, self.host);
        let mut args = self.ssh_common_options(&key);
        args.push("-p".to_string());
        args.push(self.port.to_string());
        args.push(target);
        args.push(format!("mkdir -p {}", parent));

        let status = Command::new("ssh").args(&args).status().await?;
        if !status.success() {
            return Err(BackuperError::Storage("创建远程目录失败".to_string()));
        }
        Ok(())
    }

    async fn try_sftp(&self, local_path: &Path, dest: &Path) -> Result<(), BackuperError> {
        let key_path = self.key_path()?;
        let key_pair = load_secret_key(&key_path, None)
            .map_err(|e| BackuperError::Storage(format!("加载 SSH 私钥失败: {e}")))?;

        let config = Arc::new(client::Config::default());
        let mut handle = client::connect(config, (self.host.as_str(), self.port), SshClientHandler)
            .await
            .map_err(|e| BackuperError::Storage(format!("SSH 连接失败: {e}")))?;

        let hash_alg = handle
            .best_supported_rsa_hash()
            .await
            .map_err(|e| BackuperError::Storage(format!("协商 RSA hash 算法失败: {e}")))?
            .flatten();
        let auth = handle
            .authenticate_publickey(
                &self.username,
                PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg),
            )
            .await
            .map_err(|e| BackuperError::Storage(format!("SSH 认证失败: {e}")))?;
        if !auth.success() {
            return Err(BackuperError::Storage("SSH 公钥认证未通过".to_string()));
        }

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| BackuperError::Storage(format!("打开 SSH 通道失败: {e}")))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| BackuperError::Storage(format!("启动 SFTP 子系统失败: {e}")))?;

        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| BackuperError::Storage(format!("SFTP 会话建立失败: {e}")))?;

        if let Some(parent) = self.path.parent() {
            let _ = sftp.create_dir(parent.to_string_lossy().to_string()).await;
        }

        let remote = dest.to_string_lossy().to_string();
        let mut local = tokio::fs::File::open(local_path).await?;
        let mut remote_file = sftp
            .create(&remote)
            .await
            .map_err(|e| BackuperError::Storage(format!("创建远程文件失败: {e}")))?;

        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = local.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            remote_file
                .write_all(&buf[..n])
                .await
                .map_err(|e| BackuperError::Storage(format!("写入远程文件失败: {e}")))?;
        }
        remote_file
            .shutdown()
            .await
            .map_err(|e| BackuperError::Storage(format!("关闭远程文件失败: {e}")))?;

        Ok(())
    }

    async fn try_rsync(&self, local_path: &Path, dest: &Path) -> Result<(), BackuperError> {
        self.ensure_remote_dir(dest).await?;
        let key = self.key_path()?;
        let ssh_arg = format!(
            "ssh -i {} -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/dev/null -o BatchMode=yes -p {}",
            key.display(),
            self.port
        );

        let status = Command::new("rsync")
            .args(["-az", "-e", &ssh_arg, "--timeout=300"])
            .arg(local_path)
            .arg(self.remote_dest(dest))
            .status()
            .await?;

        if !status.success() {
            return Err(BackuperError::Storage(format!(
                "rsync 退出码: {:?}",
                status.code()
            )));
        }
        Ok(())
    }

    async fn try_scp(&self, local_path: &Path, dest: &Path) -> Result<(), BackuperError> {
        self.ensure_remote_dir(dest).await?;
        let key = self.key_path()?;
        let mut args = self.ssh_common_options(&key);
        args.push("-P".to_string());
        args.push(self.port.to_string());
        args.push(local_path.to_string_lossy().to_string());
        args.push(self.remote_dest(dest));

        let status = Command::new("scp").args(&args).status().await?;

        if !status.success() {
            return Err(BackuperError::Storage(format!(
                "scp 退出码: {:?}",
                status.code()
            )));
        }
        Ok(())
    }
}

struct SshClientHandler;

impl Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[async_trait]
impl StorageBackend for SshStorage {
    async fn store(&self, local_path: &Path, remote_key: &str) -> Result<(), BackuperError> {
        let dest = self.path.join(remote_key);

        if let Err(e) = self.try_sftp(local_path, &dest).await {
            tracing::warn!(error = %e, "SFTP 上传失败，尝试 rsync");
            if let Err(e2) = self.try_rsync(local_path, &dest).await {
                tracing::warn!(error = %e2, "rsync 上传失败，尝试 scp");
                return self.try_scp(local_path, &dest).await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_path_uses_configured_key() {
        let storage = SshStorage::new(
            "example.com".to_string(),
            22,
            "root".to_string(),
            Some(PathBuf::from("/custom/key")),
            PathBuf::from("/backups"),
        );
        assert_eq!(storage.key_path().unwrap(), PathBuf::from("/custom/key"));
    }
}
