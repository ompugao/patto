package com.sifi.patto

import android.app.Activity
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import android.webkit.MimeTypeMap
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.InputStream

class MainActivity : FlutterActivity() {
    private var channel: MethodChannel? = null
    private var pending: Map<String, String?>? = null
    private var pendingPick: MethodChannel.Result? = null

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
        val messenger = flutterEngine.dartExecutor.binaryMessenger
        channel = MethodChannel(messenger, CHANNEL).also {
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
        MethodChannel(messenger, DEVICE_FILES_CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                "readClipboardImage" -> offMain(result) { findClipboardImage() }
                "pickFile" -> pickFile(result)
                else -> result.notImplemented()
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

    // Content providers may be slow or throw, and neither should stall or
    // kill the app; every failure, OutOfMemoryError included, answers null.
    private fun offMain(result: MethodChannel.Result, read: () -> Map<String, Any?>?) {
        val main = Handler(Looper.getMainLooper())
        Thread {
            var reply: Map<String, Any?>? = null
            try {
                reply = read()
            } catch (e: Throwable) {
                reply = null
            } finally {
                main.post { result.success(reply) }
            }
        }.start()
    }

    private fun findClipboardImage(): Map<String, Any?>? {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clip = clipboard.primaryClip ?: return null
        for (i in 0 until clip.itemCount) {
            val uri = clip.getItemAt(i).uri ?: continue
            val type = mimeTypeOf(uri) ?: continue
            if (!type.startsWith("image/")) continue
            val bytes = contentResolver.openInputStream(uri)?.use { readBounded(it) } ?: continue
            return mapOf("bytes" to bytes, "mimeType" to type)
        }
        return null
    }

    // Providers often report no length up front, so the cap is enforced while
    // reading rather than trusted from a descriptor.
    private fun readBounded(input: InputStream): ByteArray? {
        val out = ByteArrayOutputStream()
        val buffer = ByteArray(64 * 1024)
        while (true) {
            val n = input.read(buffer)
            if (n < 0) break
            if (out.size() + n > MAX_IMAGE_BYTES) return null
            out.write(buffer, 0, n)
        }
        return out.toByteArray()
    }

    // The resolver knows the type of a content URI; a file URI is judged by
    // its extension.
    private fun mimeTypeOf(uri: Uri): String? {
        contentResolver.getType(uri)?.let { return it }
        val extension = MimeTypeMap.getFileExtensionFromUrl(uri.toString())
        if (extension.isNullOrEmpty()) return null
        return MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension.lowercase())
    }

    private fun pickFile(result: MethodChannel.Result) {
        if (pendingPick != null) {
            result.success(null)
            return
        }
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
        }
        try {
            startActivityForResult(intent, PICK_FILE)
            pendingPick = result
        } catch (e: Exception) {
            result.success(null)
        }
    }

    @Deprecated("Deprecated in Java")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != PICK_FILE) return
        val result = pendingPick ?: return
        pendingPick = null
        val uri = data?.data
        if (resultCode != Activity.RESULT_OK || uri == null) {
            result.success(null)
            return
        }
        offMain(result) { copyToCache(uri) }
    }

    // The picked file is streamed into the app's cache, so a large PDF never
    // has to fit in memory; Dart moves it into the workspace from there.
    private fun copyToCache(uri: Uri): Map<String, Any?>? {
        val name = displayNameOf(uri) ?: "file"
        val dir = File(cacheDir, "picked").apply { mkdirs() }
        val target = File.createTempFile("pick-", "", dir)
        contentResolver.openInputStream(uri)?.use { input ->
            target.outputStream().use { output -> input.copyTo(output) }
        } ?: run {
            target.delete()
            return null
        }
        return mapOf("path" to target.absolutePath, "name" to name)
    }

    private fun displayNameOf(uri: Uri): String? {
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            ?.use { cursor ->
                if (cursor.moveToFirst() && !cursor.isNull(0)) return cursor.getString(0)
            }
        return uri.lastPathSegment
    }

    companion object {
        const val CHANNEL = "com.sifi.patto/quick_note"
        const val ACTION_QUICK_NOTE = "com.sifi.patto.action.QUICK_NOTE"
        const val DEVICE_FILES_CHANNEL = "com.sifi.patto/device_files"
        const val PICK_FILE = 0x5041
        // Three copies of the image exist while it crosses the channel.
        const val MAX_IMAGE_BYTES = 48 * 1024 * 1024
    }
}
