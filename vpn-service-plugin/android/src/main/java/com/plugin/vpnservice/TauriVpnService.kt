package com.plugin.vpnservice

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.VpnService
import android.os.Build
import android.os.Bundle
import android.os.ParcelFileDescriptor
import androidx.core.app.NotificationCompat
import app.tauri.plugin.JSObject

/**
 * 真正建立 TUN 设备的 VpnService。
 * 结构取自 EasyTier 官方 tauri-plugin-vpnservice：前台服务 + 常驻通知 + IPv4/IPv6 地址。
 */
class TauriVpnService : VpnService() {
    companion object {
        @JvmField
        var triggerCallback: (String, JSObject) -> Unit = { _, _ -> }

        @JvmField
        var onEstablished: ((Int) -> Unit)? = null

        @JvmField
        var self: TauriVpnService? = null

        @JvmField
        var ipv4Addr: String? = null

        @JvmField
        var routes: Array<String> = emptyArray()

        @JvmField
        var dns: String? = null

        const val IPV4_ADDR = "IPV4_ADDR"
        const val ROUTES = "ROUTES"
        const val DNS = "DNS"
        const val DISALLOWED_APPLICATIONS = "DISALLOWED_APPLICATIONS"
        const val MTU = "MTU"

        private const val NOTIFICATION_CHANNEL_ID = "astral_vpn_channel"
        private const val NOTIFICATION_ID = 1101
    }

    private var vpnInterface: ParcelFileDescriptor? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        try {
            startVpnForegroundService()

            val args = intent?.getExtras()
            ipv4Addr = args?.getString(IPV4_ADDR)
            routes = args?.getStringArray(ROUTES) ?: emptyArray()
            dns = args?.getString(DNS)

            val pfd = createVpnInterface(args)
            vpnInterface = pfd

            val eventData = JSObject()
            eventData.put("fd", pfd.fd)
            triggerCallback("vpn_service_start", eventData)

            onEstablished?.invoke(pfd.fd)
        } catch (e: Exception) {
            println("astral vpn start failed: ${e.message}")
            onEstablished?.invoke(-1)
        }
        return START_STICKY
    }

    override fun onCreate() {
        super.onCreate()
        self = this
    }

    override fun onDestroy() {
        disconnect()
        stopForeground(STOP_FOREGROUND_REMOVE)
        self = null
        super.onDestroy()
    }

    override fun onRevoke() {
        disconnect()
        stopForeground(STOP_FOREGROUND_REMOVE)
        self = null
        super.onRevoke()
    }

    private fun disconnect() {
        if (self == this) {
            triggerCallback("vpn_service_stop", JSObject())
        }
        try {
            vpnInterface?.close()
        } catch (_: Exception) {
            // 忽略重复关闭
        }
        vpnInterface = null
        ipv4Addr = null
        routes = emptyArray()
        dns = null
    }

    private fun startVpnForegroundService() {
        createNotificationChannel()

        val launchIntent = packageManager.getLaunchIntentForPackage(packageName)?.apply {
            addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        }
        val contentIntent = launchIntent?.let {
            PendingIntent.getActivity(
                this,
                0,
                it,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
        }

        val notification = NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_menu_manage)
            .setContentTitle("Astral VPN 正在运行")
            .setContentText("虚拟网络已连接")
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .apply { contentIntent?.let(::setContentIntent) }
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                NOTIFICATION_CHANNEL_ID,
                "Astral VPN",
                NotificationManager.IMPORTANCE_LOW
            )
            getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
        }
    }

    private fun createVpnInterface(args: Bundle?): ParcelFileDescriptor {
        val builder = Builder()
            .setSession("Astral")
            .setBlocking(false)

        val mtu = args?.getInt(MTU) ?: 1500
        val ipv4Addr = args?.getString(IPV4_ADDR) ?: "10.126.126.1/24"
        val dns = args?.getString(DNS)
        val routes = args?.getStringArray(ROUTES) ?: emptyArray()
        val disallowedApplications =
            args?.getStringArray(DISALLOWED_APPLICATIONS) ?: emptyArray()

        val ipParts = ipv4Addr.split("/")
        if (ipParts.size != 2) throw IllegalArgumentException("Invalid IP addr string")
        builder.addAddress(ipParts[0], ipParts[1].toInt())
        builder.addAddress("fd00::1", 128)

        builder.setMtu(mtu)
        dns?.let { builder.addDnsServer(it) }

        for (route in routes) {
            val parts = route.split("/")
            if (parts.size != 2) throw IllegalArgumentException("Invalid route cidr string")
            builder.addRoute(parts[0], parts[1].toInt())
        }

        for (app in disallowedApplications) {
            try {
                builder.addDisallowedApplication(app)
            } catch (_: Exception) {
                // 包名不存在时忽略
            }
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            builder.setMetered(false)
        }

        return builder.establish() ?: throw IllegalStateException("Failed to init VpnService")
    }
}
