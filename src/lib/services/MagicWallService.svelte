<script module>
/** 魔法墙控制器（Svelte 5 模块）。 */
/**
 * MagicWall 控制器服务 —— 移植自：
 * - core/database/dao/magic_wall_dao.dart（DAO → db.js 表 mw_rules/mw_groups/mw_events）
 * - features/magic_wall/services/magic_wall_store.dart（加载/孤儿迁移/打包/事件/路径回写）
 * - features/magic_wall/services/magic_wall_engine.dart（Rust 引擎同步）
 * - features/magic_wall/services/process_monitor.dart + process_path_resolver.dart
 *   （进程自动监听，仅 Windows 启动；浏览器模式 isWindows()=false 直接不启用）
 * - features/magic_wall/services/magic_wall_controller.dart（页面编排门面）
 *
 * 页面用法：读取 mwController.isRunning / groups / activeRulesCount，
 * 调用 loadData / checkStatus / toggleEngine / addRule / ... ；snackbar 由本服务统一弹出
 * （标题固定 '成功' / '错误'，对应原版页面的 onSuccess/onError 回调）。
 */
import * as db from '../db/Db.svelte';
import * as rust from '../rust/Api.svelte';
import { uuidV4 } from '../utils/Display.svelte';
import { isWindows } from '../utils/Platform.svelte';
import { snackSuccess, snackError } from '../components/common/SnackbarStore.svelte';

const RULES_TABLE = 'mw_rules';
const GROUPS_TABLE = 'mw_groups';
const EVENTS_TABLE = 'mw_events';

/** 异常文本（对应 Dart '$e' 拼接） */
function errText(e) {
  if (e instanceof Error) return e.message;
  return String(e ?? '');
}

/* ================================================================
 *  DAO —— 对应 magic_wall_dao.dart（行结构 = 模型字段平铺 + id 主键）
 * ================================================================ */

/** 规则行归一化（字段默认值对应 MagicWallRuleModel 的初始值） */
function normalizeRule(row) {
  return {
    id: row.id ?? null,
    ruleId: row.ruleId ?? '',
    groupId: row.groupId ?? '',
    name: row.name ?? '',
    enabled: row.enabled ?? true,
    action: row.action ?? 'block',
    protocol: row.protocol ?? 'both',
    direction: row.direction ?? 'both',
    appPath: row.appPath ?? null,
    remoteIp: row.remoteIp ?? null,
    localIp: row.localIp ?? null,
    remotePort: row.remotePort ?? null,
    localPort: row.localPort ?? null,
    description: row.description ?? null,
    createdAt: row.createdAt ?? null,
    updatedAt: row.updatedAt ?? null,
    priority: row.priority ?? 0,
  };
}

/** 规则组行归一化（字段默认值对应 MagicWallGroupModel 的初始值） */
function normalizeGroup(row) {
  return {
    id: row.id ?? null,
    groupId: row.groupId ?? '',
    name: row.name ?? '',
    processName: row.processName ?? '',
    enabled: row.enabled ?? false,
    autoManage: row.autoManage ?? true,
    createdAt: row.createdAt ?? null,
    updatedAt: row.updatedAt ?? null,
  };
}

/** priority 降序（并列按 id 升序保持稳定，对应 Isar sortByPriorityDesc） */
function sortRules(rules) {
  return [...rules].sort(
    (a, b) => b.priority - a.priority || (a.id ?? 0) - (b.id ?? 0),
  );
}

async function daoGetAllRulesSorted() {
  return sortRules((await db.listAll(RULES_TABLE)).map(normalizeRule));
}

async function daoGetRulesByGroup(groupId) {
  return sortRules(
    (await db.listAll(RULES_TABLE))
      .map(normalizeRule)
      .filter((rule) => rule.groupId === groupId),
  );
}

/** 新增规则（Isar put 语义：写库后回写自增主键到模型） */
async function daoAddRule(model) {
  const saved = await db.putRow(RULES_TABLE, { ...model });
  model.id = saved.id;
  return saved;
}

