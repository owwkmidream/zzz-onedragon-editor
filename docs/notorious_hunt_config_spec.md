# 恶名狩猎（`notorious_hunt`）配置规则与外部编辑工具规格

本文档面向 `tools/charge-plan-editor-tauri` 的外部编辑能力开发与维护，目标是在**不启动主程序 GUI** 的情况下，完成恶名狩猎配置的读取、校验、编辑与保存，并保证落盘结果与主程序行为一致。

本文档基于仓库源码的实际行为（以下路径是主要真相源）：
- 配置模型与保存逻辑：`src/zzz_od/application/notorious_hunt/notorious_hunt_config.py`
- 运行语义（字段如何被使用）：`src/zzz_od/application/notorious_hunt/notorious_hunt_app.py`、`src/zzz_od/operation/compendium/notorious_hunt.py`、`src/zzz_od/operation/compendium/notorious_hunt_move.py`
- 应用配置文件路径规则（含旧路径迁移）：`src/one_dragon/base/operation/application/application_config.py`
- YAML 读写行为：`src/one_dragon/base/config/yaml_operator.py`
- “全部 BOSS 列表”（用于动态补齐与校验）：`assets/game_data/compendium_data.yml`
- 预备编队索引与来源：`config/<实例>/team.yml` + `src/zzz_od/config/team_config.py`

---

## 1. 配置文件定位规则（外部工具必须实现）

### 1.1 实例（账号）选择

外部工具需要允许用户选择要编辑的“实例”，来源建议读取：
- `config/one_dragon.yml`：`instance_list` 里的 `idx`（数字）与 `name`（显示用）

实例路径规则中的 `instance_idx` 使用“两位补零”格式：
- `idx = 1` → `01`

### 1.2 组（group_id）选择

主程序的“应用配置”按组分目录保存。默认组为：
- `group_id = "one_dragon"`（见 `src/one_dragon/base/operation/application/application_const.py`）

外部工具建议：
- 默认固定 `group_id=one_dragon`；
- 在 UI 中可保留高级选项以便未来扩展其他组，但 v0 可以不暴露。

### 1.3 恶名狩猎配置文件路径（主路径与历史路径）

**主路径（必须优先读取/写入）**
- `config/{instance_idx}/{group_id}/notorious_hunt.yml`
- 示例：`config/01/one_dragon/notorious_hunt.yml`

**历史路径（仅用于提示、回退读取、或者迁移）**
- `config/{instance_idx}/notorious_hunt.yml`
- 示例：`config/01/notorious_hunt.yml`

**迁移行为说明（非常重要）**
- 主程序 `ApplicationConfig` 会在“主路径不存在且历史路径存在”时，将历史路径文件复制到主路径。
- 如果主路径已存在，历史路径即使存在也不会被使用。

外部工具推荐策略：
1. 默认只编辑主路径。
2. 若主路径不存在但历史路径存在：
   - 在 UI 中提示“检测到历史路径配置”，提供一键“迁移到新路径（复制）”功能；
   - 或者在保存时直接以“主路径”为目标写入（相当于完成迁移）。

### 1.4 修改后何时生效

主程序运行时会对配置对象与 YAML 读取做缓存；外部工具修改文件后：
- 不保证运行中的主程序立刻读到新内容；
- 最稳妥的方式是**关闭主程序 → 修改配置 → 重启主程序**。

外部工具建议：
- 保存后提示用户“如主程序正在运行，建议重启以生效”。

---

## 2. YAML 读写格式约束（外部工具落盘必须遵守）

### 2.1 编码与格式

- 文件编码：`UTF-8`
- 文件格式：YAML

主程序保存 YAML 的方式（源码行为）：
- `yaml.dump(data, allow_unicode=True, sort_keys=False)`（见 `YamlOperator.save()`）

外部工具建议采用一致策略：
- 保留键顺序（不要自动排序 key），避免无意义的 diff；
- 允许中文值不转义；
- 用稳定的缩进（建议 2 空格）；
- 保留末尾换行。

