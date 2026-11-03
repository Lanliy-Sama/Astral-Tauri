# EasyTier 版本说明

## 当前锁定版本

`src-tauri/crates/astral-core/Cargo.toml` 目前锁定：

```toml
easytier = { git = "https://github.com/EasyTier/EasyTier.git",
             rev = "8e1d0791421e95dd73533db07463b3afa08495ae", ... }
```

该 commit 对应 EasyTier **2.6.4**，与原 Flutter 版 `rust/Cargo.lock` 中的锁定一致，
是 `astral-core`（`simple.rs` / `p2p.rs` / `nat_test.rs`）实际按照编写的 API 版本。

> 早期工程没有提交 `Cargo.lock`，不锁 `rev` 时 cargo 会解析到 EasyTier main 的最新提交，
> 导致构建结果随上游漂移——这也是必须显式锁定的原因。

## 为什么暂时没有升级到 2.7.0

EasyTier 2.7.0 对实例层做了破坏性重构：

| 2.6.4 | 2.7.0 |
|---|---|
| `easytier::launcher::NetworkInstance` | 已删除，改为 `CoreInstance` + `InstanceManager` |
| `easytier::instance_manager::NetworkInstanceManager` | 整个模块已删除 |
| `NetworkInstance::get_running_info()` | 改为 `node_snapshot()` / `route_snapshots()` / `peer_snapshots()` |
| `NetworkInstance::get_tun_fd_sender()` | 改为 `CoreInstance::attach_tun_fd(fd)` |

因此升级不是改几个编译错误，而是重写 `astral-core` 的实例生命周期层，
并且需要把新的 snapshot 结构重新拼装成前端依赖的 `get_running_info` JSON 形状
（`my_node_info.virtual_ipv4.address.addr` 等）。

## 升级到 2.7.0 的工作清单

1. `astral-core/src/api/simple.rs`
   - 用 `easytier::instance::factory::create_native_instance` 或
     `InstanceManager`（需开启 `management` feature）替换 `NetworkInstance`
   - `set_tun_fd` 改为 `CoreInstance::attach_tun_fd(fd)`
   - `get_running_info` / `get_network_status` 改为基于 snapshot 重新拼装
2. `astral-core/src/api/p2p.rs`
   - 该文件依赖已删除的 `instance_manager`，且未被 `src-tauri` 任何命令引用，
     建议直接删除而不是迁移
3. `astral-core/src/api/nat_test.rs`
   - 复核 `common::stun::{StunInfoCollector, UdpNatTypeDetector}` 在 2.7.0 的签名
4. 全平台重新验证：Windows / Linux / Android 构建 + Android 真机联网

在完成上述迁移之前，**不要把 `rev` 改成 `tag = "v2.7.0"`**，否则 `astral-core` 无法编译。
