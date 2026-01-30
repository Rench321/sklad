import { Node } from "@/types";
import { Folder, FileText, ChevronRight, Lock, Plus, Trash2, FolderPlus, FilePlus, Container, Pencil, Settings as SettingsIcon } from "lucide-react";
import { useState, useRef, useEffect } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

interface SidebarProps {
    nodes: Node[];
    onSelectNode: (node: Node) => void;
    selectedNodeId?: string;
    onAddNode: (parentId: string | null, type: "folder" | "snippet") => void;
    onDeleteNode: (nodeId: string) => void;
    onRenameNode: (nodeId: string, newLabel: string) => void;
    onMoveNode?: (draggedId: string, targetId: string | null, beforeId?: string | null) => void;
}

interface TreeNodeProps {
    node: Node;
    level: number;
    onSelect: (n: Node) => void;
    selectedId?: string;
    onAddNode: (parentId: string | null, type: "folder" | "snippet") => void;
    onDeleteNode: (nodeId: string) => void;
    onRenameNode: (nodeId: string, newLabel: string) => void;
    onMoveNode?: (draggedId: string, targetId: string | null, beforeId?: string | null) => void;
    parentId?: string | null;
    siblings?: Node[];
}

const TreeNode = ({ node, level, onSelect, selectedId, onAddNode, onDeleteNode, onRenameNode, onMoveNode, parentId, siblings }: TreeNodeProps) => {
    const [isOpen, setIsOpen] = useState(false);
    const [isHovered, setIsHovered] = useState(false);
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState(node.label);
    const [isDragOver, setIsDragOver] = useState(false);
    const [dropPosition, setDropPosition] = useState<'before' | 'inside' | 'after' | null>(null);
    const [isDeleting, setIsDeleting] = useState(false);
    const deleteTimeoutRef = useRef<NodeJS.Timeout | null>(null);
    const nodeRef = useRef<HTMLDivElement>(null);

    const inputRef = useRef<HTMLInputElement>(null);
    const hasChildren = node.children && node.children.length > 0;
    const isSelected = node.id === selectedId;

    useEffect(() => {
        if (isEditing && inputRef.current) {
            inputRef.current.focus();
            inputRef.current.select();
        }
    }, [isEditing]);

    const handleToggle = (e: React.MouseEvent) => {
        e.stopPropagation();
        setIsOpen(!isOpen);
    };

    const handleClick = () => {
        if (isEditing) return;
        onSelect(node);
        if (node.type === "folder") setIsOpen(!isOpen);
    };

    const handleDoubleClick = (e: React.MouseEvent) => {
        e.stopPropagation();
        setEditValue(node.label);
        setIsEditing(true);
    };

    const handleRenameSubmit = () => {
        if (editValue.trim() && editValue !== node.label) {
            onRenameNode(node.id, editValue.trim());
        }
        setIsEditing(false);
    };

    const handleRenameKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter") {
            handleRenameSubmit();
        } else if (e.key === "Escape") {
            setEditValue(node.label);
            setIsEditing(false);
        }
    };

    const startRename = (e: React.MouseEvent) => {
        e.stopPropagation();
        setEditValue(node.label);
        setIsEditing(true);
    };

    // Drag and Drop handlers
    const handleDragStart = (e: React.DragEvent) => {
        e.stopPropagation();
        e.dataTransfer.setData("application/json", JSON.stringify({ id: node.id, parentId: parentId }));
        e.dataTransfer.effectAllowed = "move";
    };

    const handleDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        e.dataTransfer.dropEffect = "move";

        if (!nodeRef.current) return;

        const rect = nodeRef.current.getBoundingClientRect();
        const y = e.clientY - rect.top;
        const height = rect.height;

        // For folders: top 25% = before, middle 50% = inside, bottom 25% = after
        // For snippets: top 50% = before, bottom 50% = after
        if (node.type === "folder") {
            if (y < height * 0.25) {
                setDropPosition('before');
                setIsDragOver(false);
            } else if (y > height * 0.75) {
                setDropPosition('after');
                setIsDragOver(false);
            } else {
                setDropPosition('inside');
                setIsDragOver(true);
            }
        } else {
            if (y < height * 0.5) {
                setDropPosition('before');
            } else {
                setDropPosition('after');
            }
            setIsDragOver(false);
        }
    };

    const handleDragLeave = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setIsDragOver(false);
        setDropPosition(null);
    };

    const handleDrop = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();

        const currentDropPosition = dropPosition;
        setIsDragOver(false);
        setDropPosition(null);

        if (!onMoveNode) return;

        try {
            const data = JSON.parse(e.dataTransfer.getData("application/json"));
            if (!data.id || data.id === node.id) return;

            if (currentDropPosition === 'inside' && node.type === "folder") {
                // Drop inside folder
                onMoveNode(data.id, node.id);
                setIsOpen(true);
            } else if (currentDropPosition === 'before') {
                // Drop before this node
                onMoveNode(data.id, parentId ?? null, node.id);
            } else if (currentDropPosition === 'after') {
                // Drop after this node - find the next sibling
                if (siblings) {
                    const currentIndex = siblings.findIndex(s => s.id === node.id);
                    const nextSibling = siblings[currentIndex + 1];
                    onMoveNode(data.id, parentId ?? null, nextSibling?.id ?? null);
                } else {
                    onMoveNode(data.id, parentId ?? null, null);
                }
            }
        } catch (err) {
            console.error("Failed to parse drag data", err);
        }
    };

    return (
        <div className="animate-fade-in relative" style={{ animationDelay: `${level * 30}ms` }}>
            {/* Drop indicator for "before" position */}
            {dropPosition === 'before' && (
                <div
                    className="absolute left-1 right-1 h-0.5 bg-primary rounded-full z-10"
                    style={{ top: 0, marginLeft: `${level * 12}px` }}
                />
            )}
            <div
                ref={nodeRef}
                className={cn(
                    "flex items-center py-1.5 px-2 cursor-pointer rounded-md text-sm transition-all duration-200 group mx-1",
                    isSelected
                        ? "bg-primary/15 text-primary border border-primary/30"
                        : "hover:bg-accent/50 border border-transparent hover:border-border/50",
                    isDragOver && "bg-primary/20 border-primary/50 text-primary ring-1 ring-primary/30"
                )}
                style={{ paddingLeft: `${level * 12 + 8}px` }}
                onClick={handleClick}
                onDoubleClick={handleDoubleClick}
                onMouseEnter={() => setIsHovered(true)}
                onMouseLeave={() => {
                    setIsHovered(false);
                    setIsDeleting(false);
                    if (deleteTimeoutRef.current) clearTimeout(deleteTimeoutRef.current);
                }}
                draggable={!isEditing}
                onDragStart={handleDragStart}
                onDragOver={handleDragOver}
                onDragLeave={handleDragLeave}
                onDrop={handleDrop}
            >
                <span className="mr-1 w-4 h-4 flex items-center justify-center transition-transform" onClick={hasChildren ? handleToggle : undefined}>
                    {(hasChildren || node.type === "folder") && (
                        <span className={cn("transition-transform duration-200", isOpen && "rotate-90")}>
                            <ChevronRight className="w-3 h-3 text-muted-foreground" />
                        </span>
                    )}
                </span>

                {node.type === "folder" ? (
                    <Folder className={cn(
                        "w-4 h-4 mr-2 transition-colors flex-shrink-0",
                        isSelected || isDragOver ? "text-primary" : "text-primary/60"
                    )} />
                ) : (
                    <FileText className={cn(
                        "w-4 h-4 mr-2 transition-colors flex-shrink-0",
                        isSelected ? "text-foreground" : "text-muted-foreground"
                    )} />
                )}

                {isEditing ? (
                    <Input
                        ref={inputRef}
                        value={editValue}
                        onChange={(e) => setEditValue(e.target.value)}
                        onBlur={handleRenameSubmit}
                        onKeyDown={handleRenameKeyDown}
                        onClick={(e) => e.stopPropagation()}
                        className="h-5 py-0 px-1 text-sm bg-background border-primary/50 focus-visible:ring-1"
                    />
                ) : (
                    <span className={cn(
                        "truncate flex-1 transition-colors",
                        isSelected ? "font-medium" : ""
                    )}>{node.label}</span>
                )}

                {node.isSecret && !isEditing && (
                    <Lock className="w-3 h-3 ml-1 text-yellow-500/80 flex-shrink-0" />
                )}

                {/* Action buttons on hover */}
                {!isEditing && (
                    <div className={cn(
                        "flex items-center gap-0.5 ml-1 transition-opacity duration-200 flex-shrink-0",
                        isHovered ? "opacity-100" : "opacity-0"
                    )} onClick={(e) => e.stopPropagation()}>
                        <Button
                            variant="ghost"
                            size="icon"
                            className="h-5 w-5 hover:bg-primary/20 hover:text-primary"
                            onClick={startRename}
                            title="Rename"
                        >
                            <Pencil className="w-3 h-3" />
                        </Button>
                        {node.type === "folder" && (
                            <>
                                <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-5 w-5 hover:bg-primary/20 hover:text-primary"
                                    onClick={() => onAddNode(node.id, "folder")}
                                    title="Add Folder"
                                >
                                    <FolderPlus className="w-3 h-3" />
                                </Button>
                                <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-5 w-5 hover:bg-primary/20 hover:text-primary"
                                    onClick={() => onAddNode(node.id, "snippet")}
                                    title="Add Snippet"
                                >
                                    <FilePlus className="w-3 h-3" />
                                </Button>
                            </>
                        )}
                        <Button
                            variant="ghost"
                            size="icon"
                            className={cn(
                                "h-5 w-5 transition-all duration-200",
                                isDeleting
                                    ? "text-destructive bg-destructive/20 w-12 hover:bg-destructive/30"
                                    : "text-muted-foreground hover:text-destructive hover:bg-destructive/10"
                            )}
                            onClick={(e) => {
                                e.stopPropagation();
                                if (isDeleting) {
                                    onDeleteNode(node.id);
                                    setIsDeleting(false);
                                    if (deleteTimeoutRef.current) clearTimeout(deleteTimeoutRef.current);
                                } else {
                                    setIsDeleting(true);
                                    deleteTimeoutRef.current = setTimeout(() => setIsDeleting(false), 3000);
                                }
                            }}
                            title={isDeleting ? "Click again to confirm delete" : "Delete"}
                        >
                            {isDeleting ? <span className="text-[10px] font-bold px-1">DEL?</span> : <Trash2 className="w-3 h-3" />}
                        </Button>
                    </div>
                )}
            </div>

            {/* Drop indicator for "after" position */}
            {dropPosition === 'after' && (
                <div
                    className="absolute left-1 right-1 h-0.5 bg-primary rounded-full z-10"
                    style={{ bottom: 0, marginLeft: `${level * 12}px` }}
                />
            )}

            {/* Children with animation */}
            <div className={cn(
                "overflow-hidden transition-all duration-200",
                isOpen ? "opacity-100" : "opacity-0 h-0"
            )}>
                {isOpen && hasChildren && node.children!.map((child, _index, arr) => (
                    <TreeNode
                        key={child.id}
                        node={child}
                        level={level + 1}
                        onSelect={onSelect}
                        selectedId={selectedId}
                        onAddNode={onAddNode}
                        onDeleteNode={onDeleteNode}
                        onRenameNode={onRenameNode}
                        onMoveNode={onMoveNode}
                        parentId={node.id}
                        siblings={arr}
                    />
                ))}
            </div>
        </div>
    );
};

