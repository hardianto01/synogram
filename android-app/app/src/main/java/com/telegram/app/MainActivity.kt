package com.telegram.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import com.telegram.app.ui.screens.ChatListScreen
import com.telegram.app.ui.screens.LoginScreen
import com.telegram.app.viewmodel.TelegramViewModel

class MainActivity : ComponentActivity() {

    private val viewModel: TelegramViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    val isAuthorized by viewModel.isAuthorized.collectAsState()
                    val loginStep by viewModel.loginStep.collectAsState()
                    val isLoading by viewModel.isLoading.collectAsState()
                    val chats by viewModel.chats.collectAsState()

                    if (!isAuthorized) {
                        LoginScreen(
                            step = loginStep,
                            isLoading = isLoading,
                            onSendPhone = { viewModel.sendPhoneNumber(it) },
                            onSendCode = { viewModel.submitCode(it) }
                        )
                    } else {
                        ChatListScreen(
                            chats = chats,
                            onChatClick = { /* Buka detail obrolan */ },
                            onNewChatClick = { /* Tampilkan modal kontak baru */ }
                        )
                    }
                }
            }
        }
    }
}
