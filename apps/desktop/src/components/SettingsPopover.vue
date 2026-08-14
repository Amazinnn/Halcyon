<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emit as emitEvent } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "../stores/settings";
import { useAgentStore } from "../stores/agent";
import { useMusicStore } from "../stores/music";
import { useWorkflowStore } from "../stores/workflow";
import AppIcon from "./AppIcon.vue";
import SettingsHelp from "./SettingsHelp.vue";

const emit = defineEmits<{ (e: "close"): void }>();
const settings = useSettingsStore();
const agent = useAgentStore();
const music = useMusicStore();
const workflow = useWorkflowStore();
const petImporting = ref(false);
const petError = ref("");
interface ActivePetInfo {
  id: string;
  horizontalCorrection: number;
  qualityWarnings: string[];
}
const activePetInfo = ref<ActivePetInfo | null>(null);
const correctionSaving = ref(false);
const musicFolder = ref("");

const version = "v1.12.10";
const wallpaperUrl = ref("");
const acrylicOn = ref(true);
const acrylicOpacity = ref(22);
const streamingOn = ref(false);

const apps = ref<string[]>([]);
const appQuery = ref("");
const appsOpen = ref(false);
const newAgentName = ref("");
const newAgentProvider = ref<"codex" | "claude">("codex");
const stateMappingOpen = ref(false);
const petAnimations = ref<string[]>([]);
const petStateMapping = ref<Record<string, string | null>>({});
const PET_STATES = ["resting", "focusing", "working", "waiting", "happy", "troubled"] as const;
const selectedApps = computed(() => new Set(settings.distractionApps));
const filteredApps = computed(() => apps.value.filter((name) => name.toLowerCase().includes(appQuery.value.trim().toLowerCase())));

