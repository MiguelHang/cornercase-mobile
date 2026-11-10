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

private data class Seen(val network: Network?, val path: Path, val vpn: Boolean)

private val GONE = Seen(null, NONE, false)

private fun kindOf(capabilities: NetworkCapabilities): String = when {
    capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "wifi"
    capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
    else -> "other"
}

private fun seenOf(network: Network?, capabilities: NetworkCapabilities?): Seen {
    if (network == null || capabilities == null) {
        return GONE
    }
    val expensive = !capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)
    val vpn = capabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
    return Seen(network, Path(kindOf(capabilities), expensive), vpn)
}

private object Watcher : ConnectivityManager.NetworkCallback() {
    private var channel: Channel? = null
    private var last: Seen? = null

    @Synchronized
    fun watch(connectivity: ConnectivityManager, next: Channel) {
        channel = next
        if (last == null) {
            val network = connectivity.activeNetwork
            last = seenOf(network, network?.let(connectivity::getNetworkCapabilities))
            connectivity.registerDefaultNetworkCallback(this)
        }
        send((last ?: GONE).path)
    }

    override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
        changed(seenOf(network, capabilities))
    }

    @Synchronized
    override fun onLost(network: Network) {
        if (last?.network == network) {
            changed(GONE)
        }
    }

    @Synchronized
    private fun changed(seen: Seen) {
        if (seen == last) {
            return
        }
        last = seen
        send(seen.path)
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
