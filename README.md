# 体力计划编辑器（Tauri）

这个工具用于在 **不启动本项目 GUI** 的情况下，编辑体力计划配置：

- 读取/编辑/保存：`config/{实例}/one_dragon/charge_plan.yml`
- 兼容历史路径：`config/{实例}/charge_plan.yml`（仅用于读取提示与迁移）

配置规则真相源：`docs/charge_plan_config_spec.md`

（该文档在主项目仓库中的路径通常为：`docs/develop/zzz/config/charge_plan_config_spec.md`）

---

## 使用方式（开发模式）

在本仓库根目录下执行：

```bash
pnpm install
pnpm tauri dev
```

启动后需要先在界面里 **手动选择 ZenlessZoneZero-OneDragon 项目根目录**，然后点击“应用”加载数据。

设置弹窗支持：右上角关闭、点击遮罩关闭、按 `Esc` 关闭（不必强制点击“应用”才能关闭）。

### 设置项持久化方式

项目根目录与“上次选择的实例”通过前端 `localStorage` 持久化（不是写到项目配置文件里）：

- `project_root`：项目根目录（只有点击“应用”才会写入并生效）
- `instance_idx`：上次选择的实例（下次启动会优先选中）

---

## 打包（Windows）

在本仓库根目录下执行：

```bash
pnpm install
pnpm tauri build
```

打包产物通常在：

- `src-tauri/target/release/bundle/`

（具体包含 MSI/NSIS/便携版，取决于 Tauri bundler 配置与系统环境。）

---

## 重要限制与提示

- 保存前会生成同目录备份：`charge_plan.yyyymmdd-HHMMSS.bak`
- 保存使用原子写入，避免中途崩溃导致配置损坏
- 运行中的主程序可能缓存配置；保存后如果未生效，建议重启主程序
- `level` 字段：主程序当前保存逻辑不会写回该字段，即使本工具写入，也可能被主程序保存时擦除（v0 不提供该字段编辑入口）
- 条目存在未知键时，主程序会因 `ChargePlanItem(**plan_item)` 反序列化报错；本工具会把未知键作为“错误”阻止保存
