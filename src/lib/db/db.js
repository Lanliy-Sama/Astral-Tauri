/**
 * 数据持久层 —— 替代 Flutter 端的 Isar 数据库。
 *
 * 使用 tauri-plugin-sql + SQLite，库文件 astral.db。
 * 表结构统一为 (id INTEGER PRIMARY KEY, data TEXT(JSON))：
 * 与原 Isar 集合字段一一对应（Room/ServerMod/AllSettings/NetConfig/
 * ThemeSettings/MagicWall*），避免字段映射失真。
 *
 * 已移除浏览器 localStorage 回退：没有后端时直接报错，不再伪造持久化。
 */
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

let sqlDb = null;

export async function initDb() {
  if (sqlDb) return;
  const { Database } = await import('@tauri-apps/plugin-sql');
  sqlDb = await Database.load('sqlite:astral.db');
  for (const t of TABLES) {
    await sqlDb.execute(
      `CREATE TABLE IF NOT EXISTS ${t} (id INTEGER PRIMARY KEY, data TEXT NOT NULL)`
    );
  }
}

export function dbMode() {
  return sqlDb ? 'sqlite' : 'uninitialized';
}

function requireDb() {
  if (!sqlDb) throw new Error('数据库尚未初始化');
  return sqlDb;
}

/* ---------------- 统一表操作 API ---------------- */

/** 读取全表 */
export async function listAll(table) {
  const rows = await requireDb().select(`SELECT id, data FROM ${table}`);
  return rows.map((r) => ({ ...JSON.parse(r.data), id: r.id }));
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
  const db = requireDb();
  const record = { ...row };
  if (record.id == null) {
    const result = await db.execute(`INSERT INTO ${table} (data) VALUES ($1)`, [
      JSON.stringify({ ...record, id: 0 }),
    ]);
    record.id = Number(result.lastInsertId);
  }
  await db.execute(`INSERT OR REPLACE INTO ${table} (id, data) VALUES ($1, $2)`, [
    record.id,
    JSON.stringify(record),
  ]);
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
  await requireDb().execute(`DELETE FROM ${table} WHERE id = $1`, [id]);
}

/** 清空表 */
export async function clearTable(table) {
  await requireDb().execute(`DELETE FROM ${table}`);
}

/** 计数 */
export async function countRows(table) {
  const rows = await listAll(table);
  return rows.length;
}
