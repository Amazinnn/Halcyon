import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CharacterRow,
  RecentRunRow,
  WorkflowDef,
  WorkflowRunRow,
} from "../lib/workflow";

const KEY_CHAR = "focus.workflow.currentCharacterId";
const KEY_WF = "focus.workflow.currentWorkflowId";

export const useWorkflowStore = defineStore("workflow", {
  state: () => ({
    characters: [] as CharacterRow[],
    workflows: [] as WorkflowDef[],
    runs: [] as WorkflowRunRow[],
    recentRuns: [] as RecentRunRow[],
    currentCharacterId: null as string | null,
    currentWorkflowId: null as string | null,
    initialized: false,
  }),
  actions: {
    async init() {
      if (this.initialized) return;
      this.initialized = true;
      this.currentCharacterId = localStorage.getItem(KEY_CHAR);
      this.currentWorkflowId = localStorage.getItem(KEY_WF);
      await listen<{ workflowId: string; runId: string; status: string; error: string | null }>(
        "workflow:runs_changed",
        (e) => {
          if (e.payload.workflowId === this.currentWorkflowId) {
            void this.refreshRuns(this.currentWorkflowId);
          }
          void this.refreshWorkflows();
          void this.refreshRecentRuns(20);
        },
      );
      await this.refreshCharacters();
    },
    async refreshCharacters() {
      // v1.10.5.1 (#66): never trust a transient empty character list — the
      // Rust side ensures at least the default character exists; retry up to
      // 3 times (500ms apart) before giving up.
      for (let attempt = 0; attempt < 3; attempt++) {
        try {
          this.characters = await invoke<CharacterRow[]>("characters_list");
          if (this.characters.length > 0) break;
        } catch (e) {
          console.error("[workflow] characters_list failed", e);
        }
        if (attempt < 2) await new Promise((r) => setTimeout(r, 500));
      }
      if (!this.currentCharacterId && this.characters.length) {
        this.currentCharacterId = this.characters[0].id;
      }
      if (
        this.currentCharacterId &&
        !this.characters.some((c) => c.id === this.currentCharacterId)
      ) {
        this.currentCharacterId = this.characters[0]?.id ?? null;
      }
      localStorage.setItem(KEY_CHAR, this.currentCharacterId ?? "");
      await this.refreshWorkflows();
    },
    async refreshWorkflows() {
      if (!this.currentCharacterId) return;
      try {
        this.workflows = await invoke<WorkflowDef[]>("workflow_list", {
          characterId: this.currentCharacterId,
        });
        if (
          this.currentWorkflowId &&
          !this.workflows.some((w) => w.id === this.currentWorkflowId)
        ) {
          this.currentWorkflowId = null;
          localStorage.setItem(KEY_WF, "");
        }
      } catch (e) {
        console.error("[workflow] workflow_list failed", e);
      }
    },
    async refreshRuns(workflowId: string) {
      try {
        this.runs = await invoke<WorkflowRunRow[]>("workflow_runs", { id: workflowId });
      } catch (e) {
        console.error("[workflow] workflow_runs failed", e);
      }
    },
    async refreshRecentRuns(limit = 20) {
      try {
        this.recentRuns = await invoke<RecentRunRow[]>("workflow_runs_recent", { limit });
      } catch (e) {
        console.error("[workflow] workflow_runs_recent failed", e);
      }
    },
    async clearRuns() {
      try {
        await invoke("workflow_runs_clear");
        this.recentRuns = [];
        this.runs = [];
      } catch (e) {
        console.error("[workflow] workflow_runs_clear failed", e);
      }
    },
    async selectCharacter(id: string) {
      this.currentCharacterId = id;
      this.currentWorkflowId = null;
      localStorage.setItem(KEY_CHAR, id);
      localStorage.setItem(KEY_WF, "");
      this.runs = [];
      await this.refreshWorkflows();
    },
    async selectWorkflow(id: string | null) {
      this.currentWorkflowId = id;
      this.runs = [];
      localStorage.setItem(KEY_WF, id ?? "");
      if (id) await this.refreshRuns(id);
    },
    async save(workflow: WorkflowDef): Promise<WorkflowDef> {
      const saved = await invoke<WorkflowDef>("workflow_save", { workflow });
      // v1.10.5 (#59): refresh the list BEFORE publishing the new id so any
      // watcher on currentWorkflowId already finds the saved workflow.
      await this.refreshWorkflows();
      this.currentWorkflowId = saved.id;
      localStorage.setItem(KEY_WF, saved.id);
      await this.refreshRuns(saved.id);
      return saved;
    },
    async remove(id: string) {
      await invoke("workflow_delete", { id });
      if (this.currentWorkflowId === id) {
        this.currentWorkflowId = null;
        localStorage.setItem(KEY_WF, "");
        this.runs = [];
      }
      await this.refreshWorkflows();
    },
    async run(id: string) {
      await invoke<string>("workflow_run", { id });
    },
    async cancel(id: string) {
      await invoke("workflow_cancel", { id });
    },
    async copyTo(id: string, targetCharacterId: string, moveSource: boolean) {
      await invoke<WorkflowDef>("workflow_copy", {
        id,
        targetCharacterId,
        moveSource,
      });
      await this.refreshWorkflows();
    },
    async cleanupThreads() {
      await invoke("workflow_cleanup_threads");
    },
  },
});