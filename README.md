# EasyTier (Astral fork — `Easytier` branch)

Astral-Tauri 使用的 EasyTier 内核 vendored fork。

- 上游: https://github.com/EasyTier/EasyTier `main` @ `3b32aaff`
  （Astral 此前 `branch = "main"` 浮动解析命中的同一 rev，锁定行为）
- 仅包含 Astral 构建闭包所需 crate: `easytier` / `easytier-core` / `easytier-proto`
- `zmij/`: 打了 Cranelift 兼容补丁的 zmij 1.0.23 fork（强制标量回退，
  剔除全部 SSE2/SSE4.1/NEON intrinsic 路径）。主仓通过
  `[patch.crates-io]` 将 serde_json 依赖的 zmij 重定向到这里。

主仓 `src-tauri` 的引用方式:

```toml
easytier      = { git = "https://github.com/Lanliy-Sama/Astral-Tauri", branch = "Easytier", ... }
easytier-core = { git = "https://github.com/Lanliy-Sama/Astral-Tauri", branch = "Easytier", ... }

[patch.crates-io]
zmij = { git = "https://github.com/Lanliy-Sama/Astral-Tauri", branch = "Easytier" }
```