async function load() {
  await settings.load();
  const b = await invoke<{ acrylicEnabled?: boolean; chatStreamingEnabled?: boolean; acrylicOpacity?: number }>("get_bootstrap");
  acrylicOn.value = !!b.acrylicEnabled;
  streamingOn.value = !!b.chatStreamingEnabled;
  acrylicOpacity.value = b.acrylicOpacity ?? 22;
  const p = await invoke<string | null>("get_wallpaper");
  wallpaperUrl.value = p ? convertFileSrc(p) : "";
  await agent.refreshStatus();
  await refreshAgents();
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

async function changeAcrylicOpacity() {
  try {
    await invoke("set_acrylic_opacity", { opacity: acrylicOpacity.value });
  } catch (e) {
    console.error("[settings] set_acrylic_opacity failed", e);
  }
}

async function toggleStreaming() {
  streamingOn.value = !streamingOn.value;
  try {
    await invoke("set_chat_streaming_enabled", { enabled: streamingOn.value });
  } catch (e) {
    console.error("[settings] set_chat_streaming_enabled failed", e);
    streamingOn.value = !streamingOn.value;
  }
}

async function toggleApps() {
  appsOpen.value = !appsOpen.value;
  if (appsOpen.value && apps.value.length === 0) {
    try {
      apps.value = await invoke<string[]>("list_apps_catalog");
    } catch (e) {
      console.error("[settings] list_running_apps failed", e);
    }
  }
}

async function setAppList(name: string, selected: boolean) {
  const black = settings.distractionApps.filter((x) => x !== name);
  if (selected) black.push(name);
  await settings.setDistractionLists(black);
}

async function chooseMusicFolder() {
  await music.chooseFolder();
  musicFolder.value = music.folder ?? "";
}

async function importPetPack(characterId: string) {
  petError.value = "";
  const sel = await open({ directory: true });
  if (!sel) return;
  petImporting.value = true;
  try {
    await invoke("pet_import_pack", { dir: sel, characterId });
    await refreshAgents();
    void emitEvent("pet:changed", {});
  } catch (e) {
    petError.value = String(e);
  } finally {
    petImporting.value = false;
  }
}

async function removePet(characterId: string) {
  petError.value = "";
  try {
    await invoke("pet_remove_pack", { characterId });
    await refreshAgents();
    void emitEvent("pet:changed", {});
  } catch (e) {
    petError.value = String(e);
  }
}

// M5 (ADR-0022): Agent management — list/delete/open workspace.
const agentList = ref<{ id: string; name: string; tool: "codex" | "claude"; petPackId?: string | null }[]>([]);
const agentError = ref("");

async function refreshAgents() {
  await agent.refreshCharacters();
  agentList.value = agent.characters.map((c) => ({ id: c.id, name: c.name, tool: c.tool, petPackId: c.petPackId }));
  await refreshStateMapping();
}

async function refreshStateMapping() {
  const current = agent.characters.find((entry) => entry.id === agent.characterId);
  if (!current?.petPackId) {
    stateMappingOpen.value = false;
    petAnimations.value = [];
    petStateMapping.value = {};
    activePetInfo.value = null;
    return;
  }
  try {
    const info = await invoke<(ActivePetInfo & { animations: Array<{ id: string }> }) | null>("pet_active");
    activePetInfo.value = info;
    petAnimations.value = info?.animations.map((animation) => animation.id) ?? [];
    petStateMapping.value = await invoke<Record<string, string | null>>("pet_get_state_mapping", {
      characterId: current.id,
    });
  } catch (error) {
    petError.value = String(error);
  }
}

async function setPetCorrection(value: number) {
  if (!agent.characterId || correctionSaving.value) return;
  correctionSaving.value = true;
  petError.value = "";
  try {
    activePetInfo.value = await invoke<ActivePetInfo>("pet_set_horizontal_correction", {
      characterId: agent.characterId,
      horizontalCorrection: value,
    });
  } catch (error) {
    petError.value = String(error);
    await refreshStateMapping();
  } finally {
    correctionSaving.value = false;
  }
}

async function setPetStateMapping(state: string, value: string) {
  if (!agent.characterId) return;
  const next = { ...petStateMapping.value, [state]: value || null };
  petStateMapping.value = next;
  try {
    await invoke("pet_save_state_mapping", { characterId: agent.characterId, mapping: next });
    void emitEvent("pet:changed", {});
  } catch (error) {
    petError.value = String(error);
    await refreshStateMapping();
  }
}

async function createAgent() {
  agentError.value = "";
  try {
    await agent.createCharacter(newAgentName.value, newAgentProvider.value);
    newAgentName.value = "";
    await refreshAgents();
  } catch (e) { agentError.value = String(e); }
}

async function setCurrentAgent(id: string) {
  try { await agent.setCurrentCharacter(id); await settings.load(); await refreshAgents(); }
  catch (e) { agentError.value = String(e); }
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
  let workflows = 0;
  try { workflows = await invoke<number>("agent_workflow_reference_count", { characterId: id }); }
  catch (e) { agentError.value = String(e); return; }
  if (!window.confirm(`删除该 Agent？将删除其桌宠和 ${workflows} 个关联工作流，但保留工作区文件。`)) return;
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
      <SettingsHelp summary="只影响本轮锁定强度；开始后固定，暂停会完全解锁。" detail="轻度不锁定；标准拦截系统切换快捷键；学霸还会隐藏桌面与任务栏。休息和自然结束会恢复桌面。" />
      <div class="mode-help">
        <p><strong>轻度：</strong>不锁定桌面，任务栏、桌面与快捷键均可使用。</p>
        <p><strong>标准：</strong>拦截 Win、Alt+Tab、Alt+F4 与 Ctrl+Esc，保留任务栏和桌面。</p>
        <p><strong>学霸：</strong>在标准基础上隐藏任务栏与桌面图标。</p>
      </div>
    </section>

    <section class="group">
      <h4>壁纸</h4>
      <SettingsHelp summary="导入后作为主界面背景；支持 PNG、JPG、JPEG、WebP。" detail="Focus 会复制所选图片到本机数据目录；重置只恢复默认背景，不影响其他设置。" />
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
      <SettingsHelp summary="控制毛玻璃、提示音与顶条显示，不改变计时或锁定规则。" detail="顶条只展示当前计时和 Agent 状态。毛玻璃仅影响内部浮窗的视觉效果。" />
      <div class="row">
        <span class="label">毛玻璃</span>
        <button class="switch" :class="{ on: acrylicOn }" @click="toggleAcrylic">
          {{ acrylicOn ? "开" : "关" }}
        </button>
      </div>
      <div class="row">
        <span class="label">玻璃透明度 {{ acrylicOpacity }}%</span>
        <input
          class="correction-range"
          type="range"
          min="5"
          max="100"
          v-model.number="acrylicOpacity"
          :disabled="!acrylicOn"
          @change="changeAcrylicOpacity"
        />
      </div>
      <div class="row">
        <span class="label">显示流式输出</span>
        <button class="switch" :class="{ on: streamingOn }" @click="toggleStreaming">
          {{ streamingOn ? "开" : "关" }}
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
      <SettingsHelp summary="手动设置下一轮专注与休息时长，不会中断当前计时。" detail="专注可设为 1-240 分钟，休息可设为 1-120 分钟；没有内置预设。" />
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
      <h4>应用提醒</h4>
      <SettingsHelp summary="黑名单应用在专注时只提醒，不会强制关闭；未选择时不监测。" detail="列表合并已安装程序和当前可见窗口。匹配使用精确 exe 名称，不支持正则或通配符。" />
      <div class="row col">
        <button class="btn" @click="toggleApps">
          {{ appsOpen ? "收起应用列表" : `管理应用（已选 ${selectedApps.size}）` }}
        </button>
        <div v-if="appsOpen" class="app-list">
          <input v-model="appQuery" class="text-input" placeholder="搜索应用或 exe 名" />
          <div v-for="name in filteredApps" :key="name" class="app-row">
            <span class="app-name" :title="name">{{ name }}</span>
            <span class="app-actions">
              <button class="mini" :class="{ on: settings.distractionApps.includes(name) }" title="黑名单应用" @click="setAppList(name, !settings.distractionApps.includes(name))">黑名单</button>
            </span>
          </div>
        </div>
      </div>
    </section>


    <section class="group">
      <h4>Agent</h4>
      <SettingsHelp summary="每个 Agent 只能有一个桌宠；无桌宠时桌面不显示宠物。" detail="导入官方 Hatch Pet 原样 pet.json，或 format 为 focus-hatch-pet 的 Focus 包。图集路径、网格与单元尺寸都由 JSON 声明；包复制到 Agent 工作区。Provider 登录由 Codex 或 Claude CLI 自行管理。" />
      <div v-if="agentError" class="err">{{ agentError }}</div>
      <div v-if="petError" class="err">{{ petError }}</div>
      <div class="row">
        <input v-model="newAgentName" class="text-input" placeholder="Agent 名称" @keydown.enter="createAgent" />
        <select v-model="newAgentProvider" class="provider-select"><option value="codex">Codex</option><option value="claude">Claude</option></select>
        <button class="mini" @click="createAgent">添加</button>
      </div>
      <div v-if="agentList.length" class="pack-list">
        <div v-for="a in agentList" :key="a.id" class="pack-row">
          <button class="pack-name" :class="{ active: a.id === settings.currentAgentId }" @click="setCurrentAgent(a.id)">{{ a.name }}</button>
          <select :value="a.tool" class="provider-select" @change="onAgentProviderChange(a.id, $event)">
            <option value="codex">Codex</option>
            <option value="claude">Claude</option>
          </select>
          <button class="mini" title="打开工作区（编辑 AGENTS.md）" @click="openWorkspace(a.id)">工作区</button>
          <button class="mini" :disabled="petImporting" @click="importPetPack(a.id)">{{ a.petPackId ? "替换宠物" : "导入宠物" }}</button>
          <button v-if="a.petPackId" class="mini" @click="removePet(a.id)">删除宠物</button>
          <button class="mini" title="删除 Agent 与关联工作流" @click="deleteAgent(a.id)">删除</button>
        </div>
      </div>
      <div v-else class="row"><span class="label">无 Agent</span></div>
      <div class="row">
        <span class="label">状态</span>
        <span :class="agent.ready ? 'ok' : 'err'">{{ agent.ready ? `${agent.provider} ready` : `${agent.provider} unavailable` }}</span>
      </div>
      <div class="row"><span class="label">桌宠背景淡化</span><button class="switch" :class="{ on: settings.petBgFade }" @click="settings.setPetBgFade(!settings.petBgFade)">{{ settings.petBgFade ? "开" : "关" }}</button></div>
      <div v-if="activePetInfo" class="pet-correction">
        <div class="row">
          <span class="label">宽高校正 {{ activePetInfo.horizontalCorrection.toFixed(2) }}</span>
          <input
            class="correction-range"
            type="range"
            min="0.75"
            max="1.33"
            step="0.01"
            :disabled="correctionSaving"
            :value="activePetInfo.horizontalCorrection"
            @change="setPetCorrection(Number(($event.target as HTMLInputElement).value))"
          />
          <button class="mini" :disabled="correctionSaving || activePetInfo.horizontalCorrection === 1" @click="setPetCorrection(1)">恢复</button>
        </div>
        <p v-for="warning in activePetInfo.qualityWarnings" :key="warning" class="pet-warning">{{ warning }}</p>
      </div>
      <div v-if="petAnimations.length" class="state-mapping">
        <button class="mapping-toggle" @click="stateMappingOpen = !stateMappingOpen">
          宠物状态映射 <span>{{ stateMappingOpen ? "收起" : "展开" }}</span>
        </button>
        <div v-if="stateMappingOpen" class="mapping-list">
          <label v-for="state in PET_STATES" :key="state" class="mapping-row">
            <span>{{ state }}</span>
            <select :value="petStateMapping[state] ?? ''" @change="setPetStateMapping(state, ($event.target as HTMLSelectElement).value)">
              <option value="">未指定</option>
              <option v-for="animation in petAnimations" :key="animation" :value="animation">{{ animation }}</option>
            </select>
          </label>
        </div>
      </div>
    </section>

    <section class="group">
      <h4>音乐</h4>
      <SettingsHelp summary="选择本地音乐文件夹后扫描播放；支持 MP3、FLAC、M4A。" detail="Focus 仅读取该文件夹中的可播放曲目；更换文件夹不会删除原始音乐文件。" />
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
      <SettingsHelp summary="保留最近的工作流执行结果；清空不会删除工作流。" detail="运行记录用于确认日程的成功、失败或取消状态，不包含完整聊天内容。" />
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
.state-mapping { display: flex; flex-direction: column; gap: 5px; }
.mapping-toggle { display: flex; justify-content: space-between; border: 0; background: transparent; padding: 4px 0; color: var(--text-mid); font: inherit; font-size: 12px; cursor: pointer; }
.mapping-toggle span { color: var(--accent-bright); }
.mapping-list { display: flex; flex-direction: column; gap: 4px; padding: 6px; background: var(--glass-strong); border: 1px solid var(--glass-border); border-radius: var(--r-sm); }
.mapping-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; color: var(--text-mid); font-size: 11px; }
.mapping-row select { min-width: 122px; border: 1px solid var(--glass-border); border-radius: var(--r-sm); background: #101a15; color: var(--text-hi); padding: 2px 5px; font: inherit; }
.pet-correction { display: flex; flex-direction: column; gap: 4px; }
.correction-range { flex: 1; min-width: 72px; accent-color: var(--accent); }
.pet-warning { margin: 0; color: var(--text-low); font-size: 10px; line-height: 1.4; }

</style>
