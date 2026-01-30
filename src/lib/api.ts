import { invoke } from "@tauri-apps/api/core";
import { Node } from "../types";

export const api = {
    getData: async (): Promise<Node[]> => {
        return await invoke("get_data");
    },

    saveData: async (nodes: Node[]): Promise<void> => {
        return await invoke("save_data", { nodes });
    },

    copySnippet: async (id: string): Promise<void> => {
        return await invoke("copy_snippet", { id });
    },

    initVault: async (password: string): Promise<void> => {
        return await invoke("init_vault", { password });
    },

    unlockVault: async (password: string): Promise<boolean> => {
        return await invoke("unlock_vault", { password });
    },

    lockVault: async (): Promise<void> => {
        return await invoke("lock_vault");
    },

    getSettings: async (): Promise<any> => {
        return await invoke("get_settings");
    },

    saveSettings: async (settings: any): Promise<void> => {
        return await invoke("save_settings", { settings });
    },

    getSnippetsPath: async (): Promise<string> => {
        return await invoke("get_snippets_path");
    },

    openSnippetsPath: async (): Promise<void> => {
        return await invoke("open_snippets_path");
    },

    resetVault: async (): Promise<[Node[], any]> => {
        return await invoke("reset_vault");
    },

    isVaultUnlocked: async (): Promise<boolean> => {
        return await invoke("is_vault_unlocked");
    },
};
