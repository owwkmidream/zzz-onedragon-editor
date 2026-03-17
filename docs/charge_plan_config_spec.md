# 体力计划（`charge_plan`）配置规则与外部编辑工具规格

本文档面向“外部工具”开发：目标是在**不启动本项目 GUI** 的前提下，完成体力计划配置的读取、校验、编辑与保存。

文档内容基于仓库源码的实际行为（以下路径是主要真相源）：
- 配置模型与保存逻辑：`src/zzz_od/application/charge_plan/charge_plan_config.py`
- 运行语义（`loop/skip_plan/restore_charge` 如何影响执行）：`src/zzz_od/application/charge_plan/charge_plan_app.py`
- 应用配置文件路径规则（含旧路径迁移）：`src/one_dragon/base/operation/application/application_config.py`
- YAML 读写行为：`src/one_dragon/base/config/yaml_operator.py`、`src/one_dragon/base/config/yaml_config.py`
- 可选副本字典（严格校验的数据源）：`assets/game_data/compendium_data.yml` + `src/zzz_od/game_data/compendium.py`
- 预备编队索引与来源：`config/<实例>/team.yml` + `src/zzz_od/config/team_config.py`

---

## 1. 配置文件定位规则（外部工具必须实现）

### 1.1 实例（账号）选择

外部工具需要允许用户选择要编辑的“实例”，来源建议读取：
- `config/one_dragon.yml`：`instance_list` 里的 `idx`（数字）与 `name`（显示用）

实例路径规则中的 `instance_idx` 使用“两位补零”格式：
- `idx=1` → `01`

### 1.2 组（group_id）选择

本项目的“应用配置”按组分目录保存。体力计划在默认情况下使用：
- `group_id = "one_dragon"`（见 `src/one_dragon/base/operation/application/application_const.py`）

外部工具建议默认固定为 `one_dragon`，同时在 UI 中保留“高级选项”以便未来扩展其他组。

### 1.3 体力计划配置文件路径（主路径与历史路径）

**主路径（必须优先读取/写入）**
- `config/{instance_idx}/{group_id}/charge_plan.yml`
- 示例：`config/01/one_dragon/charge_plan.yml`

**历史路径（仅用于提示、回退读取、或者迁移）**
- `config/{instance_idx}/charge_plan.yml`
- 示例：`config/01/charge_plan.yml`

**迁移行为说明（非常重要）**
- `ApplicationConfig` 会在“主路径不存在且历史路径存在”时，将历史路径文件复制到主路径。
- 如果主路径已存在，历史路径即使存在也不会被使用。

外部工具推荐策略：
1. 默认只编辑主路径。
2. 若主路径不存在但历史路径存在：
   - 在 UI 中提示“检测到历史路径配置”，提供一键“迁移到新路径（复制）”功能；
   - 或者在保存时直接以“主路径”为目标写入（相当于完成迁移）。

### 1.4 修改后何时生效

本项目在运行时会对配置对象与 YAML 读取做缓存；外部工具修改文件后：
- **不保证运行中的程序立刻读到新内容**；
- 最稳妥的方式是**关闭程序 → 修改配置 → 重启程序**。

外部工具建议：
- 保存后提示用户“如程序正在运行，建议重启以生效”。

---

## 2. YAML 读写格式约束（外部工具落盘必须遵守）

### 2.1 编码与格式

- 文件编码：`UTF-8`
- 文件格式：YAML
- 本项目保存 YAML 的方式（源码行为）：
  - `yaml.dump(data, allow_unicode=True, sort_keys=False)`（见 `YamlOperator.save()`）

外部工具建议采用一致策略：
- 保留键顺序（不要自动排序 key），避免无意义的 diff；
- 允许中文值不转义；
- 用稳定的缩进（建议 2 空格）；
- 保留末尾换行。

### 2.2 原子写入与备份（强烈建议）

为了防止写文件中途崩溃导致配置损坏，外部工具建议实现：
1. 写入临时文件：`charge_plan.yml.tmp`
2. fsync（如果平台容易实现）
3. 原子替换：把 `.tmp` 替换为正式文件
4. 写入前备份：`charge_plan.yyyymmdd-HHMMSS.bak`

备份建议放在同一目录，方便用户手动回滚。

---

## 3. 顶层配置（ChargePlanConfig）字段说明

体力计划配置文件对应 `ChargePlanConfig`，落盘结构为一个 YAML mapping（字典）。

以下字段均为**顶层键**：

### 3.1 `loop: bool`

