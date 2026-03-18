import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import Sortable from "sortablejs";
import {
  ArrowDownToLine,
  ArrowUpToLine,
  GripVertical,
  History,
  ListTodo,
  Pin,
  Settings,
  Trash2,
  X,
  createElement,
  type IconNode,
} from "lucide";

type TabKind = "charge_plan" | "notorious_hunt";

type InstanceInfo = {
  idx: number;
  name: string;
  active?: boolean;
  active_in_od?: boolean;
};

type ConfigPaths = {
  main_path: string;
  legacy_path: string;
  main_exists: boolean;
  legacy_exists: boolean;
};

type ValidationResult = { errors: string[]; warnings: string[] };

type ChargePlanItem = {
  tab_name: string;
  category_name: string;
  mission_type_name: string;
  mission_name: string | null;
  level: string | null;
  auto_battle_config: string;
  run_times: number;
  plan_times: number;
  card_num: string;
  predefined_team_idx: number;
  notorious_hunt_buff_num: number;
  plan_id: string;
};

type ChargePlanConfigModel = {
  loop_enabled: boolean;
  skip_plan: boolean;
  use_coupon: boolean;
  restore_charge: string;
  plan_list: ChargePlanItem[];
  history_list: ChargePlanItem[];
};

type ReadChargePlanResult = {
  source: string;
  paths: ConfigPaths;
  config: ChargePlanConfigModel;
  validation: ValidationResult;
};

type SaveResult = { written_path: string; backup_path?: string | null };

type LabeledValue = { label: string; value: string };

type CompendiumForChargePlan = {
  categories: string[];
  mission_types_by_category: Record<string, LabeledValue[]>;
  missions_by_category_and_type: Record<string, Record<string, LabeledValue[]>>;
};

type TeamInfo = { idx: number; name: string; auto_battle: string };

type NotoriousHuntItem = {
  mission_type_name: string;
  level: string;
  predefined_team_idx: number;
  auto_battle_config: string;
  run_times: number;
  plan_times: number;
  notorious_hunt_buff_num: number;
};

type NotoriousHuntConfigModel = {
  plan_list: NotoriousHuntItem[];
};

type ReadNotoriousHuntResult = {
  source: string;
  paths: ConfigPaths;
  config: NotoriousHuntConfigModel;
  validation: ValidationResult;
};

const RESTORE_CHARGE_ALLOWED = [
  "不使用",
  "使用储蓄电量",
  "使用以太电池",
  "同时使用储蓄电量和以太电池",
];

const CARD_NUM_ALLOWED = ["默认数量", "1", "2", "3", "4", "5"];

const HUNT_LEVEL_ALLOWED = [
  "默认等级",
  "等级Lv.65",
  "等级Lv.60",
  "等级Lv.50",
  "等级Lv.40",
  "等级Lv.30",
];

const HUNT_BUFF_ALLOWED = [
  { label: "第一个BUFF", value: 1 },
  { label: "第二个BUFF", value: 2 },
  { label: "第三个BUFF", value: 3 },
];

const HUNT_CATEGORY_NAME = "恶名狩猎";
const HUNT_FILTER_MISSION_TYPE = "代理人方案培养";

const $ = <T extends HTMLElement>(sel: string) =>
  document.querySelector(sel) as T;

const storage = {
  getProjectRoot(): string {
    return localStorage.getItem("project_root") ?? "";
  },
  setProjectRoot(value: string) {
    localStorage.setItem("project_root", value);
  },
  getLastInstance(): number | null {
    const raw = localStorage.getItem("instance_idx");
    if (!raw) return null;
    const n = Number(raw);
    return Number.isFinite(n) ? n : null;
  },
  setLastInstance(value: number) {
    localStorage.setItem("instance_idx", String(value));
  },
};

const state = {
  projectRoot: "",
  instances: [] as InstanceInfo[],
  compendium: null as CompendiumForChargePlan | null,
  teams: [] as TeamInfo[],
  autoBattles: [] as string[],
  currentInstanceIdx: 1,
  activeTab: "charge_plan" as TabKind,
  chargePlan: {
    loadedInstanceIdx: null as number | null,
    paths: null as ConfigPaths | null,
    source: "none",
    config: null as ChargePlanConfigModel | null,
    validation: null as ValidationResult | null,
  },
  hunt: {
    loadedInstanceIdx: null as number | null,
    paths: null as ConfigPaths | null,
    source: "none",
    config: null as NotoriousHuntConfigModel | null,
    validation: null as ValidationResult | null,
  },
};

let planSortable: Sortable | null = null;
let huntSortable: Sortable | null = null;
let saveTimer: number | null = null;
let saving = false;
const pendingSaves = new Set<TabKind>();

function iconSvg(iconNode: IconNode, size: number) {
  const el = createElement(iconNode, {
    width: size,
    height: size,
  });
  el.setAttribute("aria-hidden", "true");
  return el;
}

