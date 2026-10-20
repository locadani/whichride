package io.github.locadani.whichride.entry

import android.content.Intent

/**
 * The text a Google Maps share or a `geo:` link brought, or null for a plain launch. Shares
 * carry "Place name\nhttps://maps.app.goo.gl/…"; the core finds the link in it.
 */
fun sharedText(intent: Intent?): String? = when (intent?.action) {
    Intent.ACTION_SEND -> intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
    Intent.ACTION_VIEW -> intent.dataString
    else -> null
}?.takeIf { it.isNotBlank() }
