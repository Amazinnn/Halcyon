<script setup lang="ts">
import { onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emit as emitEvent } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "../stores/settings";
import { useAgentStore } from "../stores/agent";
import { useMusicStore } from "../stores/music";
import { useWorkflowStore } from "../stores/workflow";
import AppIcon from "./AppIcon.vue";

const emit = defineEmits<{ (e: "close"): void }>();
const settings = useSettingsStore();
const agent = useAgentStore();
const music = useMusicStore();
const workflow = useWorkflowStore();
const agentWorkspace = ref("");
const petInfo = ref<{ id: string; displayName: string; description: string } | null>(null);
const pets = ref<{ id: string; displayName: string; description: string }[]>([]);
const petImporting = ref(false);
const petError = ref("");
const musicFolder = ref("");

const version = "v1.12.10";
const wallpaperUrl = ref("");
const acrylicOn = ref(true);

// local mirrors for editors
const taskName = ref("");
const taskMinutes = ref<number | null>(null);
const blackText = ref("");
const whiteText = ref("");
const runningApps = ref<string[]>([]);
const appsOpen = ref(false);

const PRESETS = [
  { label: "25/5", focus: 25, rest: 5 },
  { label: "50/10", focus: 50, rest: 10 },
  { label: "90/15", focus: 90, rest: 15 },
];

async function load() {
  await settings.load();
  const b = await invoke<{ acrylicEnabled?: boolean }>("get_bootstrap");
  acrylicOn.value = !!b.acrylicEnabled;
  const p = await invoke<string | null>("get_wallpaper");
  wallpaperUrl.value = p ? convertFileSrc(p) : "";
  taskName.value = settings.currentTask?.name ?? "";
  taskMinutes.value = settings.currentTask?.estimatedMinutes ?? null;
  blackText.value = settings.distractionApps.join("\n");
  whiteText.value = settings.allowedApps.join("\n");
  await agent.refreshStatus();
  agentWorkspace.value = agent.workspaceDir;
  await refreshAgents();
  await refreshPets();
  musicFolder.value = music.folder ?? (await invoke<string | null>("music_get_folder")) ?? "";
  await workflow.init();
  await workflow.refreshRecentRuns(20);
}

async function pickWallpaper() {
  const sel = await open({
    multiple: false,
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }],
  });
  if (typeof sel === "string") {
    try {
      const saved = await invoke<string>("persist_wallpaper", { src: sel });
      wallpaperUrl.value = convertFileSrc(saved);
    } catch (e) {
      console.error("[settings] wallpaper import failed", e);
    }
  }
}

async function resetWallpaper() {
  await invoke("reset_wallpaper");
  wallpaperUrl.value = "";
}

async function toggleAcrylic() {
  acrylicOn.value = !acrylicOn.value;
  try {
    await invoke("set_acrylic", { enabled: acrylicOn.value });
  } catch (e) {
    console.error("[settings] set_acrylic failed", e);
    acrylicOn.value = !acrylicOn.value;
  }
}

function applyPreset(p: { focus: number; rest: number }) {
  void settings.setFocusDurations(p.focus, p.rest);
}

async function saveTask() {
  const name = taskName.value.trim();
  if (!name) return;
  const id = settings.currentTask?.id ?? `task-${Date.now()}`;
  const saved = await settings.saveTask({
    id,
    name,
    estimatedMinutes: taskMinutes.value ? Math.max(1, taskMinutes.value) : null,
  });
  await settings.setCurrentTask(saved.id);
  taskName.value = saved.name;
}

async function toggleApps() {
  appsOpen.value = !appsOpen.value;
  if (appsOpen.value && runningApps.value.length === 0) {
    try {
      runningApps.value = await invoke<string[]>("list_running_apps");
    } catch (e) {
      console.error("[settings] list_running_apps failed", e);
    }
  }
}