### 2.2 原子写入与备份（强烈建议）

为避免写入中途崩溃导致配置损坏，外部工具建议实现：
1. 写入前备份：`notorious_hunt.yyyymmdd-HHMMSS.bak`
2. 写入临时文件并原子替换（或使用等价机制）

备份建议放在同一目录，方便用户手动回滚。

---

## 3. 顶层结构（NotoriousHuntConfig）

恶名狩猎配置文件对应 `NotoriousHuntConfig`，落盘结构为一个 YAML mapping（字典）。

### 3.1 顶层键集合

顶层仅包含：
- `plan_list: list`

说明：
- 主程序在保存时会重建 `self.data`，只写回 `plan_list`（见 `NotoriousHuntConfig.save()`）。
- 外部工具不应写入任何其他顶层键；即使写入，主程序后续保存也会丢弃。

---

## 4. 计划条目（`plan_list` item）字段说明（严格对齐 GUI）

`plan_list` 中每个元素都是一个 YAML mapping（字典）。

为与 GUI 行为一致，外部工具保存时必须保证每个条目**只包含并且必须包含**以下 10 个键（并按固定顺序落盘，便于 diff）：

1. `tab_name: string`
2. `category_name: string`
3. `mission_type_name: string`
4. `mission_name: null`
5. `level: string`
6. `predefined_team_idx: int`
7. `auto_battle_config: string`
8. `run_times: int`
9. `plan_times: int`
10. `notorious_hunt_buff_num: int`

> 注意：主程序在读取时使用 `ChargePlanItem(**plan_item)` 反序列化，理论上能接受更多键（例如 `plan_id`、`card_num`）。  
> 但为了与恶名狩猎 GUI 的保存结果一致、并避免产生“看似可写但主程序会丢弃”的字段，本工具选择严格限制为上述 10 键。

### 4.1 `tab_name`

- 类型：字符串
- 保存时固定值：`训练`
- 读取兼容：
  - `挑战` → `训练`（快捷手册 Tab 名称历史改动）
  - `作战 + 恶名狩猎` → `训练`（恶名狩猎从作战迁移到训练）

外部工具建议：
- 读取后立即规范化为 `训练`；
- 保存时强制写 `训练`。

### 4.2 `category_name`

- 类型：字符串
- 保存时固定值：`恶名狩猎`

外部工具建议：
- 保存时强制写 `恶名狩猎`（不开放编辑）。

### 4.3 `mission_type_name`

- 类型：字符串
- 语义：BOSS 标识（“这条计划对应哪个 BOSS”）

重要约束（工具必须遵守）：
- 在本工具中：**只读展示，不提供编辑入口**。
- “全部 BOSS 列表”必须动态读取（见第 5 章）。

### 4.4 `mission_name`

- 类型：`null`
- 语义：在恶名狩猎计划中主程序并不依赖该字段来选择 BOSS（BOSS 用 `mission_type_name` 决定）。

外部工具建议：
- 保存时统一写 `null`（避免空字符串等导致“同一条计划”判定出现歧义）。

### 4.5 `level`

- 类型：字符串
- 允许值（必须严格限制为以下中文值之一，大小写与空格都必须一致）：
  - `默认等级`
  - `等级Lv.65`
  - `等级Lv.60`
  - `等级Lv.50`
  - `等级Lv.40`
  - `等级Lv.30`

运行语义概述：
- `默认等级`：跳过“选择难度”步骤
- 其他值：会在难度选择界面 OCR 点击对应难度（见 `operation/compendium/notorious_hunt.py`）

### 4.6 `predefined_team_idx`

- 类型：整数
- 允许值：
  - `-1`：使用游戏内当前配队（不切换预备编队）
  - `0..9`：选择对应的预备编队

数据源：
- `config/{实例}/team.yml`（工具应读取并展示队名）

