package com.sifi.patto

import android.content.Intent
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private var channel: MethodChannel? = null
    private var pending: Map<String, String?>? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // A recreated activity gets its launching intent again; the quick note
        // it carried was already shown the first time.
        if (savedInstanceState == null) {
            pending = quickNoteFrom(intent)
        }
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL).also {
            it.setMethodCallHandler { call, result ->
                when (call.method) {
                    "consume" -> {
                        result.success(pending)
                        pending = null
                    }
                    else -> result.notImplemented()
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        val request = quickNoteFrom(intent) ?: return
        val channel = channel
        if (channel == null) {
            pending = request
            return
        }
        // Dart registers its handler once its shell has started; before that
        // the call comes back unhandled and the request waits for `consume`.
        channel.invokeMethod("quickNote", request, object : MethodChannel.Result {
            override fun success(result: Any?) {}

            override fun error(code: String, message: String?, details: Any?) {
                pending = request
            }

            override fun notImplemented() {
                pending = request
            }
        })
    }

    override fun cleanUpFlutterEngine(flutterEngine: FlutterEngine) {
        channel?.setMethodCallHandler(null)
        channel = null
        super.cleanUpFlutterEngine(flutterEngine)
    }

    private fun quickNoteFrom(intent: Intent?): Map<String, String?>? {
        if (intent == null) return null
        return when (intent.action) {
            ACTION_QUICK_NOTE -> mapOf("text" to "", "subject" to null)
            Intent.ACTION_SEND -> {
                if (intent.type?.startsWith("text/") != true) return null
                mapOf(
                    "text" to (intent.getStringExtra(Intent.EXTRA_TEXT) ?: ""),
                    "subject" to intent.getStringExtra(Intent.EXTRA_SUBJECT),
                )
            }
            else -> null
        }
    }

    companion object {
        const val CHANNEL = "com.sifi.patto/quick_note"
        const val ACTION_QUICK_NOTE = "com.sifi.patto.action.QUICK_NOTE"
    }
}
