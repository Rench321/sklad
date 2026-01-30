import { useState, useEffect } from "react";
import { Node } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Save, Copy, Lock, Unlock, FileText, Check } from "lucide-react";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

interface SnippetEditorProps {
    node: Node;
    onSave: (updatedNode: Node) => void;
    onUnlockTrigger?: () => void;
    masterPasswordEnabled?: boolean;
    isUnlocked?: boolean;
}

export function SnippetEditor({
    node,
    onSave,
    onUnlockTrigger,
    masterPasswordEnabled = true,
    isUnlocked = false
}: SnippetEditorProps) {
    const [label, setLabel] = useState(node.label);
    const [value, setValue] = useState(node.value || "");
    const [isSecret, setIsSecret] = useState(node.isSecret || false);
    const [isSaving, setIsSaving] = useState(false);
    const [copied, setCopied] = useState(false);

    useEffect(() => {
        setLabel(node.label);
        setValue(node.value || "");
        setIsSecret(node.isSecret || false);
    }, [node]);

    const handleSave = async () => {
        if (isSecret && (!masterPasswordEnabled || !isUnlocked)) return;

        setIsSaving(true);
        const updatedNode: Node = {
            ...node,
            label,
            isSecret,
            value: value
        };

        onSave(updatedNode);
        setTimeout(() => setIsSaving(false), 500);
    };

    const handleCopy = async () => {
        if (isSecret && (!masterPasswordEnabled || !isUnlocked)) return;

        await api.copySnippet(node.id);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    if (node.type === 'folder') {
        return (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-4">
                <div className="p-6 rounded-2xl bg-muted/30 border border-border/50">
                    <FileText className="w-12 h-12 text-muted-foreground/40" />
                </div>
                <div className="text-center">
                    <p className="font-medium text-foreground/80">Folder selected</p>
                    <p className="text-sm text-muted-foreground/60 mt-1">Select a snippet to edit its content</p>
                </div>
            </div>
        );
    }

    return (
        <div className="p-6 h-full flex flex-col gap-5 animate-fade-in relative overflow-hidden">
            {isSecret && (!masterPasswordEnabled || !isUnlocked) && (
                <div className="absolute inset-0 z-10 bg-background/98 backdrop-blur-xl flex items-center justify-center p-6 text-center animate-in fade-in duration-500">
                    <div className="max-w-xs space-y-6">
                        <div className="relative mx-auto w-20 h-20">
                            <div className="absolute inset-0 bg-destructive/10 rounded-2xl rotate-6 animate-pulse" />
                            <div className="absolute inset-0 bg-destructive/5 rounded-2xl -rotate-6" />
                            <div className="relative z-10 w-full h-full rounded-2xl bg-card border border-destructive/20 flex items-center justify-center shadow-xl">
                                <Lock className="w-10 h-10 text-destructive animate-bounce-slow" />
                            </div>
                        </div>
                        <div className="space-y-4">
                            <div className="space-y-2">
                                <h3 className="font-bold text-xl text-foreground tracking-tight">Access Restricted</h3>
                                <p className="text-sm text-muted-foreground leading-relaxed px-4">
                                    {!masterPasswordEnabled
                                        ? "Decryption is disabled because Master Password Security is turned off in Settings."
                                        : "This snippet is encrypted. Please unlock the vault with your master password to view its content."}
                                </p>
                            </div>
                            {masterPasswordEnabled && (
                                <Button
                                    onClick={onUnlockTrigger}
                                    variant="outline"
                                    size="sm"
                                    className="gap-2 border-primary/20 hover:border-primary/40 hover:bg-primary/5 h-10 px-6 font-medium"
                                >
                                    <Unlock className="w-4 h-4 text-primary" />
                                    <span>Unlock Now</span>
                                </Button>
                            )}
                        </div>
                    </div>
                </div>
            )}

            {/* Header */}
            <div className="flex items-center justify-between gap-4">
                <div className="flex-1">
                    <Input
                        value={label}
                        onChange={(e) => setLabel(e.target.value)}
                        className="text-lg font-semibold border-none shadow-none focus-visible:ring-0 px-0 bg-transparent h-auto py-1 placeholder:text-muted-foreground/40"
                        placeholder="Snippet name..."
                    />
                </div>
                <div className="flex items-center gap-2">
                    {/* ... Copy button ... */}
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={handleCopy}
                        disabled={isSecret && (!masterPasswordEnabled || !isUnlocked)}
                        title="Copy to Clipboard"
                        className={cn(
                            "transition-all",
                            copied && "bg-green-500/10 border-green-500/30 text-green-500"
                        )}
                    >
                        {copied ? (
                            <>
                                <Check className="w-4 h-4 mr-2" />
                                Copied!
                            </>
                        ) : (
                            <>
                                <Copy className="w-4 h-4 mr-2" />
                                Copy
                            </>
                        )}
                    </Button>
                    {(() => {
                        const isDirty =
                            label !== node.label ||
                            value !== (node.value || "") ||
                            isSecret !== (node.isSecret || false);

                        return (
                            <Button
                                onClick={handleSave}
                                disabled={isSaving || !isDirty || (isSecret && (!masterPasswordEnabled || !isUnlocked))}
                                className={cn(
                                    "shadow-lg hover:shadow-xl transition-all",
                                    !isDirty ? "opacity-50" : "bg-primary hover:bg-primary/90"
                                )}
                            >
                                <Save className="w-4 h-4 mr-2" />
                                {isSaving ? "Saving..." : "Save"}
                            </Button>
                        );
                    })()}
                </div>
            </div>

            {/* Secret toggle */}
            <div className={cn(
                "flex items-center space-x-3 p-3 rounded-lg bg-muted/30 border border-border/50 transition-all",
                !masterPasswordEnabled && "opacity-60 cursor-not-allowed"
            )}>
                <Switch
                    id="secret-mode"
                    checked={isSecret}
                    onCheckedChange={(checked) => {
                        if (!isUnlocked && masterPasswordEnabled) {
                            onUnlockTrigger?.();
                        } else {
                            setIsSecret(checked);
                        }
                    }}
                    disabled={!masterPasswordEnabled}
                    className="data-[state=checked]:bg-yellow-500"
                />
                <div className="flex items-center gap-2">
                    <Lock className={cn(
                        "w-4 h-4 transition-colors",
                        isSecret ? "text-yellow-500" : "text-muted-foreground/50"
                    )} />
                    <Label htmlFor="secret-mode" className={cn(
                        "cursor-pointer",
                        !masterPasswordEnabled && "cursor-not-allowed"
                    )}>
                        <span className={cn(
                            "font-medium transition-colors",
                            isSecret ? "text-yellow-500" : "text-muted-foreground"
                        )}>
                            Secret snippet
                        </span>
                        <span className="text-xs text-muted-foreground/60 ml-2">
                            {masterPasswordEnabled ? "(Encrypted on save)" : "(Enable Master Password in Settings to use)"}
                        </span>
                    </Label>
                </div>
            </div>

            {/* Editor */}
            <div className="flex-1 flex flex-col min-h-0">
                <Textarea
                    className="flex-1 font-mono text-sm resize-none bg-muted/20 border-border/50 focus:border-primary/50 focus:ring-primary/20 rounded-lg p-4 placeholder:text-muted-foreground/40"
                    placeholder="Type your snippet content here..."
                    value={isSecret && !isUnlocked ? "" : value}
                    onChange={(e) => setValue(e.target.value)}
                    disabled={isSecret && (!masterPasswordEnabled || !isUnlocked)}
                />
            </div>
        </div>
    );
}