async function daoUpdateRule(model) {
  model.updatedAt = Date.now();
  return db.putRow(RULES_TABLE, { ...model });
}

async function daoDeleteRule(id) {
  await db.deleteRow(RULES_TABLE, id);
}

async function daoToggleRule(id) {
  const row = (await db.listAll(RULES_TABLE)).find((r) => r.id === id);
  if (!row) return false;
  const rule = normalizeRule(row);
  rule.enabled = !rule.enabled;
  rule.updatedAt = Date.now();
  await db.putRow(RULES_TABLE, { ...rule });
  return true;
}

async function daoAddRules(models) {
  const saved = await db.putMany(RULES_TABLE, models.map((m) => ({ ...m })));
  models.forEach((m, i) => {
    m.id = saved[i].id;
  });
  return saved;
}

async function daoGetAllGroupsSorted() {
  const rows = (await db.listAll(GROUPS_TABLE)).map(normalizeGroup);
  // Isar sortByName：按字符串逐字符比较
  return rows.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
}

async function daoAddGroup(model) {
  const saved = await db.putRow(GROUPS_TABLE, { ...model });
  model.id = saved.id;
  return saved;
}

async function daoUpdateGroup(model) {
  model.updatedAt = Date.now();
  return db.putRow(GROUPS_TABLE, { ...model });
}

async function daoToggleGroup(groupId) {
  const row = (await db.listAll(GROUPS_TABLE)).find((r) => r.groupId === groupId);
  if (!row) return false;
  const group = normalizeGroup(row);
  group.enabled = !group.enabled;
  group.updatedAt = Date.now();
  await db.putRow(GROUPS_TABLE, { ...group });
  return true;
}

/** 删除规则组（连同组内规则一起删除，对应 deleteMagicWallGroup） */
async function daoDeleteGroup(groupId) {
  const row = (await db.listAll(GROUPS_TABLE)).find((r) => r.groupId === groupId);
  if (!row) return false;
  const ruleRows = (await db.listAll(RULES_TABLE)).filter((r) => r.groupId === groupId);
  for (const rule of ruleRows) {
    await db.deleteRow(RULES_TABLE, rule.id);
  }
  await db.deleteRow(GROUPS_TABLE, row.id);
  return true;
}

/* ================================================================
 *  Store —— 对应 magic_wall_store.dart
 * ================================================================ */

/** 加载组/规则；必要时把孤儿规则（groupId 为空）迁移进新组 */
async function loadGroupsAndRules() {
  const rules = await daoGetAllRulesSorted();
  const groupsList = await daoGetAllGroupsSorted();

  const hasOrphan = rules.some((rule) => !rule.groupId);
  if (groupsList.length === 0 || hasOrphan) {
    await migrateLegacyData(rules, groupsList);
    return {
      groups: await daoGetAllGroupsSorted(),
      rules: await daoGetAllRulesSorted(),
    };
  }

  return { groups: groupsList, rules };
}

/** 孤儿规则迁移：按规则名（或 appPath 文件名）建 '导入规则组'，重名 baseName(2)(3)... */
async function migrateLegacyData(rules, groupsList) {
  const now = Date.now();

  const orphanRules = rules.filter((rule) => !rule.groupId);
  if (orphanRules.length === 0) {
    return;
  }

  const existingNames = new Set(groupsList.map((g) => g.name));
  const createdGroups = [];
  const provisionalGroups = new Map();

  const deriveName = (rule) => {
    if (rule.name) {
      return rule.name;
    }
    if (rule.appPath) {
      const sanitized = rule.appPath.split(/[\\/]/).pop();
      if (sanitized) {
        return sanitized;
      }
    }
    return '导入规则组';
  };

  const ensureUniqueName = (baseName, usedNames) => {
    let name = baseName;
    let index = 1;
    while (usedNames.has(name)) {
      index += 1;
      name = `${baseName}(${index})`;
    }
    usedNames.add(name);
    return name;
  };

  const usedNames = new Set(existingNames);

  const obtainGroup = (baseName) => {
    const existing = provisionalGroups.get(baseName);
    if (existing) {
      return existing;
    }
    const uniqueName = ensureUniqueName(baseName, usedNames);
    const group = {
      id: null,
      groupId: uuidV4(),
      name: uniqueName,
      processName: '',
      enabled: false,
      autoManage: false,
      createdAt: now,
      updatedAt: now,
    };
    provisionalGroups.set(baseName, group);
    createdGroups.push(group);
    return group;
  };

  for (const rule of orphanRules) {
    const baseName = deriveName(rule);
    const group = obtainGroup(baseName);
    rule.groupId = group.groupId;
    if (rule.createdAt == null) rule.createdAt = now;
    rule.updatedAt = now;
  }

  for (const group of createdGroups) {
    await daoAddGroup(group);
  }

  await daoAddRules(orphanRules);
}

