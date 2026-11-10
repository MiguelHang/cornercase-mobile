package dev.usecornercase.network

import android.app.Activity
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import app.tauri.Logger
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class WatchArgs {
    lateinit var channel: Channel
}

private data class Path(val kind: String, val expensive: Boolean) {
    fun sent(): JSObject = JSObject().put("kind", kind).put("expensive", expensive)
}

private val NONE = Path("none", false)

private fun pathOf(capabilities: NetworkCapabilities?): Path {
    if (capabilities == null) {
        return NONE
    }
    val kind = when {
        capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "wifi"
        capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
        else -> "other"
    }
    return Path(kind, !capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED))
}

private object Watcher : ConnectivityManager.NetworkCallback() {
    private var channel: Channel? = null
    private var last: Path? = null

    @Synchronized
    fun watch(connectivity: ConnectivityManager, next: Channel) {
        channel = next
        if (last == null) {
            connectivity.registerDefaultNetworkCallback(this)
            last = pathOf(connectivity.activeNetwork?.let(connectivity::getNetworkCapabilities))
        }
        send(last ?: NONE)
    }

    override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
        changed(pathOf(capabilities))
    }

    override fun onLost(network: Network) {
        changed(NONE)
    }

    @Synchronized
    private fun changed(path: Path) {
        last = path
        send(path)
    }

    private fun send(path: Path) {
        try {
            channel?.send(path.sent())
        } catch (failed: Exception) {
            Logger.error("cornercase-mobile: network path not sent", failed)
        }
    }
}

@TauriPlugin
class NetworkPlugin(activity: Activity) : Plugin(activity) {
    private val connectivity: ConnectivityManager =
        activity.applicationContext.getSystemService(ConnectivityManager::class.java)

    @Command
    fun watch(invoke: Invoke) {
        Watcher.watch(connectivity, invoke.parseArgs(WatchArgs::class.java).channel)
        invoke.resolve()
    }
}