async function addToList(list: "black" | "white", name: string) {
  const current = list === "black" ? blackText.value : whiteText.value;
  const lines = current
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  if (lines.includes(name)) return;
  lines.push(name);
  if (list === "black") blackText.value = lines.join("\n");
  else whiteText.value = lines.join("\n");
  await saveLists();
}

async function saveLists() {
  const black = blackText.value
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  const white = whiteText.value
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  await settings.setDistractionLists(black, white);
}

function onPauseSupervision() {
  void settings.pauseSupervision(30);
}


async function applyAgentWorkspace() {
  try {
    await agent.setWorkspaceDir(agentWorkspace.value.trim());
  } catch (e) {
    console.error("[settings] set workspace failed", e);
  }
}
function onResumeSupervision() {
  void settings.resumeSupervision();
}

async function chooseMusicFolder() {
  await music.chooseFolder();
  musicFolder.value = music.folder ?? "";
}

async function refreshPets() {
  try {
    petInfo.value = await invoke<{ id: string; displayName: string; description: string } | null>("pet_active");
    pets.value = await invoke<{ id: string; displayName: string; description: string }[]>("pet_list_packs");
  } catch (e) {
    console.error("[settings] pet list failed", e);
  }
}

async function importPetPack() {
  petError.value = "";
  const sel = await open({ directory: true });
  if (!sel) return;
  petImporting.value = true;
  try {
    await invoke("pet_import_pack", { dir: sel });
    await refreshPets();
    void emitEvent("pet:changed", {});
  } catch (e) {
    petError.value = String(e);
  } finally {
    petImporting.value = false;
  }
}

async function activatePet(id: string) {
  petError.value = "";
  try {
    await invoke("pet_activate", { id });
    await refreshPets();
    void emitEvent("pet:changed", {});
  } catch (e) {
    petError.value = String(e);
  }
}

async function removePet(id: string) {
  petError.value = "";
  try {
    await invoke("pet_remove_pack", { id });
    await refreshPets();
    void emitEvent("pet:changed", {});
  } catch (e) {
    petError.value = String(e);
  }
}

// M5 (ADR-0022): Agent management — list/delete/open workspace.
const agentList = ref<{ id: string; name: string; tool: "codex" | "claude" }[]>([]);
const agentError = ref("");

async function refreshAgents() {
  await agent.refreshCharacters();
  agentList.value = agent.characters.map((c) => ({ id: c.id, name: c.name, tool: c.tool }));
}

async function setAgentProvider(id: string, provider: "codex" | "claude") {
  agentError.value = "";
  try {
    await agent.setProvider(id, provider);
    await refreshAgents();
    await agent.refreshStatus();
  } catch (e) {
    agentError.value = String(e);
    await refreshAgents();
    if (id === agent.characterId) await agent.refreshStatus();
  }
}

function onAgentProviderChange(id: string, event: Event) {
  const provider = (event.target as HTMLSelectElement).value;
  if (provider === "codex" || provider === "claude") void setAgentProvider(id, provider);
}

async function deleteAgent(id: string) {
  agentError.value = "";
  if (!window.confirm("删除该 Agent？将连带删除其工作区目录（含 AGENTS.md 与会话记录）。")) return;
  try {
    await invoke("agent_delete", { characterId: id });
    if (id === agent.characterId) {
      localStorage.removeItem("focus-agent");
    }
    await refreshAgents();
    void emitEvent("agent:changed", {});
  } catch (e) {
    agentError.value = String(e);
  }
}

async function openWorkspace(id: string) {
  agentError.value = "";
  try {
    await invoke("agent_open_workspace", { characterId: id });
  } catch (e) {
    agentError.value = String(e);
  }
}

function fmtRunTime(ts: number): string {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleString("zh-CN", { hour12: false });
}

async function clearRuns() {
  if (!window.confirm("清空全部工作流运行记录？")) return;
  await workflow.clearRuns();
}