- 类型：布尔
- 默认值：`true`（代码读取默认 `get('loop', True)`）
- 语义：当所有计划条目都“完成”后，是否进入下一轮循环。
  - `true`：会在所有条目都完成时，调用“重置计划”（见 6.2）继续循环。
  - `false`：所有条目完成后，本轮结束。

### 3.2 `skip_plan: bool`

- 类型：布尔
- 默认值：`false`
- 语义：当某条计划被判定“体力不足且无法恢复”时：
  - `true`：跳过该计划，继续尝试下一条计划；
  - `false`：直接结束本轮体力计划。

补充说明：
- 运行过程中，某些场景会把条目标为 `skipped`（内存状态），该字段**不会**写回 YAML。

### 3.3 `use_coupon: bool`

- 类型：布尔
- 默认值：`false`
- 现状：`区域巡防`中的“家政券”节点已被注释，GUI 也已隐藏该项（见 `AreaPatrol` 注释段）。
- 外部工具建议：仍展示并允许编辑，但在 UI 上标注“当前版本不生效（保留字段）”。

### 3.4 `restore_charge: string`

- 类型：字符串
- 默认值：`不使用`
- 允许值（必须严格限制为以下中文值之一，大小写与空格都必须一致）：
  - `不使用`
  - `使用储蓄电量`
  - `使用以太电池`
  - `同时使用储蓄电量和以太电池`

运行语义概述：
- `不使用`：体力不足时不会走恢复流程。
- 其他三种：当“估算体力不足”时会尝试恢复（见 `ChargePlanApp.find_and_select_next_plan()`）。

### 3.5 `plan_list: list`

- 类型：数组
- 元素类型：`ChargePlanItem`（见第 4 章）
- 缺省：可不存在；不存在时等价于空数组。

### 3.6 `history_list: list`

- 类型：数组
- 元素结构：与 `plan_list` 的条目结构一致，但语义是“历史快照/模板池”（见第 5 章）。
- 缺省：可不存在；不存在时等价于空数组。

---

## 4. 计划条目（ChargePlanItem）字段说明（严格校验核心）

`plan_list` 与 `history_list` 中每个元素都是一个 YAML mapping（字典），会被 `ChargePlanItem(**plan_item)` 反序列化。

因此：**条目中存在未知键会导致启动时报错**（Python `__init__` 不接受未知参数）。外部工具必须拒绝保存未知键，或在导入时清理未知键并提示用户。

### 4.1 字段全集（允许写入的键）

以下是 `ChargePlanItem.__init__` 支持的键（外部工具必须以此为准）：

#### 4.1.1 `tab_name: string`

- 默认值：`训练`
- 建议严格校验：必须存在于 `assets/game_data/compendium_data.yml` 的 tab 列表中。

#### 4.1.2 `category_name: string`

- 默认值：`实战模拟室`
- 建议严格校验：必须属于 `tab_name` 下的分类。

运行语义提示：
- `ChargePlanApp` 在“估算体力门槛”时会按 `category_name` 判断需要多少体力（见 6.1）。

#### 4.1.3 `mission_type_name: string`

- 默认值：`基础材料`
- 建议严格校验：必须属于 `tab_name + category_name` 下的 mission type。

补充说明（显示名与真实值）：
- compendium 里可能存在 `mission_type_name_display`；
- GUI 列表展示用 display，落盘保存用 `mission_type_name`；
- 外部工具可以使用 display 供用户选择，但保存必须写 canonical 值。

#### 4.1.4 `mission_name: string | null`

- 默认值：`调查专项`
- 建议严格校验：
  - 如果该 `mission_type_name` 下存在 mission 列表，则必须是列表中的某一项；
  - 如果该类型不需要更细分 mission（或 mission 列表为空），可允许 `null`。

运行语义提示：
- 某些副本（例如实战模拟室）会在进入后根据 `mission_name` OCR 点击对应关卡。

#### 4.1.5 `level: string`

- 默认值：`默认等级`

重要限制（必须写在工具实现提示中）：
- `ChargePlanConfig.save()` **不会**把 `level` 写回 YAML。
- 这意味着：
  - 外部工具即使写入了 `level`；
  - 只要程序运行过程中触发保存（例如运行次数变化、GUI 修改），`level` 就可能被擦除。

外部工具建议：
- 体力计划编辑器不要把“level 可长期保存”当作可靠能力；
- 如果一定要支持，需要先在项目源码中补全 `save()` 写回逻辑。

#### 4.1.6 `auto_battle_config: string`

