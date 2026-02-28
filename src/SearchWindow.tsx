import * as React from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "@/components/ui/command";
import { Node } from "@/types";
import { FileText, Lock, Search } from "lucide-react";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";

export function SearchWindow() {
    const [nodes, setNodes] = React.useState<Node[]>([]);
    const [searchValue, setSearchValue] = React.useState("");
    const inputRef = React.useRef<HTMLInputElement>(null);

    React.useEffect(() => {
        const loadNodes = async () => {
            const data = await api.getData();
            setNodes(data || []);
        };
        loadNodes();
    }, []);

    React.useEffect(() => {
        const window = getCurrentWebviewWindow();

        const unlistenFocus = window.onFocusChanged(({ payload: focused }) => {
            if (!focused) {
                window.hide();
            } else {
                setSearchValue("");
                // Small delay to ensure the window is fully visible before focusing
                setTimeout(() => {
                    inputRef.current?.focus();
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

    // Flatten nodes for search
    const flattenNodes = (list: Node[], path: string[] = []): (Node & { path: string[] })[] => {
        let result: (Node & { path: string[] })[] = [];
        for (const node of list) {
            const nodePath = [...path, node.label];
            result.push({ ...node, path: nodePath });
            if (node.children) {
                result = result.concat(flattenNodes(node.children, nodePath));
            }
        }
        return result;
    };

    const flatNodes = React.useMemo(() => flattenNodes(nodes), [nodes]);
    const snippets = flatNodes.filter(n => n.type === "snippet");

    const handleSelect = async (node: Node) => {
        try {
            await api.copySnippet(node.id);
            await getCurrentWebviewWindow().hide();
        } catch (e: any) {
            console.error("Failed to copy", e);
            if (e === "Vault is Locked") {
                const mainWindow = await (await import("@tauri-apps/api/webviewWindow")).WebviewWindow.getByLabel("main");
                if (mainWindow) {
                    await mainWindow.show();
                    await mainWindow.setFocus();
                    await mainWindow.emit("request-unlock", node.id);
                }
                await getCurrentWebviewWindow().hide();
            }
        }
    };

    return (
        <div className="flex flex-col h-screen w-screen bg-popover text-popover-foreground rounded-xl overflow-hidden shadow-2xl border border-border">
            <Command className="flex-1 bg-transparent">
                <div className="flex items-center px-4 border-b border-border/50">
                    <Search className="w-5 h-5 text-muted-foreground mr-3" />
                    <CommandInput
                        ref={inputRef}
                        value={searchValue}
                        onValueChange={setSearchValue}
                        placeholder="Search snippets..."
                        className="h-14 border-0 focus:ring-0 text-lg flex-1 bg-transparent outline-none"
                        autoFocus
                    />
                </div>
                <CommandList className="flex-1 overflow-y-auto max-h-[340px] p-2">
                    <CommandEmpty className="py-12 text-center text-muted-foreground">
                        <div className="flex flex-col items-center justify-center gap-3">
                            <Search className="w-10 h-10 text-muted-foreground/30" />
                            <span className="text-lg">No snippets found</span>
                        </div>
                    </CommandEmpty>

                    {snippets.length > 0 && (
                        <CommandGroup heading="Snippets" className="px-1">
                            {snippets.map((node) => (
                                <CommandItem
                                    key={node.id}
                                    onSelect={() => handleSelect(node)}
                                    value={node.label + " " + node.path.join(" ")}
                                    className="px-4 py-3 rounded-lg cursor-pointer mb-1 data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground"
                                >
                                    <FileText className={cn(
                                        "mr-4 h-5 w-5",
                                        node.isSecret ? "text-yellow-500" : "text-muted-foreground"
                                    )} />
                                    <div className="flex-1 flex flex-col">
                                        <span className="font-medium text-base">{node.label}</span>
                                        {node.path.length > 1 && (
                                            <span className="text-sm text-muted-foreground/70 mt-0.5">
                                                {node.path.slice(0, -1).join(" / ")}
                                            </span>
                                        )}
                                    </div>
                                    {node.isSecret && <Lock className="w-4 h-4 text-yellow-500/60 ml-2" />}
                                </CommandItem>
                            ))}
                        </CommandGroup>
                    )}
                </CommandList>
            </Command>
        </div>
    );
}