onMounted(load);
</script>

<template>
  <div class="popover glass" @click.stop>
    <div class="head">
      <span class="title">设置</span>
      <button class="ghost" title="关闭" @click="emit('close')"><AppIcon name="close" /></button>
    </div>

    <section class="group">
      <h4>专注锁定模式</h4>
      <p class="group-note">只影响本轮专注的锁定强度；开始后固定，暂停会完全解锁。</p>
      <div class="mode-help">
        <p><strong>轻度：</strong>不锁定桌面，任务栏、桌面与快捷键均可使用。</p>
        <p><strong>标准：</strong>拦截 Win、Alt+Tab、Alt+F4 与 Ctrl+Esc，保留任务栏和桌面。</p>
        <p><strong>学霸：</strong>在标准基础上隐藏任务栏与桌面图标。</p>
      </div>
    </section>

    <section class="group">
      <h4>壁纸</h4>
      <p class="group-note">导入后作为主界面背景；支持 PNG、JPG、JPEG、WebP。</p>
      <div class="row">
        <div class="btns">
          <button @click="pickWallpaper">导入</button>
          <button @click="resetWallpaper">重置</button>
        </div>
        <span v-if="wallpaperUrl" class="ok">已设置</span>
      </div>
    </section>

    <section class="group">
      <h4>外观</h4>
      <p class="group-note">控制毛玻璃、提示音与顶条显示，不改变专注计时或锁定规则。</p>
      <div class="row">
        <span class="label">毛玻璃</span>
        <button class="switch" :class="{ on: acrylicOn }" @click="toggleAcrylic">
          {{ acrylicOn ? "开" : "关" }}
        </button>
      </div>
      <div class="row">
        <span class="label">提示音</span>
        <button class="switch" :class="{ on: settings.soundEnabled }" @click="settings.setSound(!settings.soundEnabled)">
          {{ settings.soundEnabled ? "开" : "关" }}
        </button>
      </div>
      <div class="row">
        <span class="label">顶条</span>
        <div class="seg">
          <button :class="{ on: settings.showTopbar === 'auto' }" @click="settings.setShowTopbar('auto')">自动</button>
          <button :class="{ on: settings.showTopbar === 'on' }" @click="settings.setShowTopbar('on')">常显</button>
          <button :class="{ on: settings.showTopbar === 'off' }" @click="settings.setShowTopbar('off')">隐藏</button>
        </div>
      </div>
    </section>

    <section class="group">
      <h4>计时（下一轮生效）</h4>
      <p class="group-note">设置下一轮的专注与休息时长，不会中断正在进行的计时。</p>
      <div class="row presets">
        <button v-for="p in PRESETS" :key="p.label" @click="applyPreset(p)">{{ p.label }}</button>
      </div>
      <div class="row">
        <span class="label">专注</span>
        <input v-model.number="settings.focusMinutes" type="number" min="1" max="240" class="num-input" />
        <span class="unit">分钟</span>
      </div>
      <div class="row">
        <span class="label">休息</span>
        <input v-model.number="settings.restMinutes" type="number" min="1" max="120" class="num-input" />
        <span class="unit">分钟</span>
      </div>
      <div class="row">
        <button class="btn" @click="settings.setFocusDurations(settings.focusMinutes, settings.restMinutes)">应用时长</button>
      </div>
    </section>

    <section class="group">
      <h4>任务</h4>
      <p class="group-note">为当前专注标记任务名称和可选预计时长，便于回看。</p>
      <div class="row">
        <span class="label">名称</span>
        <input v-model="taskName" type="text" class="text-input" placeholder="当前任务" />
      </div>
      <div class="row">
        <span class="label">预计</span>
        <input v-model.number="taskMinutes" type="number" min="1" max="600" class="num-input" placeholder="分钟" />
      </div>
      <div class="row">
        <button class="btn" @click="saveTask">保存任务</button>
      </div>
    </section>

    <section class="group">
      <h4>监督</h4>
      <p class="group-note">发现分心应用时只提醒，不会强制关闭应用；清单每行一个名称。</p>
      <div class="row">
        <span class="label">启用</span>
        <button class="switch" :class="{ on: settings.supervisionEnabled }" @click="settings.setSupervisionEnabled(!settings.supervisionEnabled)">
          {{ settings.supervisionEnabled ? "开" : "关" }}
        </button>
      </div>
      <div class="row">
        <span class="label">暂停</span>
        <button v-if="!settings.supervisionPaused" class="btn" @click="onPauseSupervision">暂停 30 分钟</button>
        <button v-else class="btn" @click="onResumeSupervision">恢复监督</button>
      </div>
      <div class="row col">
        <span class="label">分心应用（每行一个，支持 *通配*）</span>
        <textarea v-model="blackText" rows="3" class="ta"></textarea>
      </div>
      <div class="row col">
        <span class="label">豁免应用（每行一个）</span>
        <textarea v-model="whiteText" rows="2" class="ta"></textarea>
      </div>
      <div class="row col">
        <button class="btn" @click="toggleApps">
          {{ appsOpen ? "收起运行中的应用" : "从运行中的应用选择（点此展开）" }}
        </button>
        <div v-if="appsOpen" class="app-list">
          <div v-for="name in runningApps" :key="name" class="app-row">
            <span class="app-name" :title="name">{{ name }}</span>
            <span class="app-actions">
              <button class="mini" title="加入分心（黑名单）" @click="addToList('black', name)">黑</button>
              <button class="mini" title="加入豁免（白名单）" @click="addToList('white', name)">白</button>
            </span>
          </div>
        </div>
      </div>
      <div class="row">
        <button class="btn" @click="saveLists">保存清单</button>
      </div>
    </section>


     <section class="group">
      <h4>宠物</h4>
      <p class="group-note">手动导入含 pet.json + spritesheet.webp/png 的文件夹（1536×1872）；生成与质量标准见 README。</p>
      <div class="row">
        <span class="label">当前</span>
        <span class="ok">{{ petInfo?.displayName ?? "内置占位" }}</span>
      </div>
      <div class="row">
        <button class="btn" :disabled="petImporting" @click="importPetPack">
          {{ petImporting ? "导入中…" : "导入宠物包" }}
        </button>
      </div>
      <div v-if="petError" class="err">{{ petError }}</div>
      <div v-if="pets.length" class="pack-list">
        <div v-for="p in pets" :key="p.id" class="pack-row">
          <button class="pack-name" :class="{ active: petInfo?.id === p.id }" @click="activatePet(p.id)">
            {{ p.displayName }}
          </button>
          <button class="mini" title="删除" @click="removePet(p.id)">删除</button>
        </div>
      </div>
      <div class="row">
        <span class="label">背景淡化</span>
        <button class="switch" :class="{ on: settings.petBgFade }" @click="settings.setPetBgFade(!settings.petBgFade)">
          {{ settings.petBgFade ? "开" : "关" }}
        </button>
      </div>
    </section>

    <section class="group">
      <h4>Agent</h4>
      <p class="group-note">每只宠物固定使用 Codex 或 Claude；登录由对应 CLI 自行管理。</p>
      <div v-if="agentError" class="err">{{ agentError }}</div>
      <div v-if="agentList.length" class="pack-list">
        <div v-for="a in agentList" :key="a.id" class="pack-row">
          <span class="pack-name" :class="{ active: a.id === agent.characterId }">{{ a.name }}</span>
          <select :value="a.tool" class="provider-select" @change="onAgentProviderChange(a.id, $event)">
            <option value="codex">Codex</option>
            <option value="claude">Claude</option>
          </select>
          <button class="mini" title="打开工作区（编辑 AGENTS.md）" @click="openWorkspace(a.id)">打开工作区</button>
          <button class="mini" title="删除（连带删工作区）" @click="deleteAgent(a.id)">删除</button>
        </div>
      </div>
      <div v-else class="row"><span class="label">无 Agent</span></div>
      <div class="row">
        <span class="label">状态</span>
        <span :class="agent.ready ? 'ok' : 'err'">{{ agent.ready ? `${agent.provider} ready` : `${agent.provider} unavailable` }}</span>
      </div>
      <div class="row">
        <span class="label">工作区</span>
        <input v-model="agentWorkspace" type="text" class="text-input" placeholder="默认用户主目录" />
      </div>
      <div class="row">
        <button class="btn" @click="applyAgentWorkspace">应用工作区</button>
      </div>
    </section>

    <section class="group">
      <h4>音乐</h4>
      <p class="group-note">选择本地音乐文件夹后扫描播放；支持 MP3、FLAC、M4A。</p>
      <div class="row">
        <span class="label">文件夹</span>
        <span class="text-input folder-path">{{ musicFolder || "未选择" }}</span>
      </div>
      <div class="row">
        <button class="btn" @click="chooseMusicFolder">选择 / 更换</button>
      </div>
    </section>

    <section class="group">
      <h4>运行记录</h4>
      <p class="group-note">保留最近的工作流执行结果；清空只删除这些记录，不删除工作流。</p>
      <div v-if="!workflow.recentRuns.length" class="row">
        <span class="label">暂无工作流运行记录</span>
      </div>
      <div v-else class="run-list">
        <div v-for="r in workflow.recentRuns.slice(0, 12)" :key="r.id" class="run-row">
          <div class="run-main">
            <span class="run-name" :title="r.workflowName">{{ r.workflowName }}</span>
            <span class="run-meta">{{ r.triggeredBy }} · {{ fmtRunTime(r.startedAt) }}</span>
          </div>
          <span class="run-status" :class="r.status">{{ r.status }}</span>
        </div>
      </div>
      <div class="row">
        <button class="btn" @click="clearRuns">清空记录</button>
      </div>
    </section>

    <div class="about">Focus Desktop {{ version }} · AGPLv3</div>
  </div>