- 默认值：`全配队通用`
- 语义：
  - 当 `predefined_team_idx == -1`：战斗会使用 `auto_battle_config`。
  - 当 `predefined_team_idx != -1`：战斗会优先使用 `config/<实例>/team.yml` 中该编队绑定的 `auto_battle`，此字段在运行时通常不生效。

严格校验（建议）：
- 如果要校验“配置是否存在”，可以检查对应的自动战斗 YAML 模板是否存在；
- 即使不存在，运行时也会回退到 `全配队通用`（见 `AutoBattleOperator` 的回退逻辑）。

#### 4.1.7 `run_times: int`

- 默认值：`0`
- 语义：已完成次数。完成条件是 `run_times >= plan_times`。
- 建议约束：整数且 `>= 0`。

#### 4.1.8 `plan_times: int`

- 默认值：`1`
- 语义：计划目标次数。
- 常见用法：
  - 小整数：只刷固定次数
  - 大整数（例如 `999`）：长期循环目标
- 建议约束：整数且 `>= 0`。

#### 4.1.9 `card_num: string`

- 默认值：`默认数量`
- 允许值（必须严格限制）：
  - `默认数量`
  - `'1'`、`'2'`、`'3'`、`'4'`、`'5'`

特别提示：
- 这里是**字符串**，不是数字；外部工具保存时建议保持为字符串，避免 YAML 写成 `card_num: 2` 造成类型漂移。

运行语义提示：
- 主要用于 `实战模拟室`：每张卡 20 体力，估算门槛与本字段有关（见 6.1）。

#### 4.1.10 `predefined_team_idx: int`

- 默认值：`-1`
- 允许值：
  - `-1`：使用游戏内当前配队（不切预备编队）
  - `0..9`：选择对应预备编队索引（见 `TeamConfig` 最大 10 个队）

严格校验建议：
- 读取 `config/{instance_idx}/team.yml` 并按 `TeamConfig.team_list` 规则展开为 10 个队；
- 要求索引在范围内。

#### 4.1.11 `notorious_hunt_buff_num: int`

- 默认值：`1`
- 建议约束：整数 `1..3`。

#### 4.1.12 `plan_id: string(UUID)`

- 默认值：如果缺失会在运行时生成 UUID（但外部工具不应依赖运行时补全）
- 语义：计划条目的“主键”。
  - 运行时判断“是不是同一个计划”会优先比较 `plan_id`；
  - 只有在 `plan_id` 缺失时才会退回到字段组合比较。

外部工具要求：
- 新增条目必须生成 UUIDv4 字符串；
- 修改条目时必须保持 `plan_id` 不变；
- 导入时若发现重复 `plan_id`，视为同一条目，按导入策略决定覆盖或跳过。

---

## 5. `history_list` 的语义与维护规则（与程序保存一致）

### 5.1 `history_list` 用途

`history_list` 是“历史快照池”，GUI 在编辑某条计划时，可以从历史里找到“同一条计划”的历史记录，并用历史值更新部分字段，例如：
- `card_num`
- `notorious_hunt_buff_num`
- `predefined_team_idx`
- `auto_battle_config`
- `plan_times`

外部工具如果要做到“与 GUI 一致的体验”，建议也维护 `history_list`。

### 5.2 维护算法（外部工具保存时必须可选实现）

本项目 `ChargePlanConfig.save()` 的行为可以概括为：
1. 把当前 `plan_list` 每条条目转为 dict，写入 `data.plan_list`。
2. 生成 `new_history_list`：
   - 先把“当前 plan_list 的快照”按顺序全部加入 `new_history_list`；
   - 再遍历旧的 `history_list`，把“在当前 plan_list 中找不到对应计划”的旧条目追加到 `new_history_list` 的末尾；
3. 写入 `data.history_list = new_history_list` 并保存。

其中“是否是同一计划”的判断：
- 优先 `plan_id` 相等；
- 如果任意一方缺 `plan_id`，则退回比较：
  - `tab_name`
  - `category_name`
  - `mission_type_name`
  - `mission_name`

外部工具建议的实现选择：
- 默认开启 history 维护，使得用户删除/修改计划后仍可从历史快速恢复设置；
- 在“高级选项”提供开关：允许用户关闭 history 维护（只写 `plan_list`）。

---

## 6. 运行语义（外部工具必须理解的关键点）

本节用于解释“改某个字段会带来什么后果”，以便外部工具能给出正确的提示与预览。

