use async_trait::async_trait;
use backuper_core::config::NotifierConfig;
use backuper_core::error::BackuperError;
use backuper_core::notification::Notifier;
use serde_json::json;

pub fn build_notifier(config: &NotifierConfig) -> Result<Box<dyn Notifier>, BackuperError> {
    match config {
        NotifierConfig::Webhook { url, .. } => Ok(Box::new(WebhookNotifier { url: url.clone() })),
        NotifierConfig::Discord {
            token, channel_id, ..
        } => Ok(Box::new(DiscordNotifier {
            token: token.clone(),
            channel_id: channel_id.clone(),
        })),
        NotifierConfig::Telegram { token, chat_id, .. } => Ok(Box::new(TelegramNotifier {
            token: token.clone(),
            chat_id: chat_id.clone(),
        })),
    }
}

pub fn format_message(
    success: bool,
    rule_id: &str,
    archive_key: Option<&str>,
    error: Option<&str>,
) -> String {
    let key_info = archive_key
        .map(|k| format!(", 归档: {k}"))
        .unwrap_or_default();
    let error_info = error.map(|e| format!(", 错误: {e}")).unwrap_or_default();

    if success {
        format!("规则 {rule_id} 备份成功{key_info}")
    } else {
        format!("规则 {rule_id} 备份失败{key_info}{error_info}")
    }
}

pub async fn send_all(
    configs: &[NotifierConfig],
    success: bool,
    rule_id: &str,
    archive_key: Option<&str>,
    error: Option<&str>,
) -> Result<(), BackuperError> {
    if configs.is_empty() {
        return Ok(());
    }
    let message = format_message(success, rule_id, archive_key, error);
    for config in configs {
        let notifier = build_notifier(config)?;
        if let Err(e) = notifier.send(&message).await {
            tracing::warn!(notifier_id = %config.id(), error = %e, "发送通知失败");
        }
    }
    Ok(())
}

struct WebhookNotifier {
    url: String,
}

#[async_trait]
impl Notifier for WebhookNotifier {
    async fn send(&self, message: &str) -> Result<(), BackuperError> {
        let client = reqwest::Client::new();
        client
            .post(&self.url)
            .json(&json!({ "text": message }))
            .send()
            .await
            .map_err(|e| BackuperError::Storage(format!("Webhook 请求失败: {e}")))?
            .error_for_status()
            .map_err(|e| BackuperError::Storage(format!("Webhook 响应错误: {e}")))?;
        Ok(())
    }
}

struct DiscordNotifier {
    token: String,
    channel_id: String,
}

#[async_trait]
impl Notifier for DiscordNotifier {
    async fn send(&self, message: &str) -> Result<(), BackuperError> {
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            self.channel_id
        );
        let client = reqwest::Client::new();
        client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&json!({ "content": message }))
            .send()
            .await
            .map_err(|e| BackuperError::Storage(format!("Discord 请求失败: {e}")))?
            .error_for_status()
            .map_err(|e| BackuperError::Storage(format!("Discord 响应错误: {e}")))?;
        Ok(())
    }
}

struct TelegramNotifier {
    token: String,
    chat_id: String,
}

#[async_trait]
impl Notifier for TelegramNotifier {
    async fn send(&self, message: &str) -> Result<(), BackuperError> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let client = reqwest::Client::new();
        client
            .post(&url)
            .json(&json!({
                "chat_id": self.chat_id,
                "text": message,
            }))
            .send()
            .await
            .map_err(|e| BackuperError::Storage(format!("Telegram 请求失败: {e}")))?
            .error_for_status()
            .map_err(|e| BackuperError::Storage(format!("Telegram 响应错误: {e}")))?;
        Ok(())
    }
}