运行语义概述：
- 当 `predefined_team_idx == -1`：战斗会使用条目内的 `auto_battle_config`
- 当 `predefined_team_idx != -1`：战斗会优先使用 `team.yml` 对应编队绑定的 `auto_battle`，条目内 `auto_battle_config` 通常不生效

### 4.7 `auto_battle_config`

- 类型：字符串
- 默认值：`全配队通用`

运行语义概述：
- 仅当 `predefined_team_idx == -1` 时，主程序会使用该字段加载自动战斗模板。

外部工具 UI 规则（建议作为硬规则实现）：
- 当 `predefined_team_idx != -1`：隐藏或禁用该字段，并提示“选择预备编队时自动战斗以 team.yml 为准”。

### 4.8 `run_times`

- 类型：整数
- 默认值：`0`
- 语义：已运行次数（完成判定：`run_times >= plan_times`）
- 约束：必须 `>= 0`

### 4.9 `plan_times`

- 类型：整数
- 默认值：`1`
- 语义：计划次数（目标次数）
- 约束：必须 `>= 0`
- 特殊含义：
  - `plan_times == 0`：表示“禁用该 BOSS”（该条目视为常态完成）

全局危险约束（工具必须强制校验）：
- **禁止所有条目的 `plan_times` 全部为 0**。

原因（与主程序一致）：
- 主程序 `reset_plans()` 的逻辑是“当全部条目完成时，对所有条目执行 `run_times -= plan_times` 并保存，直到出现未完成条目为止”。
- 当所有 `plan_times == 0` 时，`run_times -= 0` 永远不会改变状态，且“全部条目完成”始终成立，最终会造成死循环。

### 4.10 `notorious_hunt_buff_num`

- 类型：整数
- 默认值：`1`
- 允许值：`1..3`
- 语义：战斗前“鸣徽选择”从左到右第 1/2/3 个（见 `operation/compendium/notorious_hunt_move.py`）

外部工具必须严格限制范围：
- 如果写入 `0` 或负数，Python 负索引会导致选择逻辑出现“反向选择”，不可预期。

---

## 5. “全部 BOSS 列表”动态获取规则（外部工具必须实现）

数据源：
- `assets/game_data/compendium_data.yml`

定位规则：
- `tab_name == "训练"`
- `category_name == "恶名狩猎"`
- 取该分类下的 `mission_type_list[].mission_type_name` 作为“全部 BOSS 列表”

过滤规则：
- 过滤掉 `mission_type_name == "代理人方案培养"`（它不是 BOSS，只是训练目标）

当主程序/GUI 更新导致 BOSS 列表变化时：
- 外部工具在读取配置时应自动补齐新增 BOSS（见第 6 章）。

---

## 6. 规范化算法（读取后在内存中做，保存前强制执行）

目标：
- UI 列表长度固定为“当前全部 BOSS 数量”（动态）
- 每个 BOSS 恰好对应 1 条计划条目
- 读取旧配置/手改配置时尽量自愈到可运行状态

规范化步骤建议如下：

1) 读取 YAML（主路径优先，或回退历史路径）得到 `plan_list`
2) 读取 compendium 得到 `boss_list`（第 5 章规则）
3) 用 `mission_type_name` 作为主键构建映射：
   - 遇到重复条目：保留第一个，后续重复条目丢弃并产生告警
4) 对保留条目进行字段规范化：
   - `tab_name = "训练"`
   - `category_name = "恶名狩猎"`
   - `mission_name = null`
   - `run_times = max(0, run_times)`
   - `plan_times = max(0, plan_times)`
   - `notorious_hunt_buff_num` 不在 `1..3` 时重置为 `1` 并告警
5) 按“原有顺序优先”的策略生成新列表：
   - 按原列表顺序遍历，保留仍在 `boss_list` 中的条目
   - 缺失的 BOSS 追加到末尾，并用默认值创建条目：
     - `level = "默认等级"`
     - `predefined_team_idx = -1`
     - `auto_battle_config = "全配队通用"`
     - `run_times = 0`
     - `plan_times = 1`
     - `notorious_hunt_buff_num = 1`