/** 组 + 组内规则打包（对应 buildBundles） */
function buildBundles(groupsList, rules) {
  const grouped = new Map();
  for (const rule of rules) {
    if (!grouped.has(rule.groupId)) grouped.set(rule.groupId, []);
    grouped.get(rule.groupId).push(rule);
  }
  return groupsList.map((group) => ({
    group,
    rules: grouped.has(group.groupId) ? [...grouped.get(group.groupId)] : [],
  }));
}

/** 事件日志（对应 store.recordEvent，失败仅 debugPrint） */
async function recordEventLog(targetType, targetId, action, message) {
  try {
    await db.putRow(EVENTS_TABLE, {
      targetType,
      targetId,
      action,
      message: message ?? null,
      timestamp: Date.now(),
    });
  } catch (e) {
    console.warn('记录事件失败:', errText(e));
  }
}

/** 把组内不完整 appPath（仅进程名）回写为完整路径（对应 persistAppPathsForGroup） */
async function persistAppPathsForGroup(rules, groupAppPath) {
  for (const rule of rules) {
    const needsUpdate =
      !rule.appPath ||
      (!rule.appPath.includes('\\') && !rule.appPath.includes('/'));
    if (needsUpdate) {
      await daoUpdateRule({ ...rule, appPath: groupAppPath });
    }
  }
}

/* ================================================================
 *  Controller —— 对应 magic_wall_controller.dart（含 engine/monitor 逻辑）
 * ================================================================ */

class MagicWallController {
  /* signals */
  isRunning = $state(false);
  groups = $state([]); // List<MagicWallGroupBundle>
  activeRulesCount = $state(0);

  /* 页面回调：组数据应用后通知 UI 清理本地折叠状态（onValidGroupIdsChanged） */
  onValidGroupIdsChanged = null;

  /* 进程监控（仅 Windows；浏览器模式不启动） */
  _monitorTimer = null;
  _isCheckingProcesses = false;
  _processActive = {}; // groupId -> bool
  _processExecutablePaths = {}; // groupId -> 完整可执行路径缓存

  /* ---------------- 数据加载 ---------------- */

  async loadData() {
    try {
      const loaded = await loadGroupsAndRules();
      this._applyGroupData(loaded.groups, loaded.rules);
    } catch (e) {
      this._showError(`加载配置失败: ${errText(e)}`);
    }
  }

  _applyGroupData(groupsList, rules) {
    const bundles = buildBundles(groupsList, rules);
    this.groups = bundles;
    const validGroupIds = new Set(bundles.map((b) => b.group.groupId));
    this._pruneCaches(validGroupIds);
    this.onValidGroupIdsChanged?.(validGroupIds);
    this._updateActiveCount();
  }

  _updateActiveCount() {
    let count = 0;
    for (const bundle of this.groups) {
      if (bundle.group.enabled) {
        count += bundle.rules.filter((rule) => rule.enabled).length;
      }
    }
    this.activeRulesCount = count;
  }

  async recordEvent(targetType, targetId, action, message = null) {
    await recordEventLog(targetType, targetId, action, message);
  }

  async checkStatus() {
    try {
      const status = await rust.getMagicWallStatus();
      this.isRunning = status.isRunning;
      this.activeRulesCount = status.activeRules;
    } catch (e) {
      console.warn('检查状态失败:', errText(e));
    }
  }

