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

unsafe impl Send for TelegramClient {}
unsafe impl Sync for TelegramClient {}

impl TelegramClient {
    pub fn new(api_id: i32, api_hash: String, session_path: String) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Gagal menginisialisasi Tokio runtime");
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
        let _ = android_logger::init_once(
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

        let api_id = self.api_id;
        let api_hash = self.api_hash.clone();
        let session_path = self.session_path.clone();

        let res = self.runtime.block_on(async move {
            let session = match Session::load_file(&session_path) {
                Ok(s) => s,
                Err(_) => return Ok(false),
            };

            let client = match Client::connect(Config {
                session,
                api_id,
                api_hash,
                params: InitParams::default(),
            }).await {
                Ok(c) => c,
                Err(_) => return Ok(false),
            };

            let authed = client.is_authorized().await.unwrap_or(false);
            Ok::<_, TelegramError>((authed, client))
        });

        match res {
            Ok((authed, client)) => {
                if authed {
                    if let Ok(mut lock) = self.client.lock() {
                        *lock = Some(client);
                    }
                }
                Ok(authed)
            }
            Err(_) => Ok(false),
        }
    }

    pub fn request_login_code(&self, phone_number: String) -> Result<String, TelegramError> {
        let phone = phone_number.trim().to_string();
        if phone.is_empty() {
            return Err(TelegramError::InvalidPhone);
        }

        let api_id = self.api_id;
        let api_hash = self.api_hash.clone();
        let session_path = self.session_path.clone();

        let (client, token) = self.runtime.block_on(async move {
            let session = Session::load_file_or_create(&session_path)
                .map_err(|e| TelegramError::InternalError(format!("Gagal membuka sesi file: {}", e)))?;

            let client = Client::connect(Config {
                session,
                api_id,
                api_hash,
                params: InitParams::default(),
            }).await.map_err(|e| TelegramError::NetworkError(format!("Koneksi ke Telegram DC gagal: {}", e)))?;

            let token = client.request_login_code(&phone).await
                .map_err(|e| TelegramError::NetworkError(format!("Gagal meminta OTP untuk {}: {}", phone, e)))?;

            Ok::<_, TelegramError>((client, token))
        })?;

        if let Ok(mut client_lock) = self.client.lock() {
            *client_lock = Some(client);
        }
        if let Ok(mut token_lock) = self.login_token.lock() {
            *token_lock = Some(token);
        }

        Ok(phone_number)
    }

    pub fn submit_login_code(&self, phone_code: String, _phone_code_hash: String) -> Result<bool, TelegramError> {
        let code = phone_code.trim().to_string();
        if code.is_empty() {
            return Err(TelegramError::InvalidCode);
        }

        let client = {
            let lock = self.client.lock().map_err(|_| TelegramError::InternalError("Lock poison".to_string()))?;
            lock.clone().ok_or_else(|| TelegramError::InternalError("Client belum terkoneksi".to_string()))?
        };

        let token = {
            let mut lock = self.login_token.lock().map_err(|_| TelegramError::InternalError("Lock poison".to_string()))?;
            lock.take().ok_or_else(|| TelegramError::InternalError("Login token tidak valid atau expired. Silakan minta kode ulang.".to_string()))?
        };

        let session_path = self.session_path.clone();

        self.runtime.block_on(async move {
            match client.sign_in(&token, &code).await {
                Ok(_) => {
                    let _ = client.session().save_to_file(&session_path);
                    Ok(true)
                }
                Err(SignInError::PasswordRequired(_)) => Err(TelegramError::Password2FARequired),
                Err(SignInError::InvalidCode) => Err(TelegramError::InvalidCode),
                Err(e) => Err(TelegramError::InternalError(format!("Sign in gagal: {}", e))),
            }
        })
    }

    pub fn fetch_chats(&self, limit: i32) -> Result<Vec<ChatItem>, TelegramError> {
        let client = {
            let lock = self.client.lock().map_err(|_| TelegramError::InternalError("Lock poison".to_string()))?;
            lock.clone().ok_or_else(|| TelegramError::InternalError("Belum login".to_string()))?
        };

        self.runtime.block_on(async move {
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

        let client = {
            let lock = self.client.lock().map_err(|_| TelegramError::InternalError("Lock poison".to_string()))?;
            lock.clone().ok_or_else(|| TelegramError::InternalError("Belum login".to_string()))?
        };

        self.runtime.block_on(async move {
            let peer = client.resolve_username(&chat_id.to_string()).await;
            if let Ok(Some(chat)) = peer {
                let _ = client.send_message(&chat, text).await;
            }
            Ok(())
        })
    }
}

