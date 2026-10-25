//! Kotlin 账户模块（account/）的类型化封装

use serde_json::json;

use super::sidecar::Sidecar;
use crate::models::AuthResult;

pub struct AccountService {
    rpc: Sidecar,
}

impl AccountService {
    pub fn new(rpc: Sidecar) -> Self {
        Self { rpc }
    }

    pub async fn offline(&self, username: &str) -> anyhow::Result<AuthResult> {
        self.rpc.call_as("offline.create", json!({ "username": username })).await
    }

    /// 设备码登录；期间账户模块会发出 `account.deviceCode` 通知（转为前端事件）
    pub async fn microsoft_login(&self, client_id: &str) -> anyhow::Result<AuthResult> {
        self.rpc.call_as("microsoft.login", json!({ "clientId": client_id })).await
    }

    pub async fn microsoft_refresh(&self, client_id: &str, refresh_token: &str) -> anyhow::Result<AuthResult> {
        self.rpc
            .call_as("microsoft.refresh", json!({ "clientId": client_id, "refreshToken": refresh_token }))
            .await
    }

    pub async fn stop(&self) {
        self.rpc.stop().await;
    }
}
