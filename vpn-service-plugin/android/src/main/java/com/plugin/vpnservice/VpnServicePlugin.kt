package com.plugin.vpnservice

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.webkit.WebView
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@InvokeArg
class StartVpnArgs {
    var ipv4Addr: String? = null
    var routes: Array<String> = emptyArray()
    var dns: String? = null
    var disallowedApplications: Array<String> = emptyArray()
    var mtu: Int? = null
}

@TauriPlugin
class VpnServicePlugin(private val activity: Activity) : Plugin(activity) {

    override fun load(webView: WebView) {
        // 服务侧事件（vpn_service_start / vpn_service_stop）仍转发给前端，
        // 便于界面显示真实 VPN 状态；TUN fd 由 startVpn 直接返回。
        TauriVpnService.triggerCallback = { event, data -> trigger(event, data) }
    }

    /** 检查/申请系统 VPN 授权。 */
    @Command
    fun prepareVpn(invoke: Invoke) {
        activity.runOnUiThread {
            val intent = VpnService.prepare(activity)
            if (intent != null) {
                startActivityForResult(invoke, intent, "onPrepareVpnResult")
                return@runOnUiThread
            }
            val ret = JSObject()
            ret.put("granted", true)
            invoke.resolve(ret)
        }
    }

    @ActivityCallback
    fun onPrepareVpnResult(invoke: Invoke, result: ActivityResult) {
        val ret = JSObject()
        ret.put("granted", result.resultCode == Activity.RESULT_OK)
        invoke.resolve(ret)
    }

    /**
     * 启动 VpnService，等待 TUN 建立并把 fd 返回。
     * 建立动作在服务线程执行，这里用有界等待，避免 UI 线程阻塞。
     */
    @Command
    fun startVpn(invoke: Invoke) {
        val args = invoke.parseArgs(StartVpnArgs::class.java)
        activity.runOnUiThread {
            TauriVpnService.self?.onRevoke()

            val prepareIntent = VpnService.prepare(activity)
            val ret = JSObject()
            if (prepareIntent != null) {
                ret.put("errorMsg", "need_prepare")
                invoke.resolve(ret)
                return@runOnUiThread
            }

            val intent = Intent(activity, TauriVpnService::class.java)
            intent.putExtra(TauriVpnService.IPV4_ADDR, args.ipv4Addr)
            intent.putExtra(TauriVpnService.ROUTES, args.routes)
            intent.putExtra(TauriVpnService.DNS, args.dns)
            intent.putExtra(
                TauriVpnService.DISALLOWED_APPLICATIONS,
                args.disallowedApplications
            )
            intent.putExtra(TauriVpnService.MTU, args.mtu)

            val latch = CountDownLatch(1)
            val resultFd = intArrayOf(-1)
            TauriVpnService.onEstablished = { fd ->
                resultFd[0] = fd
                latch.countDown()
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                activity.startForegroundService(intent)
            } else {
                activity.startService(intent)
            }

            Thread {
                val ok = try {
                    latch.await(8, TimeUnit.SECONDS)
                } catch (_: InterruptedException) {
                    false
                }
                TauriVpnService.onEstablished = null
                if (ok && resultFd[0] >= 0) {
                    ret.put("fd", resultFd[0])
                    invoke.resolve(ret)
                } else {
                    invoke.reject("VPN 服务启动超时")
                }
            }.start()
        }
    }

    /** 关闭 VpnService 并回收 TUN。 */
    @Command
    fun stopVpn(invoke: Invoke) {
        activity.runOnUiThread {
            TauriVpnService.self?.onRevoke()
            activity.stopService(Intent(activity, TauriVpnService::class.java))
            invoke.resolve(JSObject())
        }
    }

    @Command
    fun getVpnStatus(invoke: Invoke) {
        val ret = JSObject()
        ret.put("running", TauriVpnService.self != null)
        ret.put("ipv4Addr", TauriVpnService.ipv4Addr)
        ret.put("routes", TauriVpnService.routes)
        ret.put("dns", TauriVpnService.dns)
        invoke.resolve(ret)
    }
}