6) 全局危险校验：
   - 如果所有条目的 `plan_times == 0`：拒绝保存并提示原因（见 4.9）

---

## 7. 外部编辑工具功能规格（仅更新与调整顺序）

本工具对齐 GUI 行为：列表固定长度，不提供新增/删除入口。

### 7.1 读取（R）

必须支持：
1. 选择实例 `idx` → 读取主路径 `notorious_hunt.yml`；
2. 主路径不存在时回退读取历史路径，并提示迁移；
3. 解析 `plan_list`，执行第 6 章规范化；
4. 进行严格校验（第 8 章）。

建议派生展示字段（不写回 YAML）：
- `completed = run_times >= plan_times`
- `remaining = max(0, plan_times - run_times)`

### 7.2 更新（U）

必须支持修改：
- `level`
- `predefined_team_idx`
- `auto_battle_config`（仅当 `predefined_team_idx == -1`）
- `notorious_hunt_buff_num`
- `run_times`
- `plan_times`

必须满足：
- 修改后立即通过自动保存写回主路径；
- 保存前必须通过严格校验，失败时拒绝落盘并提示错误原因。

### 7.3 调整顺序（Reorder）

必须支持：
- 拖拽排序；
- 单条置顶按钮（把该 BOSS 条目移动到列表头部）。

保存行为：
- 重排 `plan_list` 数组顺序并落盘。

### 7.4 不提供新增/删除（工具硬约束）

外部工具必须不提供：
- 新增条目入口
- 删除条目入口

说明：
- 配置文件不存在时，工具依然可以通过“读取→规范化→保存”的流程生成主路径文件；
- “条目数量变化”只允许由 compendium 的 BOSS 列表变化触发（自动补齐/自动移除过期 BOSS），不允许用户手工增删。

---

## 8. 严格校验规则（工具内置规则集）

建议作为“硬错误（errors）”的规则：
- 条目存在未知键（不在第 4 章 10 键集合内）
- `tab_name != 训练` 或 `category_name != 恶名狩猎`（保存前必须被规范化）
- `mission_type_name` 不在当前 `boss_list` 中
- `mission_name` 非 `null`
- `level` 不在允许集合中
- `predefined_team_idx` 不是 `-1` 或 `0..9`
- `run_times < 0` 或 `plan_times < 0`
- `notorious_hunt_buff_num` 不在 `1..3`
- **所有条目的 `plan_times` 全部为 0**

建议作为“告警（warnings）”的规则：
- 存在重复 `mission_type_name`（会在规范化时丢弃重复项）
- 配置里存在 compendium 已移除的 BOSS 条目（会在规范化时移除）

---

## 9. 配置样例（示意）

下面示例基于当前仓库的 compendium BOSS 列表。后续如果 compendium 更新增加/移除 BOSS，本工具会在读取时自动同步列表长度。

```yaml
plan_list:
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 初生死路屠夫
    mission_name: null
    level: 默认等级
    predefined_team_idx: -1
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 未知复合侵蚀体
    mission_name: null
    level: 默认等级
    predefined_team_idx: 0
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 冥宁芙·双子
    mission_name: null
    level: 默认等级
    predefined_team_idx: -1
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 「霸主侵蚀体·庞培」
    mission_name: null
    level: 默认等级
    predefined_team_idx: -1
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 牲鬼·布林格
    mission_name: null
    level: 默认等级
    predefined_team_idx: -1
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 秽息司祭
    mission_name: null
    level: 默认等级
    predefined_team_idx: -1
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 彷徨猎手
    mission_name: null
    level: 默认等级
    predefined_team_idx: -1
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
  - tab_name: 训练
    category_name: 恶名狩猎
    mission_type_name: 魇缚者·叶释渊
    mission_name: null
    level: 默认等级
    predefined_team_idx: -1
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 1
    notorious_hunt_buff_num: 1
```

