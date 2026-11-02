/**
 * 数据持久层 —— 替代 Flutter 端的 Isar 数据库。
 *
 * 模式一（Tauri 运行时）：tauri-plugin-sql + SQLite，库文件 astral.db。
 * 模式二（浏览器预览）：localStorage JSON 集合，接口与 SQL 模式完全一致。
 *
 * 表结构统一为 (id INTEGER PRIMARY KEY, data TEXT(JSON))：
 * 与原 Isar 集合字段一一对应（Room/ServerMod/AllSettings/NetConfig/
 * ThemeSettings/MagicWall*），避免字段映射失真。
 */
import { isTauri } from '../utils/platform.js';

const TABLES = [
  'rooms',
  'servers',
  'all_settings',
  'net_config',
  'theme_settings',
  'mw_rules',
  'mw_groups',
  'mw_events',
];

let sqlDb = null; // Tauri SQL 连接
let mode = 'browser';

export async function initDb() {
  if (isTauri()) {
    try {
      const { Database } = await import('@tauri-apps/plugin-sql');
      sqlDb = await Database.load('sqlite:astral.db');
      for (const t of TABLES) {
        await sqlDb.execute(
          `CREATE TABLE IF NOT EXISTS ${t} (id INTEGER PRIMARY KEY, data TEXT NOT NULL)`
        );
      }
      mode = 'sqlite';
      return;
    } catch (e) {
      console.warn('SQLite 初始化失败，回退 localStorage 模式:', e);
    }
  }
  mode = 'browser';
  // localStorage 模式无需建表
}

export function dbMode() {
  return mode;
}

/* ---------------- 统一表操作 API ---------------- */

/** 读取全表 */
export async function listAll(table) {
  if (mode === 'sqlite') {
    const rows = await sqlDb.select(`SELECT id, data FROM ${table}`);
    return rows.map((r) => ({ ...JSON.parse(r.data), id: r.id }));
  }
  return JSON.parse(localStorage.getItem(`astral-db-${table}`) || '[]');
}

/** 按 id 读取 */
export async function getById(table, id) {
  const rows = await listAll(table);
  return rows.find((r) => r.id === id) || null;
}

/**
 * 写入一行（upsert）。row 无 id 时自动分配自增 id。
 * @returns {Promise<object>} 带id的行
 */
export async function putRow(table, row) {
  const record = { ...row };
  if (mode === 'sqlite') {
    if (record.id == null) {
      const result = await sqlDb.execute(
        `INSERT INTO ${table} (data) VALUES ($1)`,
        [JSON.stringify({ ...record, id: 0 })]
      );
      record.id = Number(result.lastInsertId);
    }
    await sqlDb.execute(
      `INSERT OR REPLACE INTO ${table} (id, data) VALUES ($1, $2)`,
      [record.id, JSON.stringify(record)]
    );
    return record;
  }
  const rows = JSON.parse(localStorage.getItem(`astral-db-${table}`) || '[]');
  if (record.id == null) {
    const maxId = rows.reduce((m, r) => Math.max(m, r.id || 0), 0);
    record.id = maxId + 1;
    rows.push(record);
  } else {
    const idx = rows.findIndex((r) => r.id === record.id);
    if (idx >= 0) rows[idx] = record;
    else rows.push(record);
  }
  localStorage.setItem(`astral-db-${table}`, JSON.stringify(rows));
  return record;
}

/** 批量写入（保持顺序） */
export async function putMany(table, rows) {
  const out = [];
  for (const row of rows) {
    out.push(await putRow(table, row));
  }
  return out;
}

/** 删除一行 */
export async function deleteRow(table, id) {
  if (mode === 'sqlite') {
    await sqlDb.execute(`DELETE FROM ${table} WHERE id = $1`, [id]);
    return;
  }
  const rows = JSON.parse(localStorage.getItem(`astral-db-${table}`) || '[]');
  localStorage.setItem(
    `astral-db-${table}`,
    JSON.stringify(rows.filter((r) => r.id !== id))
  );
}

/** 清空表 */
export async function clearTable(table) {
  if (mode === 'sqlite') {
    await sqlDb.execute(`DELETE FROM ${table}`);
    return;
  }
  localStorage.removeItem(`astral-db-${table}`);
}

/** 计数 */
export async function countRows(table) {
  const rows = await listAll(table);
  return rows.length;
}
