import { useState, useEffect, useRef } from "react";
import { AppSettings, BackupInfo } from "@/types";
import { api } from "@/lib/api";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Shield, Lock, Settings as SettingsIcon, Database, ExternalLink, FileJson, AlertCircle, Bell, Power, Github, RefreshCw, CheckCircle2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isEnabled } from "@tauri-apps/plugin-autostart";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";

interface SettingsProps {
    settings: AppSettings;
    onResetTrigger: () => void;
    onSetupTrigger: () => void;
    onSettingsUpdate: (settings: AppSettings) => void;
}

export function Settings({ settings, onResetTrigger, onSetupTrigger, onSettingsUpdate }: SettingsProps) {
    const [snippetsPath, setSnippetsPath] = useState<string>("");
    const [confirmReset, setConfirmReset] = useState(false);
    const [appVersion, setAppVersion] = useState("");
    const [autostartLoading, setAutostartLoading] = useState(false);
    const [backups, setBackups] = useState<BackupInfo[]>([]);
    const [confirmRestore, setConfirmRestore] = useState<string | null>(null);
    const [creatingBackup, setCreatingBackup] = useState(false);
    const [restoringBackup, setRestoringBackup] = useState<string | null>(null);
    const [backupNotice, setBackupNotice] = useState<{ type: "success" | "error"; message: string } | null>(null);
    const confirmRestoreTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

    const loadBackups = async () => {
        try {
            const list = await api.getBackups();
            setBackups(list);
            return true;
        } catch (error) {
            console.error("Failed to load backups", error);
            setBackupNotice({ type: "error", message: "Could not load backup history." });
            return false;
        }
    };

    useEffect(() => {
        import('@tauri-apps/api/app').then(app => {
            app.getVersion().then(setAppVersion);
        });

        const fetchPath = async () => {
            try {
                const path = await api.getSnippetsPath();
                setSnippetsPath(path);
            } catch (error) {
                console.error("Failed to fetch snippets path", error);
            }
        };
        fetchPath();

        const syncAutostart = async () => {
            try {
                const enabled = await isEnabled();
                if (settings.launchAtStartup !== enabled) {
                    onSettingsUpdate({ ...settings, launchAtStartup: enabled });
                }
            } catch (error) {
                console.error("Failed to sync autostart state", error);
            }
        };
        syncAutostart();

        loadBackups();

        return () => {
            if (confirmRestoreTimer.current) {
                clearTimeout(confirmRestoreTimer.current);
            }
        };
    }, []);

    const handleCreateBackup = async () => {
        setCreatingBackup(true);
        setBackupNotice(null);

        try {
            await api.createBackup();
            if (await loadBackups()) {
                setBackupNotice({ type: "success", message: "Backup created." });
            }
        } catch (error) {
            console.error("Failed to create backup", error);
            setBackupNotice({ type: "error", message: "Could not create a backup." });
        } finally {
            setCreatingBackup(false);
        }
    };

    const handleRestoreBackup = async (filename: string) => {
        if (confirmRestore !== filename) {
            if (confirmRestoreTimer.current) {
                clearTimeout(confirmRestoreTimer.current);
            }
            setConfirmRestore(filename);
            confirmRestoreTimer.current = setTimeout(() => setConfirmRestore(null), 3000);
            return;
        }

        if (confirmRestoreTimer.current) {
            clearTimeout(confirmRestoreTimer.current);
            confirmRestoreTimer.current = null;
        }
        setRestoringBackup(filename);
        setBackupNotice(null);

        try {
            await api.restoreBackup(filename);
            setConfirmRestore(null);
            if (await loadBackups()) {
                setBackupNotice({
                    type: "success",
                    message: "Backup restored. Your previous data was saved as a new backup."
                });
            }
        } catch (error) {
            console.error("Failed to restore backup", error);
            setBackupNotice({ type: "error", message: "Could not restore this backup." });
        } finally {
            setRestoringBackup(null);
        }
    };

    const handleAutoStartChange = async (enabled: boolean) => {
        setAutostartLoading(true);
        try {
            if (enabled) {
                await invoke("plugin:autostart|enable");
            } else {
                await invoke("plugin:autostart|disable");
            }
            const actualState = await isEnabled();
            onSettingsUpdate({ ...settings, launchAtStartup: actualState });
        } catch (error) {
            console.error("Failed to toggle autostart", error);
            try {
                const actualState = await isEnabled();
                onSettingsUpdate({ ...settings, launchAtStartup: actualState });
            } catch {
                // If we can't even get state, leave toggle as-is
            }
        } finally {
            setAutostartLoading(false);
        }
    };


    return (
        <div className="p-8 flex flex-col gap-8 animate-fade-in max-w-2xl mx-auto">
            <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                    <div className="p-2 rounded-lg bg-primary/10 border border-primary/20">
                        <SettingsIcon className="w-6 h-6 text-primary" />
                    </div>
                    <div>
                        <h1 className="text-2xl font-bold tracking-tight">Sklad Settings</h1>
                        <p className="text-muted-foreground">Configure your industrial snippet storage.</p>
                    </div>
                </div>
                <div className="flex items-center gap-2">
                    <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8 text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
                        onClick={async () => {
                            try {
                                await openUrl('https://github.com/Rench321/sklad');
                            } catch (error) {
                                console.error('Failed to open link', error);
                            }
                        }}
                        title="View source on GitHub"
                    >
                        <Github className="h-4 w-4" />
                    </Button>
                    {appVersion && (
                        <div className="px-3 py-1 rounded-full bg-muted/50 border border-border/50 text-xs font-mono text-muted-foreground">
                            v{appVersion}
                        </div>
                    )}
                </div>
            </div>

            <Card className="glass border-border/50">
                <CardHeader>
                    <div className="flex items-center gap-2">
                        <Database className="w-5 h-5 text-primary" />
                        <CardTitle className="text-lg">Storage</CardTitle>
                    </div>
                    <CardDescription>
                        Configuration files and snippet storage location.
                    </CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                    <div className="flex flex-col gap-3 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <FileJson className="w-4 h-4 text-primary/70" />
                                <span className="text-sm font-semibold">Snippets Database</span>
                            </div>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => api.openSnippetsPath()}
                                className="h-8 text-xs gap-2"
                            >
                                <ExternalLink className="w-3 h-3" />
                                Open File
                            </Button>
                        </div>
                        <div className="p-2.5 rounded bg-background/50 border border-border/40 font-mono text-[11px] break-all text-muted-foreground select-all">
                            {snippetsPath || "Loading path..."}
                        </div>
                        <p className="text-[10px] text-muted-foreground/60 italic">
                            Snippets and folders are stored here. Encrypted libraries also include the vault metadata needed for password-based recovery; other preferences remain in settings.json.
                        </p>
                    </div>

                    <div className="flex flex-col gap-3 p-4 rounded-xl bg-muted/30 border border-border/50 animate-fade-in">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <AlertCircle className="w-4 h-4 text-primary/70" />
                                <span className="text-sm font-semibold">Application Logs</span>
                            </div>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={async () => {
                                    try {
                                        await api.openAppLogsDir();
                                    } catch (error) {
                                        console.error("Failed to open logs directory", error);
                                    }
                                }}
                                className="h-8 text-xs gap-2"
                            >
                                <ExternalLink className="w-3 h-3" />
                                Open Folder
                            </Button>
                        </div>
                        <p className="text-[10px] text-muted-foreground/60 italic">
                            System logs and diagnostics to help troubleshoot any issues. Does not contain any snippets or secure info.
                        </p>
                        <div className="flex items-center justify-between border-t border-border/40 pt-3 mt-1">
                            <span className="text-xs text-muted-foreground font-medium">Enable Diagnostic Logging</span>
                            <Switch
                                checked={settings.loggingEnabled || false}
                                onCheckedChange={(checked) => {
                                    onSettingsUpdate({
                                        ...settings,
                                        loggingEnabled: checked
                                    });
                                }}
                            />
                        </div>
                    </div>

                    <div className="flex flex-col gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <RefreshCw className="w-4 h-4 text-primary/70" />
                                <Label htmlFor="automatic-backups" className="text-sm font-semibold">
                                    Automatic Backups
                                </Label>
                            </div>
                            <Switch
                                id="automatic-backups"
                                checked={settings.autoBackupEnabled ?? true}
                                onCheckedChange={(checked) => {
                                    onSettingsUpdate({
                                        ...settings,
                                        autoBackupEnabled: checked
                                    });
                                }}
                            />
                        </div>

                        {settings.autoBackupEnabled && (
                            <div className="flex items-center justify-between animate-fade-in">
                                <Label htmlFor="backup-count" className="text-xs text-muted-foreground">
                                    Number of backups to keep
                                </Label>
                                <input
                                    id="backup-count"
                                    type="number"
                                    min="1"
                                    max="20"
                                    className="h-10 w-16 rounded-md border border-border/50 bg-background/50 px-2 text-center text-sm tabular-nums focus:outline-none focus:ring-1 focus:ring-primary"
                                    value={settings.autoBackupCount ?? 5}
                                    onChange={(e) => {
                                        const count = parseInt(e.target.value) || 5;
                                        onSettingsUpdate({
                                            ...settings,
                                            autoBackupCount: Math.min(20, Math.max(1, count))
                                        });
                                    }}
                                />
                            </div>
                        )}

                        <p className="text-[10px] text-muted-foreground/60 italic">
                            {settings.autoBackupEnabled
                                ? "Backups are created on startup, after saving, and before closing."
                                : "Automatic backups are off. You can still create and restore them manually."}
                        </p>

                        <div className="border-t border-border/40 pt-4 space-y-3">
                            <div className="flex items-center justify-between gap-4">
                                <div>
                                    <div className="text-xs font-medium">
                                        Backup History <span className="text-muted-foreground tabular-nums">({backups.length})</span>
                                    </div>
                                    <p className="mt-0.5 text-[10px] text-pretty text-muted-foreground/60">
                                        Restore points stay on this device. New backups containing encrypted snippets include the vault metadata required by the existing master password.
                                    </p>
                                </div>
                                <Button
                                    variant="outline"
                                    size="sm"
                                    className="h-10 shrink-0 gap-2 px-3 transition-[transform,background-color,color,box-shadow] duration-150 ease-out active:not-disabled:scale-[0.96]"
                                    onClick={handleCreateBackup}
                                    disabled={creatingBackup || restoringBackup !== null}
                                >
                                    <RefreshCw className={cn("size-4", creatingBackup && "animate-spin")} />
                                    {creatingBackup ? "Creating…" : "Back up now"}
                                </Button>
                            </div>

                            {backups.length > 0 ? (
                                <div className="max-h-48 space-y-1 overflow-y-auto pr-1">
                                    {backups.map((backup) => {
                                        const isConfirming = confirmRestore === backup.filename;
                                        const isRestoring = restoringBackup === backup.filename;

                                        return (
                                            <div
                                                key={backup.filename}
                                                className="flex min-h-12 items-center justify-between gap-3 rounded-lg border border-border/40 bg-background/50 py-1 pl-3 pr-1 text-xs"
                                            >
                                                <div className="flex min-w-0 items-center gap-2">
                                                    {backup.hasVaultMetadata && (
                                                        <Shield
                                                            aria-label="Includes vault recovery metadata"
                                                            className="size-3.5 shrink-0 text-primary/70"
                                                        />
                                                    )}
                                                    <span className="min-w-0 truncate font-mono tabular-nums text-muted-foreground">
                                                        {new Date(backup.timestamp).toLocaleString()}
                                                    </span>
                                                </div>
                                                <div className="flex shrink-0 items-center gap-1">
                                                    <span className="w-14 text-right tabular-nums text-muted-foreground/60">
                                                        {(backup.size / 1024).toFixed(1)} KB
                                                    </span>
                                                    <Button
                                                        variant={isConfirming ? "destructive" : "ghost"}
                                                        size="sm"
                                                        className="h-10 w-20 px-2 text-xs transition-[transform,background-color,color,box-shadow] duration-150 ease-out active:not-disabled:scale-[0.96]"
                                                        onClick={() => handleRestoreBackup(backup.filename)}
                                                        disabled={creatingBackup || restoringBackup !== null}
                                                    >
                                                        {isRestoring ? "Restoring…" : isConfirming ? "Restore?" : "Restore"}
                                                    </Button>
                                                </div>
                                            </div>
                                        );
                                    })}
                                </div>
                            ) : (
                                <div className="rounded-lg bg-background/30 px-3 py-4 text-center text-xs italic text-muted-foreground/60">
                                    No backups yet
                                </div>
                            )}

                            {backupNotice && (
                                <div
                                    role="status"
                                    aria-live="polite"
                                    className={cn(
                                        "flex items-start gap-2 rounded-lg px-3 py-2 text-xs text-pretty animate-fade-in",
                                        backupNotice.type === "success"
                                            ? "bg-primary/10 text-foreground"
                                            : "bg-destructive/10 text-destructive"
                                    )}
                                >
                                    {backupNotice.type === "success" ? (
                                        <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-primary" />
                                    ) : (
                                        <AlertCircle className="mt-0.5 size-3.5 shrink-0" />
                                    )}
                                    <span>{backupNotice.message}</span>
                                </div>
                            )}
                        </div>
                    </div>
                </CardContent>
            </Card>

            <Card className="glass border-border/50">
                <CardHeader>
                    <div className="flex items-center gap-2">
                        <Bell className="w-5 h-5 text-primary" />
                        <CardTitle className="text-lg">System</CardTitle>
                    </div>
                    <CardDescription>
                        Control how Sklad interacts with your system.
                    </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="autosave" className="text-base font-semibold">
                                Autosave snippets
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                Automatically save changes to snippets as you type. If turned off, Sklad will prompt you to Save or Discard changes.
                            </p>
                        </div>
                        <Switch
                            id="autosave"
                            checked={settings.autoSave}
                            onCheckedChange={(checked) => {
                                onSettingsUpdate({
                                    ...settings,
                                    autoSave: checked
                                });
                            }}
                        />
                    </div>
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="autostart" className="text-base font-semibold flex items-center gap-2">
                                <Power className="w-4 h-4" />
                                Launch at Startup
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                Automatically start Sklad when you log in to your computer.
                            </p>
                        </div>
                        <Switch
                            id="autostart"
                            checked={settings.launchAtStartup}
                            onCheckedChange={handleAutoStartChange}
                            disabled={autostartLoading}
                        />
                    </div>
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="notifications" className="text-base font-semibold">
                                Tray Copy Notifications
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                Show a system notification when you copy a snippet via the tray menu.
                            </p>
                        </div>
                        <Switch
                            id="notifications"
                            checked={settings.notificationsEnabled}
                            onCheckedChange={(checked) => {
                                onSettingsUpdate({
                                    ...settings,
                                    notificationsEnabled: checked
                                });
                            }}
                        />
                    </div>
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="tray-click-action" className="text-base font-semibold">
                                Tray Left Click Action
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                What happens when you single click the Sklad icon in your system tray.
                            </p>
                        </div>
                        <Select
                            value={settings.trayClickAction || 'copy_last'}
                            onValueChange={(value: 'copy_last' | 'open_app') => {
                                onSettingsUpdate({
                                    ...settings,
                                    trayClickAction: value
                                });
                            }}
                        >
                            <SelectTrigger className="w-48 bg-background/50 border-border/50 h-9">
                                <SelectValue placeholder="Select action..." />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="copy_last">Copy Last Used</SelectItem>
                                <SelectItem value="open_app">Open Sklad</SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="tray-menu-root-position" className="text-base font-semibold">
                                Tray Menu Open/Quit Position
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                Choose whether the Open and Quit buttons appear at the top or bottom of the menu.
                            </p>
                        </div>
                        <Select
                            value={settings.trayMenuRootPosition || 'bottom'}
                            onValueChange={(value: 'top' | 'bottom') => {
                                onSettingsUpdate({
                                    ...settings,
                                    trayMenuRootPosition: value
                                });
                            }}
                        >
                            <SelectTrigger className="w-48 bg-background/50 border-border/50 h-9">
                                <SelectValue placeholder="Select position..." />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="top">Top</SelectItem>
                                <SelectItem value="bottom">Bottom</SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="global-shortcut" className="text-base font-semibold">
                                Global Search Shortcut
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                Open Sklad search from anywhere. Press Backspace to clear.
                            </p>
                        </div>
                        <input
                            id="global-shortcut"
                            className="w-48 px-3 py-1.5 text-sm bg-background/50 border border-border/50 rounded-md font-mono focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary/50 transition-all placeholder:text-muted-foreground/30 text-center"
                            placeholder="Press keys to set..."
                            value={settings.globalSearchShortcut || ""}
                            readOnly
                            onKeyDown={(e) => {
                                e.preventDefault();

                                // Handling clear
                                if (e.key === "Backspace" || e.key === "Delete") {
                                    onSettingsUpdate({ ...settings, globalSearchShortcut: "" });
                                    e.currentTarget.blur();
                                    return;
                                }

                                // Ignore standalone modifier keys
                                if (['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) {
                                    return;
                                }

                                const keys: string[] = [];

                                // Tauri global shortcut format support:
                                // "CommandOrControl", "Alt", "Shift", "Super" AND the upper case letter or exact key representation.

                                // On macOS, metaKey is Command. On Windows/Linux, ctrlKey is Control.
                                // Tauri supports "Cmd" or "Command"
                                if (e.metaKey) keys.push("Cmd");
                                if (e.ctrlKey) keys.push("Ctrl");
                                if (e.altKey) keys.push("Alt");
                                if (e.shiftKey) keys.push("Shift");

                                // Map keys to Tauri's expectations. Usually it's just upper case single letters or digit.
                                let keyName = e.key;
                                if (e.code.startsWith("Key")) {
                                    keyName = e.code.replace("Key", ""); // e.g. "KeyF" -> "F"
                                } else if (e.code.startsWith("Digit")) {
                                    keyName = e.code.replace("Digit", "");
                                } else if (keyName === " ") {
                                    keyName = "Space";
                                } else {
                                    keyName = keyName.charAt(0).toUpperCase() + keyName.slice(1);
                                }

                                keys.push(keyName);

                                const shortcut = keys.join("+");
                                onSettingsUpdate({ ...settings, globalSearchShortcut: shortcut });
                                e.currentTarget.blur();
                            }}
                        />
                    </div>
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="global-search-action" className="text-base font-semibold">
                                Global Search Action
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                What happens when you select a snippet from global search.
                            </p>
                        </div>
                        <Select
                            value={settings.globalSearchAction || 'copy'}
                            onValueChange={(value: 'copy' | 'open') => {
                                onSettingsUpdate({
                                    ...settings,
                                    globalSearchAction: value
                                });
                            }}
                        >
                            <SelectTrigger className="w-48 bg-background/50 border-border/50 h-9">
                                <SelectValue placeholder="Select action..." />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="copy">Copy to Clipboard</SelectItem>
                                <SelectItem value="open">Open in Main Window</SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="global-create-shortcut" className="text-base font-semibold">
                                Global Create Shortcut
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                Quickly create a new snippet from anywhere.
                            </p>
                        </div>
                        <input
                            id="global-create-shortcut"
                            className="w-48 px-3 py-1.5 text-sm bg-background/50 border border-border/50 rounded-md font-mono focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary/50 transition-all placeholder:text-muted-foreground/30 text-center"
                            placeholder="Press keys to set..."
                            value={settings.globalCreateShortcut || ""}
                            readOnly
                            onKeyDown={(e) => {
                                e.preventDefault();

                                // Handling clear
                                if (e.key === "Backspace" || e.key === "Delete") {
                                    onSettingsUpdate({ ...settings, globalCreateShortcut: "" });
                                    e.currentTarget.blur();
                                    return;
                                }

                                // Ignore standalone modifier keys
                                if (['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) {
                                    return;
                                }

                                const keys: string[] = [];

                                if (e.metaKey) keys.push("Cmd");
                                if (e.ctrlKey) keys.push("Ctrl");
                                if (e.altKey) keys.push("Alt");
                                if (e.shiftKey) keys.push("Shift");

                                let keyName = e.key;
                                if (e.code.startsWith("Key")) {
                                    keyName = e.code.replace("Key", "");
                                } else if (e.code.startsWith("Digit")) {
                                    keyName = e.code.replace("Digit", "");
                                } else if (keyName === " ") {
                                    keyName = "Space";
                                } else {
                                    keyName = keyName.charAt(0).toUpperCase() + keyName.slice(1);
                                }

                                keys.push(keyName);

                                const shortcut = keys.join("+");
                                onSettingsUpdate({ ...settings, globalCreateShortcut: shortcut });
                                e.currentTarget.blur();
                            }}
                        />
                    </div>
                </CardContent>
            </Card>

            <Card className="glass border-border/50">
                <CardHeader>
                    <div className="flex items-center gap-2">
                        <Shield className="w-5 h-5 text-primary" />
                        <CardTitle className="text-lg">Security</CardTitle>
                    </div>
                    <CardDescription>
                        Manage access control and encryption settings.
                    </CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">

                    <div className="flex items-center justify-between gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-0.5">
                            <Label htmlFor="autolock" className="text-base font-semibold">
                                Auto-lock Vault
                            </Label>
                            <p className="text-sm text-muted-foreground">
                                Automatically lock the vault after a period of inactivity.
                            </p>
                        </div>
                        <Switch
                            id="autolock"
                            checked={settings.security.lockTimeout > 0}
                            onCheckedChange={(checked) => {
                                onSettingsUpdate({
                                    ...settings,
                                    security: {
                                        ...settings.security,
                                        lockTimeout: checked ? 300000 : 0
                                    }
                                });
                            }}
                        />
                    </div>

                    {settings.security.lockTimeout > 0 && (
                        <div className="flex flex-col gap-3 p-4 rounded-xl bg-muted/30 border border-border/50 animate-fade-in">
                            <Label htmlFor="timeout" className="text-sm font-semibold">
                                Lock Timeout (minutes)
                            </Label>
                            <div className="flex items-center gap-4">
                                <input
                                    type="range"
                                    id="timeout-slider"
                                    min="1"
                                    max="60"
                                    step="1"
                                    className="flex-1 h-2 bg-secondary rounded-lg appearance-none cursor-pointer accent-primary"
                                    value={settings.security.lockTimeout / 60000}
                                    onChange={(e) => {
                                        const minutes = parseInt(e.target.value);
                                        onSettingsUpdate({
                                            ...settings,
                                            security: {
                                                ...settings.security,
                                                lockTimeout: minutes * 60000
                                            }
                                        });
                                    }}
                                />
                                <span className="font-mono text-sm w-16 text-right">
                                    {Math.floor(settings.security.lockTimeout / 60000)} min
                                </span>
                            </div>
                        </div>
                    )}
                    <div className="flex flex-col gap-4 p-4 rounded-xl bg-muted/30 border border-border/50">
                        <div className="space-y-1">
                            <div className="flex items-center gap-2">
                                <Lock className={cn(
                                    "w-4 h-4 transition-colors",
                                    settings.security.masterPasswordEnabled ? "text-yellow-500" : "text-muted-foreground/40"
                                )} />
                                <span className="font-semibold">Master Password Security</span>
                            </div>
                            <p className="text-sm text-muted-foreground leading-relaxed pr-8">
                                {settings.security.masterPasswordEnabled
                                    ? "Vault security is active. Your secrets are encrypted with your master password."
                                    : "Vault security is inactive. Secret snippets cannot be used until a password is set."}
                            </p>
                        </div>

                        <div className="pt-2">
                            {settings.security.masterPasswordEnabled ? (
                                <Button
                                    variant="outline"
                                    onClick={() => {
                                        if (confirmReset) {
                                            onResetTrigger();
                                            setConfirmReset(false);
                                        } else {
                                            setConfirmReset(true);
                                            setTimeout(() => setConfirmReset(false), 3000);
                                        }
                                    }}
                                    className={cn(
                                        "w-full h-11 border-destructive/20 transition-all gap-2 font-medium",
                                        confirmReset ? "bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-lg glow-destructive" : "hover:border-destructive/40 hover:bg-destructive/5 text-destructive"
                                    )}
                                >
                                    <AlertCircle className="w-4 h-4" />
                                    {confirmReset ? "CLICK AGAIN TO CONFIRM RESET" : "Reset Vault (Forgot Password?)"}
                                </Button>
                            ) : (
                                <Button
                                    onClick={onSetupTrigger}
                                    className="w-full h-11 bg-primary hover:bg-primary/90 text-primary-foreground shadow-lg hover:shadow-xl transition-all gap-2 font-semibold"
                                >
                                    <Shield className="w-4 h-4" />
                                    Set Master Password
                                </Button>
                            )}
                        </div>
                        <p className="text-[10px] text-muted-foreground/60 italic px-1">
                            {settings.security.masterPasswordEnabled
                                ? "Note: Resetting the vault will permanently delete ALL secret snippets."
                                : "A master password is required to encrypt sensitive data."}
                        </p>
                    </div>
                </CardContent>
            </Card>
        </div>
    );
}
