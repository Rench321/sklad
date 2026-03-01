import * as React from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Plus, X } from "lucide-react";
import { api } from "@/lib/api";
import { Node } from "@/types";

export function CreateWindow() {
    const [name, setName] = React.useState("");
    const [value, setValue] = React.useState("");
    const [isSaving, setIsSaving] = React.useState(false);
    const nameRef = React.useRef<HTMLInputElement>(null);

    React.useEffect(() => {
        const window = getCurrentWebviewWindow();

        const unlistenFocus = window.onFocusChanged(({ payload: focused }) => {
            if (!focused) {
                window.hide();
            } else {
                setName("");
                setValue("");
                setTimeout(() => {
                    nameRef.current?.focus();
                }, 50);
            }
        });

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                window.hide();
            }
        };
        document.addEventListener('keydown', handleKeyDown);

        return () => {
            unlistenFocus.then(f => f());
            document.removeEventListener('keydown', handleKeyDown);
        }
    }, []);

    const handleSave = async () => {
        if (!name.trim()) return;

        try {
            console.log("Saving snippet:", name);
            setIsSaving(true);
            const nodes = await api.getData() || [];

            const newSnippet: Node = {
                id: crypto.randomUUID(),
                type: "snippet",
                label: name.trim(),
                parentId: null,
                createdAt: Date.now(),
                value: value,
                isSecret: false,
            };

            nodes.push(newSnippet);
            console.log("Calling api.saveData");
            await api.saveData(nodes);
            console.log("api.saveData success");

            const { emit } = await import("@tauri-apps/api/event");
            await emit("data-updated");

            console.log("Attempting to hide window");
            try {
                await getCurrentWebviewWindow().hide();
                console.log("Window hidden successfully");
            } catch (hideErr) {
                console.error("Failed to hide window explicitly:", hideErr);
            }

            setName("");
            setValue("");
        } catch (e) {
            console.error("Failed to save snippet", e);
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <div className="flex flex-col h-screen w-screen bg-popover text-popover-foreground rounded-xl overflow-hidden shadow-2xl border border-border">
            <div className="flex items-center px-4 border-b border-border/50 bg-muted/10">
                <Plus className="w-5 h-5 text-muted-foreground mr-3" />
                <input
                    ref={nameRef}
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                            if (e.ctrlKey) {
                                handleSave();
                            } else {
                                e.preventDefault();
                                document.querySelector('textarea')?.focus();
                            }
                        }
                    }}
                    placeholder="Snippet Name..."
                    className="h-14 border-0 focus:ring-0 text-lg flex-1 bg-transparent outline-none placeholder:text-muted-foreground/60 p-0"
                />
                <button
                    onClick={async () => {
                        try {
                            await getCurrentWebviewWindow().hide();
                        } catch (e) {
                            console.error("Failed to hide via X button", e);
                        }
                    }}
                    className="p-1.5 hover:bg-muted rounded-md transition-colors text-muted-foreground hover:text-foreground ml-2"
                >
                    <X className="w-5 h-5" />
                </button>
            </div>

            <textarea
                value={value}
                onChange={(e) => setValue(e.target.value)}
                onKeyDown={(e) => {
                    if (e.key === 'Enter' && e.ctrlKey) {
                        e.preventDefault();
                        handleSave();
                    }
                }}
                placeholder="Content..."
                className="flex-1 w-full bg-transparent border-0 p-4 text-sm font-mono focus:outline-none focus:ring-0 resize-none leading-relaxed placeholder:text-muted-foreground/40"
            />

            <div className="flex items-center justify-between px-4 py-3 border-t border-border/50 bg-muted/20">
                <div className="text-xs text-muted-foreground flex items-center gap-1.5 select-none">
                    <kbd className="px-1.5 py-0.5 bg-background border border-border rounded font-mono text-[10px] font-medium text-foreground/80 shadow-sm">Ctrl</kbd>
                    <span>+</span>
                    <kbd className="px-1.5 py-0.5 bg-background border border-border rounded font-mono text-[10px] font-medium text-foreground/80 shadow-sm">Enter</kbd>
                    <span className="ml-1">to save</span>
                </div>
                <button
                    onClick={handleSave}
                    disabled={!name.trim() || isSaving}
                    className="bg-primary hover:bg-primary/90 text-primary-foreground px-4 py-1.5 rounded-md text-sm font-medium transition-all focus:ring-2 focus:ring-ring focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed shadow-sm active:scale-95"
                >
                    {isSaving ? "Saving..." : "Save Snippet"}
                </button>
            </div>
        </div>
    );
}