  /* ---------------- Rust 规则转换（对应 engine.convertToRustRule） ---------------- */

  convertToRustRule(model, fallbackAppPath = null) {
    let resolvedAppPath = null;
    // 优先使用 fallbackAppPath（解析出的完整路径）
    if (fallbackAppPath) {
      resolvedAppPath = fallbackAppPath;
    } else if (model.appPath) {
      resolvedAppPath = model.appPath;
    }

    return {
      id: model.ruleId,
      name: model.name,
      enabled: model.enabled,
      action: model.action,
      protocol: model.protocol,
      direction: model.direction,
      appPath: resolvedAppPath,
      remoteIp: model.remoteIp ?? null,
      localIp: model.localIp ?? null,
      remotePort: model.remotePort ?? null,
      localPort: model.localPort ?? null,
      description: model.description ?? null,
      createdAt: model.createdAt ?? null,
    };
  }

  /* ---------------- 引擎启停 ---------------- */

  /** 同步所有启用组的启用规则到 Rust 层，然后启动引擎（对应 startAndSyncRules） */
  async _startEngineAndSyncRules() {
    for (const bundle of this.groups.filter((b) => b.group.enabled)) {
      const groupExecutable = await this.resolveGroupAppPath(bundle.group);

      // 更新规则的 appPath 为完整路径
      if (groupExecutable) {
        await persistAppPathsForGroup(bundle.rules, groupExecutable);
      }

      // 重新加载更新后的规则
      const updatedRules = await daoGetRulesByGroup(bundle.group.groupId);
      for (const rule of updatedRules.filter((r) => r.enabled)) {
        try {
          await rust.addMagicWallRule(this.convertToRustRule(rule, groupExecutable));
        } catch (e) {
          console.warn(`⚠️  应用规则失败: ${rule.name}, 错误: ${errText(e)}`);
        }
      }
    }

    // 启动魔法墙引擎，Rust 层会自动应用 RULE_STORE 中的所有规则
    await rust.startMagicWall();
    this.isRunning = true;
  }

  async toggleEngine() {
    try {
      if (this.isRunning) {
        await rust.stopMagicWall();
        this.isRunning = false;
        await this.recordEvent('engine', 'engine', 'off', '手动操作');
        this._showSuccess('魔法墙已停止');
      } else {
        await this._startEngineAndSyncRules();
        await this.recordEvent('engine', 'engine', 'on', '手动操作');
        this._showSuccess('魔法墙已启动');
      }
    } catch (e) {
      this._showError(`操作失败: ${errText(e)}`);
      this.checkStatus(); // 发生错误时刷新状态，避免 UI 状态不同步
    }
  }

  /* ---------------- 引擎同步辅助（对应 magic_wall_engine.dart） ---------------- */

  _isGroupEnabled(bundles, groupId) {
    return bundles.some((b) => b.group.groupId === groupId && b.group.enabled);
  }

  _findBundle(bundles, groupId) {
    return bundles.find((b) => b.group.groupId === groupId) ?? null;
  }

  async _removeRuleQuietly(ruleId, ruleName, contextLabel = '移除规则失败') {
    try {
      await rust.removeMagicWallRule(ruleId);
    } catch (e) {
      console.warn(`⚠️  ${contextLabel}: ${ruleName}, 错误: ${errText(e)}`);
    }
  }

  /** 推送单个启用组的全部启用规则（对应 syncEnabledGroupToEngine） */
  async _syncEnabledGroupToEngine(bundle) {
    const groupAppPath = await this.resolveGroupAppPath(bundle.group);

    if (groupAppPath) {
      await persistAppPathsForGroup(bundle.rules, groupAppPath);
    }

    const updatedRules = await daoGetRulesByGroup(bundle.group.groupId);
    for (const rule of updatedRules.filter((r) => r.enabled)) {
      try {
        await rust.addMagicWallRule(this.convertToRustRule(rule, groupAppPath));
      } catch (e) {
        console.warn(`⚠️  添加规则失败: ${rule.name}, 错误: ${errText(e)}`);
      }
    }
  }

