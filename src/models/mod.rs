use serde::{Deserialize, Serialize};

// ===================== Account / Profile =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    #[serde(rename = "encryptedPrivateKey")]
    pub encrypted_private_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(rename = "masterPasswordHash")]
    pub master_password_hash: String,
    #[serde(rename = "masterPasswordHint")]
    pub master_password_hint: String,
    pub key: String,
    pub keys: KeyPair,
    #[serde(skip)]
    pub refresh_token: String,
    #[serde(skip)]
    pub two_factor_secret: String,
    pub kdf: i32,
    #[serde(rename = "kdfIterations")]
    pub kdf_iterations: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: Option<String>,
    pub email: String,
    #[serde(rename = "emailVerified")]
    pub email_verified: bool,
    pub premium: bool,
    pub culture: String,
    #[serde(rename = "twoFactorEnabled")]
    pub two_factor_enabled: bool,
    pub key: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
    #[serde(rename = "securityStamp")]
    pub security_stamp: Option<String>,
    pub organizations: Vec<String>,
    pub object: String,
    #[serde(rename = "masterPasswordHint")]
    pub master_password_hint: Option<String>,
}

impl From<Account> for Profile {
    fn from(acc: Account) -> Self {
        Profile {
            id: acc.id.clone(),
            name: if acc.name.is_empty() { None } else { Some(acc.name) },
            email: acc.email,
            email_verified: false,
            premium: false,
            culture: "en-US".to_string(),
            two_factor_enabled: !acc.two_factor_secret.is_empty(),
            key: acc.key,
            private_key: acc.keys.encrypted_private_key,
            security_stamp: None,
            organizations: vec![],
            object: "profile".to_string(),
            master_password_hint: if acc.master_password_hint.is_empty() {
                None
            } else {
                Some(acc.master_password_hint)
            },
        }
    }
}

// ===================== Cipher =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Login {
    pub password: Option<String>,
    pub totp: Option<String>,
    pub uri: Option<String>,
    pub uris: Option<Vec<Uri>>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uri {
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureNote {
    #[serde(rename = "type")]
    pub type_field: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cipher {
    #[serde(rename = "type")]
    pub type_field: i32,
    #[serde(rename = "folderId", default)]
    pub folder_id: Option<String>,
    #[serde(rename = "organizationId", default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub edit: bool,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub data: Option<CipherData>,
    #[serde(default)]
    pub attachments: Vec<String>,
    #[serde(rename = "organizationUseTotp", default)]
    pub organization_use_totp: bool,
    #[serde(rename = "revisionDate", default)]
    pub revision_date: String,
    #[serde(default)]
    pub object: String,
    #[serde(rename = "collectionIds", default)]
    pub collection_ids: Vec<String>,
    #[serde(default)]
    pub card: Option<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub identity: Option<String>,
    #[serde(default)]
    pub login: Option<Login>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(rename = "secureNote", default)]
    pub secure_note: Option<SecureNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CipherData {
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub totp: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub uris: Option<Vec<Uri>>,
}

// ===================== Folder =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub object: String,
    #[serde(rename = "revisionDate")]
    pub revision_date: String,
}

// ===================== Sync =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncData {
    pub profile: Profile,
    pub folders: Vec<Folder>,
    pub ciphers: Vec<Cipher>,
    pub domains: Domains,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domains {
    #[serde(rename = "equivalentDomains")]
    pub equivalent_domains: Vec<String>,
    #[serde(rename = "globalEquivalentDomains")]
    pub global_equivalent_domains: Vec<GlobalEquivalentDomain>,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalEquivalentDomain {
    #[serde(rename = "type")]
    pub type_field: i32,
    pub domains: Vec<String>,
    pub excluded: bool,
}

// ===================== API Request/Response Types =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreloginRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreloginResponse {
    pub kdf: i32,
    #[serde(rename = "kdfIterations")]
    pub kdf_iterations: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    #[serde(rename = "masterPasswordHash")]
    pub master_password_hash: String,
    #[serde(rename = "masterPasswordHint")]
    pub master_password_hint: String,
    pub key: String,
    pub keys: KeyPair,
    pub kdf: i32,
    #[serde(rename = "kdfIterations")]
    pub kdf_iterations: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device: Option<DeviceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    #[serde(rename = "deviceIdentifier")]
    pub device_identifier: String,
    #[serde(rename = "deviceName")]
    pub device_name: String,
    #[serde(rename = "deviceType")]
    pub device_type: String,
    #[serde(rename = "pushToken")]
    pub push_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    #[serde(rename = "expires_in")]
    pub expires_in: i64,
    pub token_type: String,
    pub refresh_token: String,
    #[serde(rename = "Key")]
    pub key: Option<String>,
    #[serde(rename = "privateKey")]
    pub private_key: Option<String>,
    #[serde(rename = "Kdf")]
    pub kdf: Option<i32>,
    #[serde(rename = "KdfIterations")]
    pub kdf_iterations: Option<i32>,
    #[serde(rename = "twoFactorToken")]
    pub two_factor_token: Option<String>,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorRequest {
    #[serde(rename = "type")]
    pub type_field: i32,
    pub token: String,
    pub remember: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorResponse {
    pub enabled: bool,
    pub object: String,
    #[serde(rename = "twoFactorProviders")]
    pub two_factor_providers: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderCreateRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRenameRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysUpdateRequest {
    #[serde(rename = "encryptedPrivateKey")]
    pub encrypted_private_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
}
