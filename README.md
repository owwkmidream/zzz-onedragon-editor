# ZZZ OneDragon 配置编辑器（Tauri）

这个工具用于在 **不启动本项目 GUI** 的情况下，直接编辑
ZenlessZoneZero-OneDragon 的配置文件。当前界面同时支持：

- `体力计划`：`config/{实例}/one_dragon/charge_plan.yml`
- `恶名狩猎`：`config/{实例}/one_dragon/notorious_hunt.yml`

两类配置都兼容历史路径：

- `config/{实例}/charge_plan.yml`
- `config/{实例}/notorious_hunt.yml`

历史路径只用于读取提示与迁移，新的保存目标始终是
`config/{实例}/one_dragon/` 下的主路径。

配置规则文档以当前工具目录下这两份为准：

- `docs/charge_plan_config_spec.md`
- `docs/notorious_hunt_config_spec.md`

---

## 使用方式（开发模式）

在本仓库根目录下执行：

```bash
pnpm install
pnpm tauri dev
```

启动后需要先在界面里 **手动选择 ZenlessZoneZero-OneDragon 项目根目录**，
然后点击“应用”加载数据。

设置弹窗支持：右上角关闭、点击遮罩关闭、按 `Esc` 关闭
（不必强制点击“应用”才能关闭）。

### 设置项持久化方式

项目根目录和上次实例通过前端 localStorage 持久化
（不是写到项目配置文件里）：

- `project_root`：项目根目录（只有点击“应用”才会写入并生效）
- `instance_idx`：上次选择的实例（下次启动会优先选中）

---

## 打包（Windows）

在本仓库根目录下执行：

```bash
pnpm install
pnpm run build:win
```

这条命令会同时整理出：

- 安装版：Tauri 官方 bundler 产物
  - `src-tauri/target/release/bundle/nsis/`
  - `src-tauri/target/release/bundle/msi/`
- 便携版：从 release 裸二进制复制出的独立 exe
  - `src-tauri/target/release/bundle/portable/`

补充说明：

- 当前 Tauri CLI 在 Windows 下的 bundle 目标是 `nsis` 和 `msi`
- 不存在单独的 `portable` bundle target
- 本仓库的“便携版”由构建脚本在 `tauri build` 完成后，额外从
  `src-tauri/target/release/charge-plan-editor-tauri.exe`
  整理到 `bundle/portable/`

如果你只想看 Tauri 原生行为，也可以直接执行：

```bash
pnpm tauri build
```

这时会生成：

- 安装包：`nsis` + `msi`
- 裸 exe：`src-tauri/target/release/charge-plan-editor-tauri.exe`

---

## 重要限制与提示

- Tauri Windows 图标使用主项目的 `assets/ui/logo.ico`
- 保存前会生成同目录备份：
  - `charge_plan.yyyymmdd-HHMMSS.bak`
  - `notorious_hunt.yyyymmdd-HHMMSS.bak`
- 保存使用原子写入，避免中途崩溃导致配置损坏
- 运行中的主程序可能缓存配置；保存后如果未生效，建议重启主程序
- `charge_plan` 的 `level` 字段：主程序当前保存逻辑不会写回该字段，
  即使本工具写入，也可能被主程序保存时擦除
- 条目存在未知键时，主程序会因反序列化报错；
  本工具会把未知键作为“错误”阻止保存


