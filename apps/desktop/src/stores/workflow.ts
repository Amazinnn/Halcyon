import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CharacterRow, WorkflowDef, WorkflowRunRow } from "../lib/workflow";

export const useWorkflowStore = defineStore("workflow", {
  state: () => ({
    characters: [] as CharacterRow[],
    workflows: [] as WorkflowDef[],
    runs: [] as WorkflowRunRow[],
    currentCharacterId: null as string | null,
    currentWorkflowId: null as string | null,
    initialized: false,
  }),
  actions: {
    async init() {
      if (this.initialized) return;
      this.initialized = true;
      await listen<{ workflowId: string; runId: string; status: string; error: string | null }>(
        "workflow:runs_changed",
        (e) => {
          if (e.payload.workflowId === this.currentWorkflowId) {
            void this.refreshRuns(this.currentWorkflowId);
          }
          void this.refreshWorkflows();
        },
      );
      await this.refreshCharacters();
    },
    async refreshCharacters() {
      try {
        this.characters = await invoke<CharacterRow[]>("characters_list");
        if (!this.currentCharacterId && this.characters.length) {
          this.currentCharacterId = this.characters[0].id;
          await this.refreshWorkflows();
        }
      } catch (e) {
        console.error("[workflow] characters_list failed", e);
      }
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
    async selectCharacter(id: string) {
      this.currentCharacterId = id;
      this.currentWorkflowId = null;
      this.runs = [];
      await this.refreshWorkflows();
    },
    async selectWorkflow(id: string | null) {
      this.currentWorkflowId = id;
      this.runs = [];
      if (id) await this.refreshRuns(id);
    },
    async save(workflow: WorkflowDef): Promise<WorkflowDef> {
      const saved = await invoke<WorkflowDef>("workflow_save", { workflow });
      this.currentWorkflowId = saved.id;
      await this.refreshWorkflows();
      await this.refreshRuns(saved.id);
      return saved;
    },
    async remove(id: string) {
      await invoke("workflow_delete", { id });
      if (this.currentWorkflowId === id) {
        this.currentWorkflowId = null;
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