  /** 从引擎移除组的全部规则（按 ruleId 去重，对应 removeGroupRulesFromEngine） */
  async _removeGroupRulesFromEngine(bundle) {
    const uniqueRuleIds = new Set();
    for (const rule of bundle.rules) {
      if (uniqueRuleIds.has(rule.ruleId)) {
        continue;
      }
      uniqueRuleIds.add(rule.ruleId);

      try {
        await rust.removeMagicWallRule(rule.ruleId);
      } catch (e) {
        console.warn(`⚠️  移除规则失败: ${rule.name}, 错误: ${errText(e)}`);
      }
    }
  }

  /* ---------------- 规则 CRUD ---------------- */

  /** 新增规则（对话框已返回模型）：解析 groupId → 回写组路径 → 入库 → 全量重读 → 引擎同步 */
  async addRule(rule, { targetGroupId, groupsList }) {
    try {
      const resolvedGroupId = !rule.groupId ? targetGroupId : rule.groupId;
      rule.groupId = resolvedGroupId;
      const targetGroup =
        groupsList.find((g) => g.groupId === resolvedGroupId) ?? null;
      const groupAppPath = targetGroup
        ? await this.resolveGroupAppPath(targetGroup)
        : null;
      rule.appPath = groupAppPath;
      await daoAddRule(rule);

      // 确保规则有完整路径
      const updatedRule = await this._ensureRuleHasCompletePath(rule, groupAppPath);
      await this.loadData();
      // 引擎同步：仅 isRunning && 组 enabled && 规则 enabled 时 add
      if (
        this.isRunning &&
        this._isGroupEnabled(this.groups, resolvedGroupId) &&
        updatedRule.enabled
      ) {
        try {
          await rust.addMagicWallRule(
            this.convertToRustRule(updatedRule, groupAppPath),
          );
        } catch (e) {
          console.warn(
            `⚠️  添加规则到防火墙失败: ${updatedRule.name}, 错误: ${errText(e)}`,
          );
          throw e;
        }
      }

      this._showSuccess('规则已添加');
    } catch (e) {
      this._showError(`添加规则失败: ${errText(e)}`);
    }
  }

  async updateRule(updated, { groupsList }) {
    try {
      const targetGroup =
        groupsList.find((g) => g.groupId === updated.groupId) ?? null;
      const groupAppPath = targetGroup
        ? await this.resolveGroupAppPath(targetGroup)
        : null;
      if (groupAppPath) {
        updated.appPath = groupAppPath;
      }
      await daoUpdateRule(updated);

      // 确保规则有完整路径
      const updatedRuleWithPath = await this._ensureRuleHasCompletePath(
        updated,
        groupAppPath,
      );
      await this.loadData();
      // 引擎同步：运行中时 enabled+组enabled → update，否则移除
      if (this.isRunning) {
        const groupEnabled = this._isGroupEnabled(
          this.groups,
          updatedRuleWithPath.groupId,
        );
        if (updatedRuleWithPath.enabled && groupEnabled) {
          await rust.updateMagicWallRule(
            this.convertToRustRule(
              updatedRuleWithPath,
              groupAppPath ?? updatedRuleWithPath.appPath,
            ),
          );
        } else {
          await this._removeRuleQuietly(
            updatedRuleWithPath.ruleId,
            updatedRuleWithPath.name,
          );
        }
      }

      this._showSuccess('规则已更新');
    } catch (e) {
      this._showError(`更新规则失败: ${errText(e)}`);
    }
  }

  async deleteRule(rule) {
    try {
      // 删除类操作：先引擎后 DB
      if (
        this.isRunning &&
        rule.enabled &&
        this._isGroupEnabled(this.groups, rule.groupId)
      ) {
        await this._removeRuleQuietly(
          rule.ruleId,
          rule.name,
          '从防火墙删除规则失败',
        );
        // 继续删除数据库记录
      }

      await daoDeleteRule(rule.id);
      await this.loadData();
      this._showSuccess('规则已删除');
    } catch (e) {
      this._showError(`删除规则失败: ${errText(e)}`);
    }
  }