</template>

<style scoped>
.popover {
  position: absolute;
  right: 24px;
  bottom: 72px;
  width: 300px;
  max-height: 78vh;
  overflow-y: auto;
  z-index: 30;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-radius: var(--r-md);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.title { font-size: 13px; font-weight: 600; color: var(--text-hi); }
.ghost {
  border: none; background: transparent; color: var(--text-mid);
  border-radius: var(--r-sm); padding: 3px; cursor: pointer; display: inline-flex;
}
.ghost:hover { color: var(--accent); background: var(--accent-wash); }
.group { display: flex; flex-direction: column; gap: 8px; padding-top: 8px; border-top: 1px solid var(--glass-border); }
.group h4 { margin: 0; font-size: 11px; color: var(--text-low); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
.group-note { margin: -2px 0 0; color: var(--text-low); font-size: 11px; line-height: 1.45; }
.row { display: flex; align-items: center; gap: 8px; }
.row.col { flex-direction: column; align-items: stretch; }
.label { font-size: 12px; color: var(--text-mid); flex-shrink: 0; }
.btns { display: flex; gap: 6px; }
.btns button, .btn {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 4px 10px;
  font-size: 12px; cursor: pointer;
}
.btns button:hover, .btn:hover { border-color: var(--accent); color: var(--accent-bright); }
.ok { font-size: 11px; color: var(--accent-bright); }
.switch {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-low); border-radius: var(--r-pill); padding: 4px 14px;
  font-size: 12px; cursor: pointer;
}
.switch.on { background: var(--accent); color: #0a110e; border-color: var(--accent); font-weight: 600; }
.seg { display: flex; gap: 4px; }
.seg button {
  border: 1px solid var(--glass-border); background: transparent; color: var(--text-mid);
  border-radius: var(--r-sm); padding: 3px 10px; font-size: 12px; cursor: pointer;
}
.seg button.on { background: var(--accent-wash); color: var(--accent-bright); border-color: var(--accent); }
.presets { gap: 6px; }
.presets button {
  border: 1px solid var(--glass-border); background: transparent; color: var(--text-mid);
  border-radius: var(--r-sm); padding: 3px 12px; font-size: 12px; cursor: pointer;
}
.presets button:hover { border-color: var(--accent); color: var(--accent-bright); }
.num-input, .text-input, .ta {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 4px 8px; font-size: 12px;
  font-family: inherit;
}
.num-input { width: 64px; }
.text-input { flex: 1; min-width: 0; }
.ta { width: 100%; resize: vertical; }
.unit { font-size: 11px; color: var(--text-low); }
.folder-path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.run-list { display: flex; flex-direction: column; gap: 3px; max-height: 180px; overflow-y: auto; }
.run-row {
  display: flex; align-items: center; justify-content: space-between; gap: 6px;
  padding: 3px 6px; border-radius: var(--r-sm); background: var(--glass-strong);
}
.run-main { display: flex; flex-direction: column; min-width: 0; }
.run-name { font-size: 11px; color: var(--text-hi); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.run-meta { font-size: 9px; color: var(--text-low); }
.run-status { font-size: 10px; padding: 1px 6px; border-radius: var(--r-pill); }
.run-status.success { color: #2ecc71; background: rgba(46, 204, 113, 0.12); }
.run-status.failed { color: #ff5555; background: rgba(255, 85, 85, 0.12); }
.run-status.cancelled { color: #e8c766; background: rgba(232, 199, 102, 0.12); }
.run-status.skipped, .run-status.running { color: var(--text-low); background: rgba(255, 255, 255, 0.06); }
.app-list {
  max-height: 180px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 2px;
}
.app-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 3px 6px;
  border-radius: var(--r-sm);
}
.app-row:hover { background: var(--accent-wash); }
.app-name {
  font-size: 11px;
  color: var(--text-mid);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.app-actions { display: inline-flex; gap: 4px; flex-shrink: 0; }
.mini {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  padding: 2px 8px;
  font-size: 11px;
  cursor: pointer;
}
.mini:hover { border-color: var(--accent); color: var(--accent-bright); }
.err { font-size: 11px; color: var(--err); }
.pack-list { display: flex; flex-direction: column; gap: 2px; max-height: 140px; overflow-y: auto; }
.pack-row { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
.pack-name {
  flex: 1; min-width: 0; text-align: left;
  border: none; background: transparent; color: var(--text-hi);
  padding: 4px 8px; border-radius: var(--r-sm); cursor: pointer; font-size: 12px;
}
.pack-name:hover { background: var(--accent-wash); color: var(--accent-bright); }
.pack-name.active { background: var(--accent-wash); color: var(--accent-bright); }
.provider-select {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 2px 5px;
  font-size: 11px; cursor: pointer;
}
.about { font-size: 11px; color: var(--text-low); border-top: 1px solid var(--glass-border); padding-top: 8px; }
.mode-help { display: flex; flex-direction: column; gap: 5px; }
.mode-help p { margin: 0; color: var(--text-mid); font-size: 12px; line-height: 1.45; }
.mode-help strong { color: var(--accent-bright); }
.seg { display: flex; gap: 4px; }
.seg button {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-mid); border-radius: var(--r-sm); padding: 3px 10px;
  font-size: 12px; cursor: pointer;
}
.seg button.on { background: var(--accent); color: #0a110e; border-color: var(--accent); }
.text-input {
  flex: 1; border: 1px solid var(--glass-border); background: #101a15;
  color: var(--text-hi); border-radius: var(--r-sm); padding: 4px 8px; font-size: 12px;
}

</style>
