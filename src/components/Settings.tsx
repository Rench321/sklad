import { useState, useEffect } from "react";
import { AppSettings } from "@/types";
import { api } from "@/lib/api";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Shield, Lock, Settings as SettingsIcon, Database, ExternalLink, FileJson, AlertCircle, Bell } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

interface SettingsProps {
    settings: AppSettings;
    onResetTrigger: () => void;
    onSetupTrigger: () => void;
    onSettingsUpdate: (settings: AppSettings) => void;
}

export function Settings({ settings, onResetTrigger, onSetupTrigger, onSettingsUpdate }: SettingsProps) {
    const [snippetsPath, setSnippetsPath] = useState<string>("");
    const [confirmReset, setConfirmReset] = useState(false);

    useEffect(() => {
        const fetchPath = async () => {
            try {
                const path = await api.getSnippetsPath();
                setSnippetsPath(path);
            } catch (error) {
                console.error("Failed to fetch snippets path", error);
            }
        };
        fetchPath();
    }, []);


    return (
        <div className="p-8 flex flex-col gap-8 animate-fade-in max-w-2xl mx-auto">
            <div className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-primary/10 border border-primary/20">
                    <SettingsIcon className="w-6 h-6 text-primary" />
                </div>
                <div>
                    <h1 className="text-2xl font-bold tracking-tight">Sklad Settings</h1>
                    <p className="text-muted-foreground">Configure your industrial snippet storage.</p>
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
                            All your snippets, folders, and settings are stored in this JSON file.
                        </p>
                    </div>
                </CardContent>
            </Card>

            <Card className="glass border-border/50">
                <CardHeader>
                    <div className="flex items-center gap-2">
                        <Bell className="w-5 h-5 text-primary" />
                        <CardTitle className="text-lg">Desktop Notifications</CardTitle>
                    </div>
                    <CardDescription>
                        Control how Sklad interacts with your system.
                    </CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                    <div className="flex items-center justify-between p-4 rounded-xl bg-muted/30 border border-border/50">
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
