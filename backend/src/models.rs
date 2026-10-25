use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub profile_id: Option<String>,
    pub java_path: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub game_args: Option<String>,
    pub icon: Option<String>,
    pub installed: bool,
    pub last_played: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstance {
    pub name: String,
    pub version_id: String,
    #[serde(default)]
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub java_path: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub game_args: Option<String>,
    pub icon: Option<String>,
}

impl Instance {
    /// 启动与解析时使用的版本 ID：加载器配置（安装后才有）或原版版本
    pub fn launch_version(&self) -> &str {
        self.profile_id.as_deref().filter(|_| self.loader != "vanilla").unwrap_or(&self.version_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstance {
    pub name: String,
    pub java_path: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub game_args: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub kind: String,
    pub username: String,
    pub uuid: String,
    #[serde(skip_serializing)]
    pub access_token: Option<String>,
    #[serde(skip_serializing)]
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub skin_url: Option<String>,
    pub is_default: bool,
    pub created_at: i64,
}

/// Account 模块（Kotlin）返回的登录结果
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResult {
    pub kind: String,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub skin_url: Option<String>,
}

impl AuthResult {
    pub fn into_account(self, created_at: i64) -> Account {
        Account {
            id: format!("{}:{}", self.kind, self.uuid),
            kind: self.kind,
            username: self.username,
            uuid: self.uuid,
            access_token: Some(self.access_token),
            refresh_token: self.refresh_token,
            expires_at: self.expires_at,
            skin_url: self.skin_url,
            is_default: false,
            created_at,
        }
    }
}
