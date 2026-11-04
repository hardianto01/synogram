package com.telegram.app.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.telegram.app.ui.screens.LoginStep
import uniffi.telegram_core.ChatItem
import uniffi.telegram_core.TelegramClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.io.File

class TelegramViewModel(application: Application) : AndroidViewModel(application) {

    private val apiId = 33417352
    private val apiHash = "5143e930a58dfa074620f2f3b885f5b1"
    private val sessionPath = File(application.filesDir, "telegram.session").absolutePath

    private val client: TelegramClient by lazy {
        try {
            System.loadLibrary("telegram_core")
        } catch (t: Throwable) {
            t.printStackTrace()
        }
        TelegramClient(apiId, apiHash, sessionPath).apply {
            try {
                initLogger()
            } catch (t: Throwable) {
                t.printStackTrace()
            }
        }
    }

    private val _isAuthorized = MutableStateFlow(false)
    val isAuthorized = _isAuthorized.asStateFlow()

    private val _loginStep = MutableStateFlow<LoginStep>(LoginStep.EnterPhone)
    val loginStep = _loginStep.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading = _isLoading.asStateFlow()

    private val _chats = MutableStateFlow<List<ChatItem>>(emptyList())
    val chats = _chats.asStateFlow()

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage = _errorMessage.asStateFlow()

    init {
        checkSession()
    }

    private fun checkSession() {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val authed = client.isAuthorized()
                _isAuthorized.value = authed
                if (authed) {
                    loadChats()
                }
            } catch (e: Throwable) {
                e.printStackTrace()
                _isAuthorized.value = false
            }
        }
    }

    fun sendPhoneNumber(phone: String) {
        viewModelScope.launch(Dispatchers.IO) {
            _isLoading.value = true
            _errorMessage.value = null
            try {
                val phoneCodeHash = client.requestLoginCode(phone)
                _loginStep.value = LoginStep.EnterCode(phoneCodeHash)
            } catch (e: Throwable) {
                e.printStackTrace()
                _errorMessage.value = e.localizedMessage ?: "Gagal meminta kode verifikasi"
            } finally {
                _isLoading.value = false
            }
        }
    }

    fun submitCode(code: String) {
        val currentStep = _loginStep.value as? LoginStep.EnterCode ?: return
        viewModelScope.launch(Dispatchers.IO) {
            _isLoading.value = true
            _errorMessage.value = null
            try {
                val success = client.submitLoginCode(code, currentStep.phoneCodeHash)
                if (success) {
                    _isAuthorized.value = true
                    loadChats()
                }
            } catch (e: uniffi.telegram_core.TelegramException.Password2FARequired) {
                _loginStep.value = LoginStep.EnterPassword("Masukkan kata sandi 2FA")
            } catch (e: Throwable) {
                e.printStackTrace()
                _errorMessage.value = e.localizedMessage ?: "Kode verifikasi salah"
            } finally {
                _isLoading.value = false
            }
        }
    }

    fun loadChats() {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val chatList = client.fetchChats(30)
                _chats.value = chatList
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }
}