### 6.1 体力不足判断的“估算门槛”（选择下一条计划前）

体力计划在选择下一条计划时，会按 `category_name` 做一个“最低体力门槛”估算：
- `实战模拟室`：
  - 如果 `card_num == 默认数量`：门槛为 `20`
  - 否则：门槛为 `int(card_num) * 20`
- `区域巡防`：门槛为 `60`
- `专业挑战室`：门槛为 `40`
- `恶名狩猎`：门槛为 `60`

外部工具建议提供：
- “预计单次最低体力消耗”展示（只是门槛，不代表真实消耗）；
- 条目列表的筛选/排序（例如按门槛排序），方便用户把高耗体力放前或放后。

### 6.2 `loop` 与“重置计划”的数学含义

当所有条目都满足 `run_times >= plan_times`，如果 `loop == true`，会执行“重置计划”：
- 如果存在任意条目 `run_times < plan_times`：停止重置（说明仍有未完成条目）；
- 否则对所有未被 skipped 的条目执行：
  - `run_times = run_times - plan_times`；
- 重置后保存并开始下一轮。

外部工具建议提供：
- 一键操作：“把所有 `run_times` 归零”（与 loop 逻辑不同，但用户常需要）；
- 预览提示：“loop 开启会循环到体力耗尽”。

### 6.3 `skip_plan` 与 skipped（内存态）的区别

- `skip_plan` 是配置开关（落盘字段）：
  - 控制“遇到体力不足时是否跳过该计划”。
- `skipped` 是运行期内存标记（不落盘）：
  - 当某条计划在本轮被跳过，会被标记，后续 `get_next_plan` 会跳过它。

外部工具不要尝试写入 `skipped`，因为该字段不会被读取或保存。

补充说明：
- 当 `mission_type_name == '代理人方案培养'` 时，即使 `skip_plan == false`，运行时在体力不足场景也会选择跳过（见 `ChargePlanApp.charge_not_enough()` 的 `is_agent_plan` 分支）。

### 6.4 `predefined_team_idx` 对自动战斗选择的影响

当开始战斗时：
1. 如果 `predefined_team_idx == -1`：使用条目中的 `auto_battle_config`；
2. 否则：使用 `config/<实例>/team.yml` 中对应编队的 `auto_battle`。

外部工具建议：
- 在 UI 中把这条规则显示出来，避免用户误以为修改 `auto_battle_config` 会影响“选了预备编队”的条目。

---

## 7. 外部编辑工具功能规格（增删改查与常用批量操作）

本节定义外部工具的功能集合，适用于轻量 UI（Tauri 或 WPF）。

### 7.1 读取（R）

必须支持：
1. 选择实例 `idx` → 读取主路径 `charge_plan.yml`；
2. 解析顶层字段：
   - `loop`、`skip_plan`、`use_coupon`、`restore_charge`；
3. 解析 `plan_list` 与 `history_list`：
   - 每条条目按 4.1 字段集解析；
   - 发现未知键：必须提示并拒绝保存（或在“导入/修复”模式清理）。

建议派生展示字段（不写回 YAML）：
- `completed = run_times >= plan_times`
- `remaining = max(0, plan_times - run_times)`
- `uid = f'{tab_name}_{category_name}_{mission_type_name}_{mission_name}'`（便于去重与检索；与源码 `ChargePlanItem.uid` 一致）
- `is_agent_plan = (mission_type_name == '代理人方案培养')`（与源码 `ChargePlanItem.is_agent_plan` 一致）
- `energy_floor`（按 6.1 估算门槛）

### 7.2 新增（C）

新增条目必须满足：
- 自动生成 `plan_id`（UUIDv4）；
- `run_times` 默认 0；
- `plan_times` 默认 1；
- `card_num` 默认 `默认数量`；
- `predefined_team_idx` 默认 -1；
- `notorious_hunt_buff_num` 默认 1。

新增时的严格校验数据源：
- `tab/category/mission_type/mission` 都从 `assets/game_data/compendium_data.yml` 读取；
- `predefined_team_idx` 结合 `config/<实例>/team.yml` 校验范围与显示队名。

### 7.3 修改（U）

修改定位必须以 `plan_id` 为主键（不要用数组下标当主键）。

必须支持的修改项：
- 顶层：`loop/skip_plan/restore_charge/use_coupon`；
- 条目：除 `plan_id` 外全部字段。

