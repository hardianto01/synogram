# Telegram Android Client: Jetpack Compose + Rust Engine (via UniFFI)

Arsitektur:
- **Rust Core (`telegram-core`)**: Engine MTProto Telegram berbasis `grammers` / Tokio, di-compile menjadi Android `.so` (JNI C-ABI) melalui Mozilla UniFFI.
- **Android App (`app`)**: Antarmuka murni **Jetpack Compose + Material 3** yang memanggil binding Kotlin hasil generate UniFFI.

## Struktur Project
```
telegram-rust-compose/
├── telegram-core/          # Rust Crate (Core MTProto + UniFFI interface)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   └── telegram.udl    # UniFFI Definition Language / UDL
│   └── build.rs
├── android-app/            # Android Project (Kotlin + Jetpack Compose)
│   ├── build.gradle.kts
│   ├── settings.gradle.kts
│   └── app/
│       ├── build.gradle.kts
│       └── src/main/
│           ├── AndroidManifest.xml
│           ├── java/com/telegram/app/
│           │   ├── MainActivity.kt
│           │   ├── ui/
│           │   │   ├── screens/LoginScreen.kt
│           │   │   ├── screens/ChatListScreen.kt
│           │   │   └── theme/Theme.kt
│           │   └── viewmodel/TelegramViewModel.kt
│           └── jniLibs/    # Output compile .so dari Rust (arm64-v8a, etc)
```