  async toggleRule(rule) {
    try {
      await daoToggleRule(rule.id);
      await this.loadData();
      if (!this.isRunning) return;

      const bundle = this._findBundle(this.groups, rule.groupId);
      if (!bundle) return;

      let updatedRule =
        bundle.rules.find((item) => item.ruleId === rule.ruleId) ?? rule;

      if (bundle.group.enabled && updatedRule.enabled) {
        const groupAppPath = await this.resolveGroupAppPath(bundle.group);
        // 确保规则有完整路径
        updatedRule = await this._ensureRuleHasCompletePath(
          updatedRule,
          groupAppPath,
        );
        await rust.addMagicWallRule(
          this.convertToRustRule(updatedRule, groupAppPath),
        );
      } else {
        // 即使删除失败也继续，可能规则已经不存在了
        await this._removeRuleQuietly(updatedRule.ruleId, updatedRule.name);
      }
    } catch (e) {
      this._showError(`切换规则状态失败: ${errText(e)}`);
    }
  }

  /* ---------------- 规则组 CRUD ---------------- */

  async toggleGroup(group) {
    try {
      await daoToggleGroup(group.groupId);
      await this.loadData();
      if (!this.isRunning) return;

      const bundle = this._findBundle(this.groups, group.groupId);
      if (!bundle) return;

      if (bundle.group.enabled) {
        await this.recordEvent('group', bundle.group.groupId, 'on', '手动操作');
        await this._syncEnabledGroupToEngine(bundle);
      } else {
        await this.recordEvent('group', bundle.group.groupId, 'off', '手动操作');
        await this._removeGroupRulesFromEngine(bundle);
      }
    } catch (e) {
      this._showError(`切换规则组失败: ${errText(e)}`);
    }
  }

  async addGroup(group) {
    try {
      await daoAddGroup(group);
      await this.loadData();
      this._showSuccess('规则组已添加');
    } catch (e) {
      this._showError(`添加规则组失败: ${errText(e)}`);
    }
  }

  async updateGroup(group) {
    try {
      await daoUpdateGroup(group);
      await this.loadData();
      this._showSuccess('规则组已更新');
    } catch (e) {
      this._showError(`更新规则组失败: ${errText(e)}`);
    }
  }

  async deleteGroup(group) {
    try {
      // 删除类操作：先引擎后 DB（运行中且组启用时移除组内规则，不做静默兜底）
      const bundle = this._findBundle(this.groups, group.groupId);
      if (this.isRunning && bundle && bundle.group.enabled) {
        for (const rule of bundle.rules) {
          await rust.removeMagicWallRule(rule.ruleId);
        }
      }

      await daoDeleteGroup(group.groupId);
      await this.loadData();
      this._showSuccess('规则组已删除');
    } catch (e) {
      this._showError(`删除规则组失败: ${errText(e)}`);
    }
  }

  /* ---------------- 进程路径解析（对应 process_path_resolver.dart） ---------------- */

  /** 解析组的可执行文件路径：缓存 → 组内规则 appPath → 进程查询 */
  async resolveGroupAppPath(group) {
    if (!group.processName || !group.processName.trim()) {
      return null;
    }

    const cached = this._processExecutablePaths[group.groupId];
    if (cached) {
      return cached;
    }

    for (const bundle of this.groups) {
      if (bundle.group.groupId === group.groupId) {
        for (const rule of bundle.rules) {
          if (rule.appPath) {
            this._processExecutablePaths[group.groupId] = rule.appPath;
            return rule.appPath;
          }
        }
        break;
      }
    }

    const path = await this.getProcessExecutablePath(group.processName);
    if (path) {
      this._processExecutablePaths[group.groupId] = path;
      return path;
    }
    return null;
  }

  /**
   * 原版经 PowerShell `Get-Process` 解析进程完整路径（仅 Windows）。
   * Tauri/浏览器端暂无进程桥接 API（rust/Api.svelte 未暴露对应命令），恒返回 null——
   * 自动监听与路径修复逻辑保持移植结构，待 Rust 侧补充命令后即恢复行为。
   */
  async getProcessExecutablePath(processName) {
    const trimmed = (processName || '').trim();
    if (!trimmed) {
      return null;
    }
    return null;
  }