function setText(id: string, text: string) {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

function setSaveStatus(
  text: string,
  kind: "" | "saving" | "ok" | "err" = "",
) {
  const el = document.getElementById("save-status");
  if (!el) return;
  el.textContent = text;
  if (!kind) el.removeAttribute("data-kind");
  else el.setAttribute("data-kind", kind);
}

function fmtClock(d = new Date()) {
  return d.toLocaleTimeString("zh-CN", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function fmtStatusDot(ok: boolean) {
  return ok ? "●" : "○";
}

function instanceLabel(i: InstanceInfo) {
  const flags: string[] = [];
  if (i.active) flags.push("active");
  if (i.active_in_od) flags.push("active_in_od");
  const flagText = flags.length ? ` (${flags.join(",")})` : "";
  return `${String(i.idx).padStart(2, "0")} - ${i.name}${flagText}`;
}

function tabLabel(kind: TabKind) {
  return kind === "charge_plan" ? "体力计划" : "恶名狩猎";
}

function resetLoadedConfigs() {
  state.chargePlan.loadedInstanceIdx = null;
  state.chargePlan.paths = null;
  state.chargePlan.source = "none";
  state.chargePlan.config = null;
  state.chargePlan.validation = null;

  state.hunt.loadedInstanceIdx = null;
  state.hunt.paths = null;
  state.hunt.source = "none";
  state.hunt.config = null;
  state.hunt.validation = null;
}

async function applyProjectRoot(root: string) {
  state.projectRoot = root.trim();
  ($<HTMLInputElement>("#project-root")).value = state.projectRoot;
  if (!state.projectRoot) {
    setText("root-status", "未设置项目根目录。");
    return;
  }

  await invoke("set_project_root", { path: state.projectRoot });
  storage.setProjectRoot(state.projectRoot);
  setText("root-status", `已应用：${state.projectRoot}`);

  resetLoadedConfigs();
  await loadInstances();
  await loadOptions();
  renderTabs();
  await loadActiveTab(true);
  renderActiveTabUI();
}

async function loadInstances() {
  const instances = await invoke<InstanceInfo[]>("list_instances");
  state.instances = instances;
  const select = $<HTMLSelectElement>("#instance-select");
  select.innerHTML = "";
  for (const inst of instances) {
    const opt = document.createElement("option");
    opt.value = String(inst.idx);
    opt.textContent = instanceLabel(inst);
    select.appendChild(opt);
  }

  const last = storage.getLastInstance();
  const preferred =
    last && instances.some((x) => x.idx === last)
      ? last
      : instances[0]?.idx ?? 1;
  state.currentInstanceIdx = preferred;
  select.value = String(preferred);
  storage.setLastInstance(preferred);
}

async function loadOptions() {
  state.compendium = await invoke<CompendiumForChargePlan>(
    "load_compendium_for_charge_plan",
  );
  state.teams = await invoke<TeamInfo[]>("load_team_list", {
    instanceIdx: state.currentInstanceIdx,
  });
  state.autoBattles = await invoke<string[]>("list_auto_battle_templates");

  const restoreSel = $<HTMLSelectElement>("#cfg-restore");
  restoreSel.innerHTML = "";
  for (const v of RESTORE_CHARGE_ALLOWED) {
    const opt = document.createElement("option");
    opt.value = v;
    opt.textContent = v;
    restoreSel.appendChild(opt);
  }
}

async function loadChargePlan(force = false) {
  if (
    !force &&
    state.chargePlan.loadedInstanceIdx === state.currentInstanceIdx &&
    state.chargePlan.config
  ) {
    return;
  }

  const res = await invoke<ReadChargePlanResult>("read_charge_plan", {
    instanceIdx: state.currentInstanceIdx,
  });
  state.chargePlan.loadedInstanceIdx = state.currentInstanceIdx;
  state.chargePlan.paths = res.paths;
  state.chargePlan.source = res.source;
  state.chargePlan.config = res.config;
  state.chargePlan.validation = res.validation;
}

async function loadHunt(force = false) {
  if (
    !force &&
    state.hunt.loadedInstanceIdx === state.currentInstanceIdx &&
    state.hunt.config
  ) {
    return;
  }

  const res = await invoke<ReadNotoriousHuntResult>("read_notorious_hunt", {
    instanceIdx: state.currentInstanceIdx,
  });
  state.hunt.loadedInstanceIdx = state.currentInstanceIdx;
  state.hunt.paths = res.paths;
  state.hunt.source = res.source;
  state.hunt.config = res.config;
  state.hunt.validation = res.validation;
}

async function loadActiveTab(force = false) {
  if (!state.projectRoot) return;
  if (state.activeTab === "charge_plan") await loadChargePlan(force);
  else await loadHunt(force);
}

function renderPathsStatus() {
  const tabState = state.activeTab === "charge_plan" ? state.chargePlan : state.hunt;
  if (!tabState.paths) {
    setText("paths-status", "");
    return;
  }
  const p = tabState.paths;
  const text = [
    `${fmtStatusDot(p.main_exists)} main: ${p.main_path}`,
    `${fmtStatusDot(p.legacy_exists)} legacy: ${p.legacy_path}`,
    `source: ${tabState.source}`,
  ].join(" | ");
  setText("paths-status", text);
}

function renderMigrationCard() {
  const card = document.getElementById("migration-card") as HTMLElement | null;
  const text = document.getElementById("migration-text") as HTMLElement | null;
  const tabState = state.activeTab === "charge_plan" ? state.chargePlan : state.hunt;
  if (!card || !text) return;
  if (!tabState.paths) {
    card.hidden = true;
    return;
  }
  if (tabState.source !== "legacy") {
    card.hidden = true;
    return;
  }
  card.hidden = false;
  text.textContent = `当前读取自 legacy：${tabState.paths.legacy_path}。建议迁移到主路径：${tabState.paths.main_path}。`;
}

function renderChargePlanHeader() {
  if (!state.chargePlan.config) return;
  ($<HTMLInputElement>("#cfg-loop")).checked = state.chargePlan.config.loop_enabled;
  ($<HTMLInputElement>("#cfg-skip")).checked = state.chargePlan.config.skip_plan;
  ($<HTMLInputElement>("#cfg-coupon")).checked = state.chargePlan.config.use_coupon;
  ($<HTMLSelectElement>("#cfg-restore")).value = state.chargePlan.config.restore_charge;
}

function syncChargePlanFromHeader() {
  if (!state.chargePlan.config) return;
  state.chargePlan.config.loop_enabled = ($<HTMLInputElement>("#cfg-loop")).checked;
  state.chargePlan.config.skip_plan = ($<HTMLInputElement>("#cfg-skip")).checked;
  state.chargePlan.config.use_coupon = ($<HTMLInputElement>("#cfg-coupon")).checked;
  state.chargePlan.config.restore_charge = ($<HTMLSelectElement>("#cfg-restore")).value;
}

function scheduleAutoSave(kind: TabKind = state.activeTab) {
  pendingSaves.add(kind);
  if (saveTimer) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => void autoSave(), 600);
  setSaveStatus(`自动保存：待保存（${fmtClock()}）`);
}

async function autoSave() {
  if (pendingSaves.size === 0) return;
  if (saving) return;

  saving = true;
  setSaveStatus("自动保存：保存中…", "saving");

  const toSave = Array.from(pendingSaves);
  pendingSaves.clear();

  const errors: Array<{ kind: TabKind; message: string }> = [];
  for (const kind of toSave) {
    try {
      if (kind === "charge_plan") await saveChargePlan();
      else await saveHunt();
    } catch (e) {
      errors.push({ kind, message: String(e) });
    }
  }

  if (errors.length) {
    const first = errors[0];
    setSaveStatus(
      `自动保存失败（${tabLabel(first.kind)}）：${first.message}`,
      "err",
    );
  } else {
    setSaveStatus(`自动保存：已保存（${fmtClock()}）`, "ok");
  }

  saving = false;
  if (pendingSaves.size) scheduleAutoSave();
}

async function saveChargePlan() {
  if (!state.chargePlan.config) return;
  syncChargePlanFromHeader();
  const res = await invoke<SaveResult>("save_charge_plan", {
    instanceIdx: state.currentInstanceIdx,
    config: state.chargePlan.config,
    options: { update_history_list: true },
  });
  void res;

  if (state.chargePlan.paths) state.chargePlan.paths.main_exists = true;
  state.chargePlan.source = "main";

  if (state.activeTab === "charge_plan") {
    renderPathsStatus();
    renderMigrationCard();
  }
}

async function saveHunt() {
  if (!state.hunt.config) return;
  const res = await invoke<SaveResult>("save_notorious_hunt", {
    instanceIdx: state.currentInstanceIdx,
    config: state.hunt.config,
  });
  void res;

  if (state.hunt.paths) state.hunt.paths.main_exists = true;
  state.hunt.source = "main";

  if (state.activeTab === "notorious_hunt") {
    renderPathsStatus();
    renderMigrationCard();
  }
}

function isCompleted(item: ChargePlanItem) {
  return item.run_times >= item.plan_times;
}

function getMissionTypes(category: string): LabeledValue[] {
  if (!state.compendium) return [];
  return state.compendium.mission_types_by_category[category] ?? [];
}

function getMissions(category: string, missionType: string): LabeledValue[] {
  if (!state.compendium) return [];
  return (
    state.compendium.missions_by_category_and_type[category]?.[missionType] ?? []
  );
}

function ensureValidSelection(item: ChargePlanItem) {
  if (!state.compendium) return;

  // category
  if (!state.compendium.categories.includes(item.category_name)) {
    item.category_name = state.compendium.categories[0] ?? item.category_name;
  }

  const types = getMissionTypes(item.category_name);
  if (!types.find((t) => t.value === item.mission_type_name)) {
    item.mission_type_name = types[0]?.value ?? item.mission_type_name;
  }

  const missions = getMissions(item.category_name, item.mission_type_name);
  if (!missions.length) {
    item.mission_name = null;
  } else {
    if (item.mission_name && missions.some((m) => m.value === item.mission_name)) {
      return;
    }
    item.mission_name = missions[0]?.value ?? null;
  }
}

function createSelect(
  options: { value: string; label: string }[],
  value: string,
  onChange: (value: string) => void,
  disabled = false,
) {
  const sel = document.createElement("select");
  sel.className = "select select--inline";
  sel.disabled = disabled;

  for (const optItem of options) {
    const opt = document.createElement("option");
    opt.value = optItem.value;
    opt.textContent = optItem.label;
    sel.appendChild(opt);
  }

  sel.value = value;
  sel.addEventListener("change", () => onChange(sel.value));
  return sel;
}

function createNumberInput(
  value: number,
  onChange: (value: number) => void,
  disabled = false,
) {
  const input = document.createElement("input");
  input.className = "input input--inline";
  input.type = "number";
  input.min = "0";
  input.step = "1";
  input.disabled = disabled;
  input.value = String(value);

  const parse = () => {
    const n = Number(input.value);
    return Number.isFinite(n) ? Math.max(0, Math.trunc(n)) : 0;
  };

  input.addEventListener("change", () => onChange(parse()));
  input.addEventListener("blur", () => {
    const n = parse();
    input.value = String(n);
  });

  return input;
}

function syncFormValuesForClone(item: HTMLElement, clone: HTMLElement) {
  // SortableJS 在 fallback 拖拽时会 clone DOM；部分浏览器对 <select>/<input> 的当前值不会被 cloneNode 保留，
  // 导致拖拽预览（clone/ghost）显示为“第一个选项/0/空”等默认值。这里把表单值从原节点拷贝到 clone。
  const itemSelects = item.querySelectorAll<HTMLSelectElement>("select");
  const cloneSelects = clone.querySelectorAll<HTMLSelectElement>("select");
  itemSelects.forEach((sel, idx) => {
    const target = cloneSelects[idx];
    if (!target) return;
    const value = sel.value;
    target.value = value;
    target.selectedIndex = sel.selectedIndex;

    // 确保 option.selected 与 selected 属性同步，避免某些环境下 UI 仍显示默认项。
    Array.from(target.options).forEach((opt) => {
      const selected = opt.value === value;
      opt.selected = selected;
      if (selected) opt.setAttribute("selected", "selected");
      else opt.removeAttribute("selected");
    });
  });

  const itemInputs = item.querySelectorAll<HTMLInputElement>("input");
  const cloneInputs = clone.querySelectorAll<HTMLInputElement>("input");
  itemInputs.forEach((input, idx) => {
    const target = cloneInputs[idx];
    if (!target) return;
    if (input.type === "checkbox" || input.type === "radio") {
      target.checked = input.checked;
      if (input.checked) target.setAttribute("checked", "checked");
      else target.removeAttribute("checked");
    } else {
      target.value = input.value;
      target.setAttribute("value", input.value);
    }
  });

  const itemTextareas = item.querySelectorAll<HTMLTextAreaElement>("textarea");
  const cloneTextareas = clone.querySelectorAll<HTMLTextAreaElement>("textarea");
  itemTextareas.forEach((ta, idx) => {
    const target = cloneTextareas[idx];
    if (!target) return;
    target.value = ta.value;
    target.textContent = ta.value;
  });
}

function syncSortableDragPreview(evt: { item: HTMLElement; clone: HTMLElement }) {
  const item = evt.item;

  const apply = (target: HTMLElement | null | undefined) => {
    if (!target) return;
    syncFormValuesForClone(item, target);
  };

  apply(evt.clone);
  apply(Sortable.dragged);
  apply(Sortable.clone);
  apply(Sortable.ghost);

  // 部分场景 clone/ghost 会在同一帧后才插入/更新，补一拍同步更稳。
  requestAnimationFrame(() => {
    apply(evt.clone);
    apply(Sortable.dragged);
    apply(Sortable.clone);
    apply(Sortable.ghost);
  });
}

function renderTable() {
  // 兼容旧调用点：统一走新的卡片式列表渲染
  renderPlanList();
  renderHistoryList();
}

function renderPlanList() {
  const listEl = document.getElementById("plan-list") as HTMLElement | null;
  const config = state.chargePlan.config;
  if (!listEl || !config) return;
  listEl.innerHTML = "";

  const field = (label: string, control: HTMLElement, extraClass = "") => {
    const wrap = document.createElement("label");
    wrap.className = `plan-field ${extraClass}`.trim();
    const l = document.createElement("span");
    l.className = "plan-field__label";
    l.textContent = label;
    wrap.appendChild(l);
    wrap.appendChild(control);
    return wrap;
  };

  const fieldTooltip = (label: string, control: HTMLElement, extraClass = "") => {
    if (
      control instanceof HTMLInputElement ||
      control instanceof HTMLSelectElement ||
      control instanceof HTMLTextAreaElement
    ) {
      control.setAttribute("title", label);
      control.setAttribute("aria-label", label);
    } else {
      control.setAttribute("title", label);
      control.setAttribute("aria-label", label);
    }

    const wrap = field(label, control, `plan-field--tooltip ${extraClass}`.trim());
    wrap.setAttribute("title", label);
    return wrap;
  };

  for (let index = 0; index < config.plan_list.length; index++) {
    const item = config.plan_list[index];
    ensureValidSelection(item);

    const card = document.createElement("div");
    card.className = "plan-card";
    card.dataset.planId = item.plan_id;
    card.classList.toggle("plan-card--done", isCompleted(item));

    const icon = document.createElement("div");
    icon.className = "plan-card__icon";
    icon.title = "计划项目";
    icon.appendChild(iconSvg(ListTodo, 18));
    card.appendChild(icon);

    const content = document.createElement("div");
    content.className = "plan-card__content";
    card.appendChild(content);

    const rowTop = document.createElement("div");
    rowTop.className = "plan-row plan-row--top";
    content.appendChild(rowTop);

    const hasAutoBattle = item.predefined_team_idx === -1;

    const categoryOptions = (state.compendium?.categories ?? []).map((c) => ({
      value: c,
      label: c,
    }));
    const categorySel = createSelect(categoryOptions, item.category_name, (v) => {
      item.category_name = v;
      const types = getMissionTypes(v);
      item.mission_type_name = types[0]?.value ?? "";
      const missions = getMissions(v, item.mission_type_name);
      item.mission_name = missions[0]?.value ?? null;
      scheduleAutoSave();
      renderPlanList();
    });
    rowTop.appendChild(fieldTooltip("分类", categorySel, "plan-field--cat"));

    const typeOptions = getMissionTypes(item.category_name).map((t) => ({
      value: t.value,
      label: t.label,
    }));
    const typeSel = createSelect(typeOptions, item.mission_type_name, (v) => {
      item.mission_type_name = v;
      const missions = getMissions(item.category_name, v);
      item.mission_name = missions[0]?.value ?? null;
      scheduleAutoSave();
      renderPlanList();
    });
    rowTop.appendChild(fieldTooltip("类型", typeSel, "plan-field--type"));

    const missions = getMissions(item.category_name, item.mission_type_name);
    const missionOptions = [
      { value: "", label: "（无/不需要）" },
      ...missions.map((m) => ({ value: m.value, label: m.label })),
    ];
    const missionDisabled = missions.length === 0;
    const missionSel = createSelect(
      missionOptions,
      missionDisabled ? "" : item.mission_name ?? "",
      (v) => {
        item.mission_name = v ? v : null;
        scheduleAutoSave();
      },
      missionDisabled,
    );
    rowTop.appendChild(
      fieldTooltip(
        "关卡",
        missionSel,
        hasAutoBattle ? "plan-field--mission" : "plan-field--mission-wide",
      ),
    );

    const cardDisabled = item.category_name !== "实战模拟室";
    const cardOptions = CARD_NUM_ALLOWED.map((v) => ({ value: v, label: v }));
    const cardSel = createSelect(
      cardOptions,
      item.card_num,
      (v) => {
        item.card_num = v;
        scheduleAutoSave();
      },
      cardDisabled,
    );
    rowTop.appendChild(fieldTooltip("卡片数", cardSel, "plan-field--card"));

    const teamOptions = [
      { value: "-1", label: "-1（游戏内配队）" },
      ...state.teams.map((t) => ({
        value: String(t.idx),
        label: `${t.idx}（${t.name}）`,
      })),
    ];
    const teamSel = createSelect(teamOptions, String(item.predefined_team_idx), (v) => {
      item.predefined_team_idx = Number(v);
      const teamIdx = item.predefined_team_idx;
      if (teamIdx !== -1) {
        const t = state.teams.find((x) => x.idx === teamIdx);
        if (t && t.auto_battle) item.auto_battle_config = t.auto_battle;
      }
      scheduleAutoSave();
      renderPlanList();
    });
    rowTop.appendChild(
      fieldTooltip(
        "配队",
        teamSel,
        hasAutoBattle ? "plan-field--team" : "plan-field--team-wide",
      ),
    );

    if (hasAutoBattle) {
      const autoOptions = (state.autoBattles.length
        ? state.autoBattles
        : ["全配队通用"]
      ).map((ab) => ({ value: ab, label: ab }));
      const autoSel = createSelect(autoOptions, item.auto_battle_config, (v) => {
        item.auto_battle_config = v;
        scheduleAutoSave();
      });
      rowTop.appendChild(
        fieldTooltip("自动战斗配置", autoSel, "plan-field--auto"),
      );
    }

    const rowBottom = document.createElement("div");
    rowBottom.className = "plan-row plan-row--bottom";
    content.appendChild(rowBottom);

    const runInput = createNumberInput(item.run_times, (v) => {
      item.run_times = v;
      card.classList.toggle("plan-card--done", isCompleted(item));
      scheduleAutoSave();
    });
    rowBottom.appendChild(field("已运行次数", runInput, "plan-field--run"));

    const planInput = createNumberInput(item.plan_times, (v) => {
      item.plan_times = v;
      card.classList.toggle("plan-card--done", isCompleted(item));
      scheduleAutoSave();
    });
    rowBottom.appendChild(field("计划次数", planInput, "plan-field--plan"));

    const ops = document.createElement("div");
    ops.className = "plan-ops";
    rowBottom.appendChild(ops);

    const mkIcon = (
      iconNode: IconNode,
      title: string,
      danger: boolean,
      onClick: () => void,
    ) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = `icon-btn ${danger ? "icon-btn--danger" : ""}`.trim();
      b.title = title;
      b.setAttribute("aria-label", title);
      b.appendChild(iconSvg(iconNode, 16));
      b.addEventListener("click", onClick);
      return b;
    };

    ops.appendChild(
      mkIcon(ArrowUpToLine, "置顶", false, () => {
        const list = state.chargePlan.config!.plan_list;
        if (index <= 0) return;
        const [moved] = list.splice(index, 1);
        list.unshift(moved);
        scheduleAutoSave();
        renderPlanList();
      }),
    );
    ops.appendChild(
      mkIcon(ArrowDownToLine, "置底", false, () => {
        const list = state.chargePlan.config!.plan_list;
        if (index >= list.length - 1) return;
        const [moved] = list.splice(index, 1);
        list.push(moved);
        scheduleAutoSave();
        renderPlanList();
      }),
    );
    ops.appendChild(
      mkIcon(Trash2, "删除", true, () => {
        state.chargePlan.config!.plan_list.splice(index, 1);
        scheduleAutoSave();
        renderPlanList();
      }),
    );

    listEl.appendChild(card);
  }

  // 使用 SortableJS 实现拖拽排序（不依赖 HTML5 drag/drop）
  if (planSortable) {
    planSortable.destroy();
    planSortable = null;
  }
  planSortable = new Sortable(listEl, {
    animation: 150,
    forceFallback: true,
    ghostClass: "plan-card--ghost",
    chosenClass: "plan-card--chosen",
    draggable: ".plan-card",
    filter: "select, option, input, textarea, button, a, summary",
    preventOnFilter: false,
    onStart: (evt) => {
      syncSortableDragPreview(evt);
    },
    onClone: (evt) => {
      syncSortableDragPreview(evt);
    },
    onEnd: (evt) => {
      void evt;
      if (!state.chargePlan.config) return;

      // 用 DOM 顺序重建列表，避免 oldIndex/newIndex 在 fallback 模式下偶发不准
      const ids = Array.from(
        listEl.querySelectorAll<HTMLElement>(".plan-card"),
      ).map((el) => el.dataset.planId ?? "");
      const byId = new Map(
        state.chargePlan.config.plan_list.map((x) => [x.plan_id, x] as const),
      );
      const reordered = ids.map((id) => byId.get(id)).filter(Boolean) as ChargePlanItem[];
      if (reordered.length === state.chargePlan.config.plan_list.length) {
        state.chargePlan.config.plan_list = reordered;
        scheduleAutoSave();
      }

      // 避免在 Sortable 的回调栈内 destroy/re-init 导致 DOM 状态不一致
      setTimeout(() => renderPlanList(), 0);
    },
  });
}

function renderHistoryList() {
  const details = document.getElementById("history-details") as HTMLDetailsElement | null;
  const countEl = document.getElementById("history-count") as HTMLElement | null;
  const listEl = document.getElementById("history-list") as HTMLElement | null;
  if (!details || !countEl || !listEl || !state.chargePlan.config) return;

  const items = state.chargePlan.config.history_list ?? [];
  if (!items.length) {
    details.hidden = true;
    return;
  }

  details.hidden = false;
  const max = 200;
  countEl.textContent = items.length > max ? `（${items.length}，仅显示前 ${max} 条）` : `（${items.length}）`;

  listEl.innerHTML = "";

  const pill = (text: string) => {
    const el = document.createElement("div");
    el.className = "plan-pill";
    el.textContent = text;
    el.title = text;
    return el;
  };

  const field = (label: string, text: string, extraClass = "") => {
    const wrap = document.createElement("label");
    wrap.className = `plan-field ${extraClass}`.trim();
    const l = document.createElement("span");
    l.className = "plan-field__label";
    l.textContent = label;
    wrap.appendChild(l);
    wrap.appendChild(pill(text));
    return wrap;
  };

  for (let index = 0; index < Math.min(items.length, max); index++) {
    const item = items[index];

    const card = document.createElement("div");
    card.className = "plan-card plan-card--history";

    const icon = document.createElement("div");
    icon.className = "plan-card__icon";
    icon.title = "历史条目";
    icon.appendChild(iconSvg(History, 18));
    card.appendChild(icon);

    const content = document.createElement("div");
    content.className = "plan-card__content";
    card.appendChild(content);

    const rowTop = document.createElement("div");
    rowTop.className = "plan-row plan-row--top";
    content.appendChild(rowTop);

    rowTop.appendChild(field("分类", item.category_name ?? "", "plan-field--cat"));
    rowTop.appendChild(field("类型", item.mission_type_name ?? "", "plan-field--type"));
    rowTop.appendChild(
      field("关卡", item.mission_name ?? "（无/不需要）", "plan-field--mission"),
    );
    rowTop.appendChild(field("卡片数", item.card_num ?? "", "plan-field--card"));
    rowTop.appendChild(
      field("配队", String(item.predefined_team_idx), "plan-field--team"),
    );
    rowTop.appendChild(
      field("自动战斗配置", item.auto_battle_config ?? "", "plan-field--auto"),
    );

    const rowBottom = document.createElement("div");
    rowBottom.className = "plan-row plan-row--bottom";
    content.appendChild(rowBottom);

    rowBottom.appendChild(field("已运行次数", String(item.run_times), "plan-field--run"));
    rowBottom.appendChild(field("计划次数", String(item.plan_times), "plan-field--plan"));

    listEl.appendChild(card);
  }
}

function newPlanItem(): ChargePlanItem {
  return {
    tab_name: "训练",
    category_name: "实战模拟室",
    mission_type_name: "基础材料",
    mission_name: "调查专项",
    level: null,
    auto_battle_config: "全配队通用",
    run_times: 0,
    plan_times: 1,
    card_num: "默认数量",
    predefined_team_idx: -1,
    notorious_hunt_buff_num: 1,
    plan_id: crypto.randomUUID(),
  };
}

async function migrateCopyActiveTab() {
  if (state.activeTab === "charge_plan") {
    const res = await invoke<{ written_path: string }>("migrate_legacy_to_main", {
      instanceIdx: state.currentInstanceIdx,
      mode: "copy",
    });
    setText("root-status", `已迁移到：${res.written_path}`);
    await loadChargePlan(true);
  } else {
    const res = await invoke<{ written_path: string }>(
      "migrate_notorious_hunt_legacy_to_main",
      {
        instanceIdx: state.currentInstanceIdx,
        mode: "copy",
      },
    );
    setText("root-status", `已迁移到：${res.written_path}`);
    await loadHunt(true);
  }
  renderActiveTabUI();
}

function deleteCompleted() {
  if (!state.chargePlan.config) return;
  state.chargePlan.config.plan_list = state.chargePlan.config.plan_list.filter(
    (x) => !isCompleted(x),
  );
  renderTable();
  scheduleAutoSave("charge_plan");
}

function deleteAll() {
  if (!state.chargePlan.config) return;
  state.chargePlan.config.plan_list = [];
  renderTable();
  scheduleAutoSave("charge_plan");
}

function renderTabs() {
  const isCharge = state.activeTab === "charge_plan";

  const tabCharge = document.getElementById("tab-charge-plan");
  const tabHunt = document.getElementById("tab-hunt");
  tabCharge?.classList.toggle("tab--active", isCharge);
  tabHunt?.classList.toggle("tab--active", !isCharge);
  tabCharge?.setAttribute("aria-selected", isCharge ? "true" : "false");
  tabHunt?.setAttribute("aria-selected", !isCharge ? "true" : "false");

  const panelCharge = document.getElementById("panel-charge-plan") as HTMLElement | null;
  const panelHunt = document.getElementById("panel-hunt") as HTMLElement | null;
  panelCharge?.classList.toggle("panel--active", isCharge);
  panelHunt?.classList.toggle("panel--active", !isCharge);
  if (panelCharge) panelCharge.hidden = !isCharge;
  if (panelHunt) panelHunt.hidden = isCharge;

  const actions = document.getElementById("charge-plan-actions") as HTMLElement | null;
  if (actions) actions.hidden = !isCharge;
}

async function switchTab(kind: TabKind) {
  if (state.activeTab === kind) return;
  state.activeTab = kind;
  renderTabs();

  if (!state.projectRoot) return;

  setSaveStatus(`自动保存：加载中（${fmtClock()}）`, "saving");
  try {
    await loadActiveTab(false);
    renderActiveTabUI();
    setSaveStatus(`自动保存：就绪（${fmtClock()}）`, "ok");
  } catch (e) {
    setSaveStatus(`加载失败：${String(e)}`, "err");
  }
}

function renderActiveTabUI() {
  renderTabs();
  renderPathsStatus();
  renderMigrationCard();

  if (state.activeTab === "charge_plan") {
    renderChargePlanHeader();
    renderTable();
    return;
  }

  renderHuntList();
}

function getHuntBossOptions(): { value: string; label: string }[] {
  const types = getMissionTypes(HUNT_CATEGORY_NAME);
  const filtered = types.filter((t) => t.value !== HUNT_FILTER_MISSION_TYPE);
  return filtered.map((t) => ({ value: t.value, label: t.label }));
}

function getTeamLabel(idx: number) {
  const found = state.teams.find((t) => t.idx === idx);
  return found ? `${idx}（${found.name}）` : `${idx}（预备编队）`;
}

function renderHuntList() {
  const listEl = document.getElementById("hunt-list") as HTMLElement | null;
  const config = state.hunt.config;
  if (!listEl || !config) return;
  listEl.innerHTML = "";

  const field = (label: string, control: HTMLElement, extraClass = "") => {
    const wrap = document.createElement("label");
    wrap.className = `plan-field ${extraClass}`.trim();
    const l = document.createElement("span");
    l.className = "plan-field__label";
    l.textContent = label;
    wrap.appendChild(l);
    wrap.appendChild(control);
    return wrap;
  };

  const fieldTooltip = (label: string, control: HTMLElement, extraClass = "") => {
    if (
      control instanceof HTMLInputElement ||
      control instanceof HTMLSelectElement ||
      control instanceof HTMLTextAreaElement
    ) {
      control.setAttribute("title", label);
      control.setAttribute("aria-label", label);
    } else {
      control.setAttribute("title", label);
      control.setAttribute("aria-label", label);
    }

    const wrap = field(label, control, `plan-field--tooltip ${extraClass}`.trim());
    wrap.setAttribute("title", label);
    return wrap;
  };

  const bossOptions = getHuntBossOptions();
  const levelOptions = HUNT_LEVEL_ALLOWED.map((v) => ({ value: v, label: v }));
  const autoOptions = (state.autoBattles.length
    ? state.autoBattles
    : ["全配队通用"]
  ).map((ab) => ({ value: ab, label: ab }));

  for (let index = 0; index < config.plan_list.length; index++) {
    const item = config.plan_list[index];

    const card = document.createElement("div");
    card.className = "plan-card";
    card.dataset.boss = item.mission_type_name;
    card.classList.toggle("plan-card--done", item.run_times >= item.plan_times);

    const icon = document.createElement("div");
    icon.className = "plan-card__icon";
    icon.title = "拖拽排序";
    icon.appendChild(iconSvg(GripVertical, 18));
    card.appendChild(icon);

    const content = document.createElement("div");
    content.className = "plan-card__content";
    card.appendChild(content);

    const rowTop = document.createElement("div");
    rowTop.className = "plan-row plan-row--top";
    content.appendChild(rowTop);

    const bossSel = createSelect(
      bossOptions,
      item.mission_type_name,
      () => void 0,
      true,
    );
    rowTop.appendChild(fieldTooltip("BOSS", bossSel, "plan-field--boss"));

    const levelSel = createSelect(levelOptions, item.level, (v) => {
      item.level = v;
      scheduleAutoSave("notorious_hunt");
    });
    rowTop.appendChild(fieldTooltip("等级", levelSel, "plan-field--level"));

    const teamOptions = [
      { value: "-1", label: "-1（游戏内配队）" },
      ...Array.from({ length: 10 }, (_, i) => ({
        value: String(i),
        label: getTeamLabel(i),
      })),
    ];
    const teamSel = createSelect(
      teamOptions,
      String(item.predefined_team_idx),
      (v) => {
        item.predefined_team_idx = Number(v);
        scheduleAutoSave("notorious_hunt");
        renderHuntList();
      },
    );
    rowTop.appendChild(fieldTooltip("配队", teamSel, "plan-field--team"));

    if (item.predefined_team_idx === -1) {
      const autoSel = createSelect(autoOptions, item.auto_battle_config, (v) => {
        item.auto_battle_config = v;
        scheduleAutoSave("notorious_hunt");
      });
      rowTop.appendChild(fieldTooltip("战斗配置", autoSel, "plan-field--auto"));
    }

    const buffOptions = HUNT_BUFF_ALLOWED.map((b) => ({
      value: String(b.value),
      label: b.label,
    }));
    const buffSel = createSelect(
      buffOptions,
      String(item.notorious_hunt_buff_num),
      (v) => {
        item.notorious_hunt_buff_num = Number(v);
        scheduleAutoSave("notorious_hunt");
      },
    );
    rowTop.appendChild(fieldTooltip("BUFF", buffSel, "plan-field--buff"));

    const rowBottom = document.createElement("div");
    rowBottom.className = "plan-row plan-row--bottom";
    content.appendChild(rowBottom);

    const runInput = createNumberInput(item.run_times, (v) => {
      item.run_times = v;
      card.classList.toggle("plan-card--done", item.run_times >= item.plan_times);
      scheduleAutoSave("notorious_hunt");
    });
    rowBottom.appendChild(field("已运行次数", runInput, "plan-field--run"));

    const planInput = createNumberInput(item.plan_times, (v) => {
      item.plan_times = v;
      card.classList.toggle("plan-card--done", item.run_times >= item.plan_times);
      scheduleAutoSave("notorious_hunt");
    });
    rowBottom.appendChild(field("计划次数", planInput, "plan-field--plan"));

    const ops = document.createElement("div");
    ops.className = "plan-ops";
    rowBottom.appendChild(ops);

    const pinBtn = document.createElement("button");
    pinBtn.type = "button";
    pinBtn.className = "icon-btn";
    pinBtn.title = "置顶";
    pinBtn.setAttribute("aria-label", "置顶");
    pinBtn.appendChild(iconSvg(Pin, 16));
    pinBtn.addEventListener("click", () => {
      const list = state.hunt.config!.plan_list;
      if (index <= 0) return;
      const [moved] = list.splice(index, 1);
      list.unshift(moved);
      scheduleAutoSave("notorious_hunt");
      renderHuntList();
    });
    ops.appendChild(pinBtn);

    listEl.appendChild(card);
  }

  if (huntSortable) {
    huntSortable.destroy();
    huntSortable = null;
  }

  huntSortable = new Sortable(listEl, {
    animation: 150,
    forceFallback: true,
    ghostClass: "plan-card--ghost",
    chosenClass: "plan-card--chosen",
    draggable: ".plan-card",
    handle: ".plan-card__icon",
    filter: "select, option, input, textarea, button, a, summary",
    preventOnFilter: false,
    onStart: (evt) => {
      syncSortableDragPreview(evt);
    },
    onClone: (evt) => {
      syncSortableDragPreview(evt);
    },
    onEnd: (evt) => {
      void evt;
      const cfg = state.hunt.config;
      if (!cfg) return;

      const ids = Array.from(
        listEl.querySelectorAll<HTMLElement>(".plan-card"),
      ).map((el) => el.dataset.boss ?? "");
      const byId = new Map(
        cfg.plan_list.map((x) => [x.mission_type_name, x] as const),
      );

      const reordered: NotoriousHuntItem[] = [];
      for (const id of ids) {
        const it = byId.get(id);
        if (it) reordered.push(it);
      }
      if (reordered.length === cfg.plan_list.length) {
        cfg.plan_list = reordered;
        scheduleAutoSave("notorious_hunt");
      }

      setTimeout(() => renderHuntList(), 0);
    },
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  const settingsDialog = document.getElementById("settings-dialog") as HTMLDialogElement | null;

  const settingsBtn = $<HTMLButtonElement>("#btn-settings");
  settingsBtn.replaceChildren(iconSvg(Settings, 18));

  const settingsCloseTop = document.getElementById("btn-settings-close-top") as HTMLButtonElement | null;
  settingsCloseTop?.replaceChildren(iconSvg(X, 18));

  settingsBtn.addEventListener("click", () => {
    if (!settingsDialog) return;
    if (typeof settingsDialog.showModal === "function") settingsDialog.showModal();
    else settingsDialog.open = true;
  });
  $<HTMLButtonElement>("#btn-settings-close").addEventListener("click", () => {
    if (!settingsDialog) return;
    settingsDialog.close();
  });
  settingsCloseTop?.addEventListener("click", () => {
    settingsDialog?.close();
  });
  settingsDialog?.addEventListener("click", (e) => {
    if (e.target === settingsDialog) settingsDialog.close();
  });
  settingsDialog?.addEventListener("cancel", (e) => {
    e.preventDefault();
    settingsDialog.close();
  });

  ($<HTMLInputElement>("#project-root")).value = storage.getProjectRoot();

  $<HTMLButtonElement>("#tab-charge-plan").addEventListener("click", () => {
    void switchTab("charge_plan");
  });
  $<HTMLButtonElement>("#tab-hunt").addEventListener("click", () => {
    void switchTab("notorious_hunt");
  });
  renderTabs();

  $<HTMLButtonElement>("#btn-pick-root").addEventListener("click", async () => {
    try {
      const result = await open({
        directory: true,
        multiple: false,
        title: "选择 ZenlessZoneZero-OneDragon 项目根目录",
      });
      if (typeof result === "string") {
        ($<HTMLInputElement>("#project-root")).value = result;
      }
    } catch (e) {
      setText("root-status", `打开目录选择器失败：${String(e)}`);
    }
  });

  $<HTMLButtonElement>("#btn-apply-root").addEventListener("click", async () => {
    const root = ($<HTMLInputElement>("#project-root")).value;
    try {
      await applyProjectRoot(root);
      settingsDialog?.close();
    } catch (e) {
      setText("root-status", String(e));
    }
  });

  $<HTMLSelectElement>("#instance-select").addEventListener(
    "change",
    async (e) => {
      const v = Number((e.target as HTMLSelectElement).value);
      state.currentInstanceIdx = v;
      storage.setLastInstance(v);
      try {
        resetLoadedConfigs();
        await loadOptions();
        await loadActiveTab(true);
        renderActiveTabUI();
      } catch (err) {
        setText("root-status", String(err));
      }
    },
  );

  $<HTMLButtonElement>("#btn-reload").addEventListener("click", async () => {
    resetLoadedConfigs();
    await loadOptions();
    await loadActiveTab(true);
    renderActiveTabUI();
  });

  // 配置区：自动保存（无手动保存/校验按钮）
  $<HTMLInputElement>("#cfg-loop").addEventListener("change", () => {
    syncChargePlanFromHeader();
    scheduleAutoSave("charge_plan");
  });
  $<HTMLInputElement>("#cfg-skip").addEventListener("change", () => {
    syncChargePlanFromHeader();
    scheduleAutoSave("charge_plan");
  });
  $<HTMLSelectElement>("#cfg-restore").addEventListener("change", () => {
    syncChargePlanFromHeader();
    scheduleAutoSave("charge_plan");
  });

  $<HTMLButtonElement>("#btn-add").addEventListener("click", () => {
    if (!state.chargePlan.config) return;
    const item = newPlanItem();
    state.chargePlan.config.plan_list.push(item);
    renderTable();
    scheduleAutoSave("charge_plan");
  });

  $<HTMLButtonElement>("#btn-delete-done").addEventListener(
    "click",
    deleteCompleted,
  );
  $<HTMLButtonElement>("#btn-delete-all").addEventListener("click", deleteAll);
  $<HTMLButtonElement>("#btn-migrate-copy").addEventListener(
    "click",
    () => void migrateCopyActiveTab(),
  );

  // 自动填充（不自动应用）：提示用户手动确认
  try {
    const saved = storage.getProjectRoot();
    if (saved) {
      await applyProjectRoot(saved);
      settingsDialog?.close();
      return;
    }
    const res = await invoke<{ found: boolean; reason: string; path?: string | null }>(
      "detect_project_root",
    );
    setText("root-status", res.reason);
    if (res.found && res.path) {
      const input = $<HTMLInputElement>("#project-root");
      if (!input.value.trim()) input.value = res.path;
    }
  } catch (e) {
    setText("root-status", String(e));
  }
});

