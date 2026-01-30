import * as React from "react";
import {
    CommandDialog,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "@/components/ui/command";
import { Node } from "@/types";
import { FileText, Folder, Lock, Search } from "lucide-react";
import { cn } from "@/lib/utils";

interface CommandPaletteProps {
    nodes: Node[];
    onSelect: (node: Node) => void;
}

export function CommandPalette({ nodes, onSelect }: CommandPaletteProps) {
    const [open, setOpen] = React.useState(false);

    React.useEffect(() => {
        const down = (e: KeyboardEvent) => {
            if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                setOpen((open) => !open);
            }
        };
        document.addEventListener("keydown", down);
        return () => document.removeEventListener("keydown", down);
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
    const folders = flatNodes.filter(n => n.type === "folder");

    return (
        <CommandDialog open={open} onOpenChange={setOpen}>
            <div className="flex items-center border-b border-border/50 px-3">
                <Search className="w-4 h-4 text-muted-foreground mr-2" />
                <CommandInput
                    placeholder="Search snippets and folders..."
                    className="border-0 focus:ring-0"
                />
            </div>
            <CommandList className="max-h-[400px]">
                <CommandEmpty className="py-8 text-center">
                    <div className="flex flex-col items-center gap-2 text-muted-foreground">
                        <Search className="w-8 h-8 text-muted-foreground/30" />
                        <span>No results found</span>
                    </div>
                </CommandEmpty>

                {snippets.length > 0 && (
                    <CommandGroup heading="Snippets" className="px-2">
                        {snippets.map((node) => (
                            <CommandItem
                                key={node.id}
                                onSelect={() => {
                                    onSelect(node);
                                    setOpen(false);
                                }}
                                className="px-3 py-2 rounded-lg cursor-pointer"
                            >
                                <FileText className={cn(
                                    "mr-3 h-4 w-4",
                                    node.isSecret ? "text-yellow-500" : "text-muted-foreground"
                                )} />
                                <div className="flex-1 flex flex-col">
                                    <span className="font-medium">{node.label}</span>
                                    {node.path.length > 1 && (
                                        <span className="text-xs text-muted-foreground/60">
                                            {node.path.slice(0, -1).join(" / ")}
                                        </span>
                                    )}
                                </div>
                                {node.isSecret && <Lock className="w-3 h-3 text-yellow-500/60" />}
                            </CommandItem>
                        ))}
                    </CommandGroup>
                )}

                {folders.length > 0 && (
                    <CommandGroup heading="Folders" className="px-2">
                        {folders.map((node) => (
                            <CommandItem
                                key={node.id}
                                onSelect={() => {
                                    onSelect(node);
                                    setOpen(false);
                                }}
                                className="px-3 py-2 rounded-lg cursor-pointer"
                            >
                                <Folder className="mr-3 h-4 w-4 text-primary/70" />
                                <div className="flex-1 flex flex-col">
                                    <span className="font-medium">{node.label}</span>
                                    {node.path.length > 1 && (
                                        <span className="text-xs text-muted-foreground/60">
                                            {node.path.slice(0, -1).join(" / ")}
                                        </span>
                                    )}
                                </div>
                            </CommandItem>
                        ))}
                    </CommandGroup>
                )}
            </CommandList>
        </CommandDialog>
    );
}