export function Sidebar({ nodes, onSelectNode, selectedNodeId, onAddNode, onDeleteNode, onRenameNode, onMoveNode }: SidebarProps) {
    const handleRootDrop = (e: React.DragEvent) => {
        e.preventDefault();
        if (!onMoveNode) return;

        try {
            const data = JSON.parse(e.dataTransfer.getData("application/json"));
            if (data.id) {
                onMoveNode(data.id, null);
            }
        } catch (err) {
            console.error("Failed to parse drag data", err);
        }
    };

    const handleRootDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
    };

    return (
        <div
            className="w-64 border-r border-border/50 bg-sidebar h-full overflow-hidden flex flex-col"
            onDrop={handleRootDrop}
            onDragOver={handleRootDragOver}
        >
            {/* Header */}
            <div className="px-4 py-3 border-b border-border/50">
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                        <Container className="w-4 h-4 text-primary" />
                        <span className="text-xs font-bold text-muted-foreground uppercase tracking-widest">Sklad</span>
                    </div>
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button
                                variant="ghost"
                                size="icon"
                                className="h-6 w-6 hover:bg-primary/20 hover:text-primary transition-colors"
                            >
                                <Plus className="w-4 h-4" />
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="glass">
                            <DropdownMenuItem
                                onClick={() => onAddNode(null, "folder")}
                                className="cursor-pointer hover:bg-primary/10 focus:bg-primary/10"
                            >
                                <FolderPlus className="w-4 h-4 mr-2 text-primary" />
                                <span>New Folder</span>
                            </DropdownMenuItem>
                            <DropdownMenuItem
                                onClick={() => onAddNode(null, "snippet")}
                                className="cursor-pointer hover:bg-primary/10 focus:bg-primary/10"
                            >
                                <FilePlus className="w-4 h-4 mr-2 text-muted-foreground" />
                                <span>New Snippet</span>
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                </div>
            </div>

            {/* Tree */}
            <div className="flex-1 overflow-y-auto py-2">
                {nodes.length === 0 ? (
                    <div className="flex flex-col items-center justify-center h-full text-center px-4">
                        <Container className="w-10 h-10 text-muted-foreground/30 mb-3" />
                        <p className="text-sm text-muted-foreground/70">Warehouse is empty</p>
                        <p className="text-xs text-muted-foreground/50 mt-1">Click + to add items</p>
                        <p className="text-xs text-muted-foreground/40 mt-3 pt-3 border-t border-border/30 w-full">
                            Tip: Drag & drop to reorder
                        </p>
                    </div>
                ) : (
                    nodes.map((node, _index, arr) => (
                        <TreeNode
                            key={node.id}
                            node={node}
                            level={0}
                            onSelect={onSelectNode}
                            selectedId={selectedNodeId}
                            onAddNode={onAddNode}
                            onDeleteNode={onDeleteNode}
                            onRenameNode={onRenameNode}
                            onMoveNode={onMoveNode}
                            parentId={null}
                            siblings={arr}
                        />
                    ))
                )}
            </div>

            {/* Bottom Actions */}
            <div className="p-3 border-t border-border/50 bg-card/30">
                <Button
                    variant="ghost"
                    size="sm"
                    className={cn(
                        "w-full justify-start gap-3 hover:bg-primary/10 hover:text-primary transition-all border border-transparent",
                        selectedNodeId === "settings"
                            ? "bg-primary/15 text-primary border-primary/30"
                            : "text-muted-foreground hover:border-border/50"
                    )}
                    onClick={() => onSelectNode({ id: 'settings', type: 'folder', label: 'Settings', parentId: null, createdAt: 0 })}
                >
                    <SettingsIcon className="w-4 h-4" />
                    <span className="text-xs font-semibold uppercase tracking-wider">Settings</span>
                </Button>
            </div>
        </div>
    );
}