  /** 单条规则 appPath 补全（对应 ensureRuleHasCompletePath） */
  async _ensureRuleHasCompletePath(rule, groupAppPath) {
    const currentPath = rule.appPath;
    if (
      currentPath &&
      (currentPath.includes('\\') || currentPath.includes('/'))
    ) {
      return rule;
    }

    let resolvedPath = groupAppPath ?? null;
    if (!resolvedPath) {
      for (const bundle of this.groups) {
        if (bundle.group.groupId === rule.groupId) {
          resolvedPath = await this.resolveGroupAppPath(bundle.group);
          break;
        }
      }
    }

    if (!resolvedPath) {
      return rule;
    }

    const updated = { ...rule, appPath: resolvedPath };
    await daoUpdateRule(updated);
    return updated;
  }

  /** 修复库中不完整的 appPath（仅进程名而非完整路径）——仅 Windows 生效 */
  async fixIncompleteAppPaths() {
    if (!isWindows()) {
      return;
    }

    try {
      let anyUpdated = false;
      const groups = this.groups;

      for (const bundle of groups) {
        if (!bundle.group.processName.trim()) {
          continue;
        }

        const needsFix = bundle.rules.some(
          (rule) =>
            !rule.appPath ||
            (!rule.appPath.includes('\\') && !rule.appPath.includes('/')),
        );
        if (!needsFix) {
          continue;
        }

        const executablePath = await this.getProcessExecutablePath(
          bundle.group.processName,
        );
        if (!executablePath) {
          continue;
        }

        for (const rule of bundle.rules) {
          if (
            !rule.appPath ||
            (!rule.appPath.includes('\\') && !rule.appPath.includes('/'))
          ) {
            await daoUpdateRule({ ...rule, appPath: executablePath });
            anyUpdated = true;
          }
        }
      }

      if (anyUpdated) {
        await this.loadData();
      }
    } catch (e) {
      console.warn('修复应用路径失败:', errText(e));
    }
  }

  /* ---------------- 进程监控（对应 process_monitor.dart，仅 Windows） ---------------- */

  startProcessMonitor() {
    this.stopProcessMonitor();
    this._monitorTimer = setInterval(() => {
      this._checkProcesses();
    }, 5000);
    this._checkProcesses();
  }

  stopProcessMonitor() {
    if (this._monitorTimer) {
      clearInterval(this._monitorTimer);
      this._monitorTimer = null;
    }
  }

  _pruneCaches(validGroupIds) {
    for (const key of Object.keys(this._processExecutablePaths)) {
      if (!validGroupIds.has(key)) delete this._processExecutablePaths[key];
    }
    for (const key of Object.keys(this._processActive)) {
      if (!validGroupIds.has(key)) delete this._processActive[key];
    }
  }

  async _checkProcesses() {
    if (!isWindows()) {
      return;
    }

    if (this._isCheckingProcesses) {
      return;
    }

    const targets = this.groups.filter(
      (bundle) => bundle.group.autoManage && bundle.group.processName,
    );

    if (targets.length === 0) {
      this._processActive = {};
      this._processExecutablePaths = {};
      return;
    }

    this._isCheckingProcesses = true;
    let requireReload = false;
    try {
      for (const bundle of targets) {
        const executable = await this.getProcessExecutablePath(
          bundle.group.processName,
        );
        const running = !!executable;
        const wasRunning = this._processActive[bundle.group.groupId] ?? false;

        if (running) {
          this._processExecutablePaths[bundle.group.groupId] = executable;
        } else {
          delete this._processExecutablePaths[bundle.group.groupId];
        }

        if (running && !wasRunning) {
          const changed = await this._handleProcessStarted(bundle);
          requireReload = requireReload || changed;
          this._processActive[bundle.group.groupId] = true;
        } else if (!running && wasRunning) {
          const changed = await this._handleProcessStopped(bundle);
          requireReload = requireReload || changed;
          this._processActive[bundle.group.groupId] = false;
        }
      }
    } finally {
      this._isCheckingProcesses = false;
    }

    if (requireReload) {
      await this.loadData();
    }
  }

