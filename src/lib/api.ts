import { invoke } from "@tauri-apps/api/core";
import { Node, AppSettings } from "../types";

export const api = {
    getData: (): Promise<Node[]> => invoke("get_data"),

    saveData: (nodes: Node[]): Promise<void> => invoke("save_data", { nodes }),

    copySnippet: (id: string): Promise<void> => invoke("copy_snippet", { id }),

    initVault: (password: string): Promise<void> => invoke("init_vault", { password }),

    unlockVault: (password: string): Promise<boolean> => invoke("unlock_vault", { password }),

    lockVault: (): Promise<void> => invoke("lock_vault"),

    getSettings: (): Promise<AppSettings> => invoke("get_settings"),

    saveSettings: (settings: AppSettings): Promise<void> => invoke("save_settings", { settings }),

    getSnippetsPath: (): Promise<string> => invoke("get_snippets_path"),

    openSnippetsPath: (): Promise<void> => invoke("open_snippets_path"),

    resetVault: (): Promise<[Node[], AppSettings]> => invoke("reset_vault"),

    isVaultUnlocked: (): Promise<boolean> => invoke("is_vault_unlocked"),
};

