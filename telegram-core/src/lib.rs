use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use thiserror::Error;
use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_session::Session;

uniffi::include_scaffolding!("telegram");

#[derive(Debug, Error)]
pub enum TelegramError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Invalid phone number")]
    InvalidPhone,
    #[error("Invalid verification code")]
    InvalidCode,
    #[error("Password 2FA required")]
    Password2FARequired,
    #[error("Session has expired")]
    SessionExpired,
    #[error("Internal error: {0}")]
    InternalError(String),
}

pub struct ChatItem {
    pub id: i64,
    pub title: String,
    pub last_message: String,
    pub timestamp: i64,
    pub unread_count: i32,
}

pub fn get_version() -> String {
    "0.1.0-alpha (Rust + UniFFI + Grammers)".to_string()
}

pub struct TelegramClient {
    api_id: i32,
    api_hash: String,
    session_path: String,
    runtime: Arc<Runtime>,
    client: Arc<Mutex<Option<Client>>>,
    login_token: Arc<Mutex<Option<grammers_client::types::LoginToken>>>,
}

impl TelegramClient {
    pub fn new(api_id: i32, api_hash: String, session_path: String) -> Self {
        let rt = Runtime::new().expect("Gagal menginisialisasi Tokio runtime");
        Self {
            api_id,
            api_hash,
            session_path,
            runtime: Arc::new(rt),
            client: Arc::new(Mutex::new(None)),
            login_token: Arc::new(Mutex::new(None)),
        }
    }

    pub fn init_logger(&self) -> Result<(), TelegramError> {
        #[cfg(target_os = "android")]
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag("TelegramRustCore")
                .with_max_level(log::LevelFilter::Debug),
        );
        Ok(())
    }

    pub fn is_authorized(&self) -> Result<bool, TelegramError> {
        let path = std::path::Path::new(&self.session_path);
        if !path.exists() {
            return Ok(false);
        }

        self.runtime.block_on(async {
            let session = match Session::load_file(&self.session_path) {
                Ok(s) => s,
                Err(_) => return Ok(false),
            };

            let client = match Client::connect(Config {
                session,
                api_id: self.api_id,
                api_hash: self.api_hash.clone(),
                params: InitParams::default(),
            }).await {
                Ok(c) => c,
                Err(e) => return Err(TelegramError::NetworkError(e.to_string())),
            };

            let authorized = client.is_authorized().await.unwrap_or(false);
            if authorized {
                let mut lock = self.client.lock().unwrap();
                *lock = Some(client);
            }
            Ok(authorized)
        })
    }

    pub fn request_login_code(&self, phone_number: String) -> Result<String, TelegramError> {
        let phone = phone_number.trim().to_string();
        if phone.is_empty() {
            return Err(TelegramError::InvalidPhone);
        }

        self.runtime.block_on(async {
            let session = Session::load_file_or_create(&self.session_path)
                .map_err(|e| TelegramError::InternalError(e.to_string()))?;

            let client = Client::connect(Config {
                session,
                api_id: self.api_id,
                api_hash: self.api_hash.clone(),
                params: InitParams::default(),
            }).await.map_err(|e| TelegramError::NetworkError(e.to_string()))?;

            let token = client.request_login_code(&phone).await
                .map_err(|e| TelegramError::NetworkError(e.to_string()))?;

            let mut client_lock = self.client.lock().unwrap();
            *client_lock = Some(client);

            let mut token_lock = self.login_token.lock().unwrap();
            *token_lock = Some(token);

            Ok(phone)
        })
    }

    pub fn submit_login_code(&self, phone_code: String, _phone_code_hash: String) -> Result<bool, TelegramError> {
        let code = phone_code.trim().to_string();
        if code.is_empty() {
            return Err(TelegramError::InvalidCode);
        }

        self.runtime.block_on(async {
            let client = {
                let lock = self.client.lock().unwrap();
                lock.clone().ok_or_else(|| TelegramError::InternalError("Client belum terkoneksi".to_string()))?
            };

            let token = {
                let mut lock = self.login_token.lock().unwrap();
                lock.take().ok_or_else(|| TelegramError::InternalError("Login token tidak valid atau expired".to_string()))?
            };

            match client.sign_in(&token, &code).await {
                Ok(_) => {
                    if let Ok(mut lock) = self.client.lock() {
                        if let Some(c) = lock.as_mut() {
                            let _ = c.session().save_to_file(&self.session_path);
                        }
                    }
                    Ok(true)
                }
                Err(SignInError::PasswordRequired(_)) => Err(TelegramError::Password2FARequired),
                Err(SignInError::InvalidCode) => Err(TelegramError::InvalidCode),
                Err(e) => Err(TelegramError::InternalError(e.to_string())),
            }
        })
    }

    pub fn fetch_chats(&self, limit: i32) -> Result<Vec<ChatItem>, TelegramError> {
        self.runtime.block_on(async {
            let client = {
                let lock = self.client.lock().unwrap();
                lock.clone().ok_or_else(|| TelegramError::InternalError("Belum login".to_string()))?
            };

            let mut iter = client.iter_dialogs();
            let mut results = Vec::new();
            let mut count = 0;

            while let Ok(Some(dialog)) = iter.next().await {
                let chat = dialog.chat;
                let last_msg = dialog.last_message.map(|m| m.text().to_string()).unwrap_or_default();

                results.push(ChatItem {
                    id: chat.id(),
                    title: chat.name().to_string(),
                    last_message: last_msg,
                    timestamp: 0,
                    unread_count: 0,
                });

                count += 1;
                if count >= limit {
                    break;
                }
            }

            Ok(results)
        })
    }

    pub fn send_text_message(&self, chat_id: i64, text: String) -> Result<(), TelegramError> {
        if text.trim().is_empty() {
            return Ok(());
        }

        self.runtime.block_on(async {
            let client = {
                let lock = self.client.lock().unwrap();
                lock.clone().ok_or_else(|| TelegramError::InternalError("Belum login".to_string()))?
            };

            let chat = match client.resolve_username(&chat_id.to_string()).await {
                Ok(Some(c)) => c,
                _ => return Err(TelegramError::InternalError("Chat tidak ditemukan".to_string())),
            };
            client.send_message(&chat, text).await
                .map_err(|e| TelegramError::NetworkError(e.to_string()))?;
            Ok(())
        })
    }
}
