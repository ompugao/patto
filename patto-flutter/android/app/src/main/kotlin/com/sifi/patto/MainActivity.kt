package com.sifi.patto

import android.content.ClipboardManager
import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.MimeTypeMap
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "com.sifi.patto/clipboard")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "readImage" -> readClipboardImage(result)
                    else -> result.notImplemented()
                }
            }
    }

    // Reads the first image on the clipboard off the main thread: a content
    // provider may be slow or throw, and neither should stall or kill the app.
    // Replies with {bytes, mimeType}, or null when the clipboard holds no image.
    private fun readClipboardImage(result: MethodChannel.Result) {
        val main = Handler(Looper.getMainLooper())
        Thread {
            val reply = try {
                findClipboardImage()
            } catch (e: Exception) {
                null
            }
            main.post { result.success(reply) }
        }.start()
    }

    private fun findClipboardImage(): Map<String, Any?>? {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clip = clipboard.primaryClip ?: return null
        for (i in 0 until clip.itemCount) {
            val uri = clip.getItemAt(i).uri ?: continue
            val type = mimeTypeOf(uri) ?: continue
            if (!type.startsWith("image/")) continue
            val size = contentResolver.openAssetFileDescriptor(uri, "r")?.use { it.length } ?: -1L
            if (size > MAX_IMAGE_BYTES) return null
            val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: continue
            if (bytes.size > MAX_IMAGE_BYTES) return null
            return mapOf("bytes" to bytes, "mimeType" to type)
        }
        return null
    }

    // The resolver knows the type of a content URI; a file URI is judged by
    // its extension.
    private fun mimeTypeOf(uri: Uri): String? {
        contentResolver.getType(uri)?.let { return it }
        val extension = MimeTypeMap.getFileExtensionFromUrl(uri.toString())
        if (extension.isNullOrEmpty()) return null
        return MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension.lowercase())
    }

    private companion object {
        // Three copies of the image exist while it crosses the channel.
        const val MAX_IMAGE_BYTES = 48L * 1024 * 1024
    }
}
