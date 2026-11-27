// 并行执行 Vite 与 Tauri/Rust；TAURI_CONFIG 只覆盖 beforeBuildCommand，保留 CLI 配置合并。
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '../..');
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx';
const children = new Set();

// Tauri 启动时会检查 frontendDist；占位文件只用于并行启动，Vite 会清空并重建 dist。
const dist = resolve(root, 'dist');
mkdirSync(dist, { recursive: true });
const placeholder = resolve(dist, 'index.html');
if (!existsSync(placeholder)) writeFileSync(placeholder, '<!doctype html><title>Astral build</title>');

let inheritedConfig = {};
try { inheritedConfig = JSON.parse(process.env.TAURI_CONFIG || '{}'); } catch { /* 由 Tauri 报告原配置错误 */ }
const env = {
  ...process.env,
  RUSTUP_TOOLCHAIN: 'nightly',
  RUSTFLAGS: `${process.env.RUSTFLAGS || ''} -Z threads=16`.trim(),
  TAURI_CONFIG: JSON.stringify({
    ...inheritedConfig,
    build: { ...(inheritedConfig.build || {}), beforeBuildCommand: '' },
  }),
};

function run(command, args, label) {
  const child = spawn(command, args, {
    cwd: root,
    env,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  children.add(child);
  return new Promise((resolvePromise, rejectPromise) => {
    child.once('error', rejectPromise);
    child.once('close', (code, signal) => {
      children.delete(child);
      if (code === 0) resolvePromise();
      else rejectPromise(new Error(`${label} 失败（code=${code ?? 'null'}, signal=${signal ?? 'none'}）`));
    });
  });
}

console.log('[parallel] Vite 与 Tauri/Rust Nightly 同时启动');
try {
  await Promise.all([
    run(npm, ['run', 'build'], '前端构建'),
    run(npx, ['tauri', ...process.argv.slice(2)], 'Rust/Tauri 构建'),
  ]);
  console.log('[parallel] 前端与 Rust/Tauri 均已完成');
} catch (error) {
  for (const child of children) child.kill();
  console.error(`[parallel] ${error.message}`);
  process.exitCode = 1;
}