  async _handleProcessStarted(bundle) {
    let stateChanged = false;
    const group = bundle.group;

    if (!this.isRunning) {
      try {
        await this._startEngineAndSyncRules();
        await this.recordEvent(
          'engine',
          'engine',
          'auto_on',
          `进程 ${group.processName}`,
        );
      } catch (e) {
        this._showError(`自动启动魔法墙失败: ${errText(e)}`);
      }
    }

    if (!group.enabled) {
      await daoUpdateGroup({ ...group, enabled: true });
      stateChanged = true;
    }

    const rules = await daoGetRulesByGroup(group.groupId);
    const executablePath = await this.resolveGroupAppPath(group);

    // 更新规则的 appPath 为完整可执行文件路径
    if (executablePath) {
      for (const rule of rules) {
        const needsUpdate =
          !rule.appPath ||
          (!rule.appPath.includes('\\') && !rule.appPath.includes('/'));
        if (needsUpdate) {
          await daoUpdateRule({ ...rule, appPath: executablePath });
        }
      }
    }

    // 重新加载更新后的规则（Set 去重，避免重复添加）
    const updatedRules = await daoGetRulesByGroup(group.groupId);
    const addedRuleIds = new Set();
    for (const rule of updatedRules.filter((r) => r.enabled)) {
      if (addedRuleIds.has(rule.ruleId)) {
        continue;
      }
      addedRuleIds.add(rule.ruleId);

      try {
        await rust.addMagicWallRule(
          this.convertToRustRule(rule, executablePath),
        );
      } catch (e) {
        console.warn(`⚠️  添加规则失败: ${rule.name}, 错误: ${errText(e)}`);
        // 如果规则已存在，忽略错误继续
        if (!String(errText(e)).includes('已存在')) {
          throw e;
        }
      }
    }

    await this.recordEvent(
      'group',
      group.groupId,
      'auto_on',
      `进程 ${group.processName}`,
    );

    return stateChanged;
  }

  async _handleProcessStopped(bundle) {
    let stateChanged = false;
    const group = bundle.group;

    const rules = await daoGetRulesByGroup(group.groupId);

    // Set 去重，移除组内全部规则
    const uniqueRuleIds = new Set();
    for (const rule of rules) {
      if (uniqueRuleIds.has(rule.ruleId)) {
        continue;
      }
      uniqueRuleIds.add(rule.ruleId);

      try {
        await rust.removeMagicWallRule(rule.ruleId);
      } catch (e) {
        console.warn(`⚠️  移除规则失败: ${rule.name}, 错误: ${errText(e)}`);
        // 继续移除其他规则
      }
    }

    if (group.enabled) {
      await daoUpdateGroup({ ...group, enabled: false });
      stateChanged = true;
    }

    await this.recordEvent(
      'group',
      group.groupId,
      'auto_off',
      `进程 ${group.processName}`,
    );

    let otherGroupEnabled = false;
    for (const item of this.groups) {
      if (item.group.groupId === group.groupId) {
        continue;
      }
      if (item.group.enabled) {
        otherGroupEnabled = true;
        break;
      }
    }

    if (!otherGroupEnabled && this.isRunning) {
      try {
        await rust.stopMagicWall();
        this.isRunning = false;
        await this.recordEvent(
          'engine',
          'engine',
          'auto_off',
          `进程 ${group.processName}`,
        );
      } catch (e) {
        this._showError(`自动停止魔法墙失败: ${errText(e)}`);
      }
    }

    return stateChanged;
  }

  /* ---------------- snackbar（对应页面 onSuccess/onError） ---------------- */

  _showSuccess(message) {
    snackSuccess('成功', message);
  }

  _showError(message) {
    snackError('错误', message);
  }
}

/** 单例控制器（页面直接导入使用） */
export const mwController = new MagicWallController();
</script>