建议支持的批量修改：
- 批量设置 `predefined_team_idx`；
- 批量设置 `plan_times`；
- 批量重置 `run_times`（归零，或者设置为 `plan_times` 直接标完成）；
- 批量设置 `card_num`（仅对 `category_name == 实战模拟室` 生效）。

### 7.4 删除（D）

必须支持：
- 删除单条（按 `plan_id`）；
- 删除已完成：删除所有 `run_times >= plan_times` 的条目；
- 删除全部：清空 `plan_list`。

建议支持撤销：
- 在一次保存周期内保留“上一次 plan_list 快照”，允许撤销到保存前状态。

### 7.5 调整顺序（Reorder）

必须支持：
- 上移 / 下移 / 置顶 / 置底；
- 拖拽排序（如果 UI 框架支持）。

保存行为：
- 重排 `plan_list` 数组顺序并落盘。

### 7.6 导入/导出（建议实现）

导出建议：
- 只导出 `plan_list`（不导出 `history_list`），作为“可复用模板”；
- 格式可以是 YAML 或 JSON，但必须固定并在工具内自描述版本号，例如：
  - `export_version: 1`
  - `plan_list: [...]`

导入策略建议：
- `plan_id` 相同：视为同一条，默认覆盖（或提供开关：跳过或覆盖）；
- 缺失 `plan_id`：视为新条目，导入时生成新的 UUID；
- 导入后按 5.2 的 history 维护规则更新（如果启用）。

---

## 8. 严格校验规则（建议作为工具内置规则集）

外部工具选择“严格校验”时，建议采用以下硬规则。

### 8.1 副本字典校验（compendium）

数据源：`assets/game_data/compendium_data.yml`

校验规则：
1. `tab_name` 必须存在；
2. `category_name` 必须属于该 tab；
3. `mission_type_name` 必须属于该 category；
4. `mission_name`：
   - 如果 mission_type 下存在 mission_list：必须存在；
   - 如果 mission_list 为空：允许 `null` 或空字符串（推荐统一保存为 `null`）。

### 8.2 枚举与范围校验

- `restore_charge`：必须是 3.4 的四个字符串之一；
- `card_num`：必须是 `默认数量` 或 `'1'..'5'`；
- `predefined_team_idx`：必须是 `-1` 或 `0..9`，并且能映射到 `team.yml` 展开的队伍列表；
- `notorious_hunt_buff_num`：建议限制在 `1..3`；
- `run_times/plan_times`：必须是整数且 `>= 0`。

### 8.3 禁止未知键

- 条目 dict 允许键集合严格等于 4.1 中列出的字段集合。
- 发现未知键时的处理建议：
  - 列表展示时标红并显示“未知键列表”；
  - 保存时拒绝落盘，或要求用户进入“修复模式”清理未知键。

---

## 9. 配置样例（最小可用）

下面给出一个最小可用的 `charge_plan.yml` 示例。注意：`plan_id` 需要使用真实 UUIDv4。

```yaml
loop: true
skip_plan: true
use_coupon: false
restore_charge: 不使用
plan_list:
  - tab_name: 训练
    category_name: 实战模拟室
    mission_type_name: 基础材料
    mission_name: 调查专项
    level: 默认等级
    auto_battle_config: 全配队通用
    run_times: 0
    plan_times: 10
    card_num: '2'
    predefined_team_idx: -1
    notorious_hunt_buff_num: 1
    plan_id: 00000000-0000-4000-8000-000000000000
history_list: []
```

---

## 10. 与体力计划相关但不属于本配置文件的内容（避免误解）

### 10.1 运行记录文件（不是体力计划配置）

运行记录属于另一个体系，路径通常是：
- `config/{instance_idx}/app_run_record/charge_plan.yml`

它记录的是运行状态与体力快照（例如 `current_charge_power_snapshot`），外部工具如果目标仅是“修改体力计划”，不需要编辑该文件。

---

## 11. 外部工具实现建议（Tauri / WPF 都适用）

建议的内部数据模型（仅建议，不要求与源码同名）：
- `ChargePlanConfigModel`
  - `loop: bool`
  - `skip_plan: bool`
  - `use_coupon: bool`
  - `restore_charge: string`
  - `plan_list: ChargePlanItemModel[]`
  - `history_list: ChargePlanItemModel[]`
- `ChargePlanItemModel`
  - 对应 4.1 字段全集

建议保存流程：
1. 读取 YAML → 解析 → 严格校验 → 生成内存模型；
2. 用户编辑 → 再次校验；
3. 按 5.2 生成 `history_list`（如果启用）；
4. 写备份 → 原子写入 → 成功提示。

