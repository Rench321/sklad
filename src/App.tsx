import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Sidebar } from "@/components/Sidebar";
import { SnippetEditor } from "@/components/SnippetEditor";
import { VaultLock } from "@/components/VaultLock";
import { CommandPalette } from "@/components/CommandPalette";
import { ThemeToggle } from "@/components/ThemeToggle";
import { Settings } from "@/components/Settings";
import { api } from "@/lib/api";
import { Node, AppSettings } from "@/types";
import { Container, Search, Lock, Unlock } from "lucide-react";

function App() {
  const [showLockModal, setShowLockModal] = useState(false);
  const [showSetupModal, setShowSetupModal] = useState(false);
  const [isUnlocked, setIsUnlocked] = useState(false);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [pendingCopyAction, setPendingCopyAction] = useState<{ id: string, autoHide: boolean } | null>(null);

  useEffect(() => {
    initializeApp();

    const setupListener = async () => {
      const unlisten = await listen<string>("request-unlock", (event) => {
        console.log("Unlock requested for snippet:", event.payload);
        setPendingCopyAction({ id: event.payload, autoHide: true });
        setShowLockModal(true);
      });
      return unlisten;
    };

    const unlistenPromise = setupListener();
    return () => {
      unlistenPromise.then(unlisten => unlisten && unlisten());
    };
  }, []);

  const refreshSelectedNode = (freshNodes: Node[]) => {
    if (selectedNode) {
      if (selectedNode.id === 'settings') return;

      const findInTree = (list: Node[]): Node | undefined => {
        for (const n of list) {
          if (n.id === selectedNode.id) return n;
          if (n.children) {
            const found = findInTree(n.children);
            if (found) return found;
          }
        }
        return undefined;
      };

      const freshSelected = findInTree(freshNodes);
      if (freshSelected) {
        setSelectedNode(freshSelected);
      }
    }
  };

  const initializeApp = async () => {
    try {
      const [data, appSettings, unlocked] = await Promise.all([
        api.getData(),
        api.getSettings(),
        api.isVaultUnlocked()
      ]);

      const freshNodes = data || [];
      setNodes(freshNodes);
      setSettings(appSettings);
      setIsUnlocked(unlocked);
      refreshSelectedNode(freshNodes);

      setLoaded(true);
    } catch (e) {
      console.error(e);
      setLoaded(true);
    }
  };

  useEffect(() => {
    if (loaded && settings) {
      loadNodes();
    }
  }, [settings?.security.masterPasswordEnabled]);

  const loadNodes = async () => {
    try {
      const [data, unlocked] = await Promise.all([
        api.getData(),
        api.isVaultUnlocked()
      ]);

      const freshNodes = data || [];
      setNodes(freshNodes);
      setIsUnlocked(unlocked);
      refreshSelectedNode(freshNodes);
    } catch (e) {
      console.error(e);
    }
  };

  // Auto-lock timer: 5 minutes after unlocking
  useEffect(() => {
    let timeout: NodeJS.Timeout;
    if (isUnlocked) {
      timeout = setTimeout(async () => {
        try {
          await api.lockVault();
          setIsUnlocked(false);

          // Immediate memory purge
          const purgeSecrets = (list: Node[]): Node[] => {
            return list.map(n => ({
              ...n,
              value: n.isSecret ? "" : n.value,
              children: n.children ? purgeSecrets(n.children) : undefined
            }));
          };

          setNodes(prev => purgeSecrets(prev));
          setSelectedNode(prev => {
            if (prev?.isSecret) return { ...prev, value: "" };
            return prev;
          });

          loadNodes(); // Reload to confirm state
        } catch (e) {
          console.error("Auto-lock failed", e);
        }
      }, 5 * 60 * 1000); // 5 minutes
    }
    return () => clearTimeout(timeout);
  }, [isUnlocked]);

  const handleSaveNode = async (updatedNode: Node) => {
    const updateTree = (list: Node[]): Node[] => {
      return list.map((n) => {
        if (n.id === updatedNode.id) return updatedNode;
        if (n.children) return { ...n, children: updateTree(n.children) };
        return n;
      });
    };

    const newNodes = updateTree(nodes);
    setNodes(newNodes);
    await api.saveData(newNodes);
  };

  const handleLockVault = async () => {
    await api.lockVault();
    setIsUnlocked(false);

    // Immediate memory purge
    const purgeSecrets = (list: Node[]): Node[] => {
      return list.map(n => ({
        ...n,
        value: n.isSecret ? "" : n.value,
        children: n.children ? purgeSecrets(n.children) : undefined
      }));
    };

    setNodes(prev => purgeSecrets(prev));
    if (selectedNode?.isSecret) {
      setSelectedNode(prev => prev ? { ...prev, value: "" } : null);
    }

    loadNodes();
  };

  const handleAddNode = async (parentId: string | null, type: "folder" | "snippet") => {
    const newNode: Node = {
      id: crypto.randomUUID(),
      type,
      label: type === "folder" ? "New Folder" : "New Snippet",
      parentId,
      createdAt: Date.now(),
      value: type === "snippet" ? "" : undefined,
      children: type === "folder" ? [] : undefined,
    };

    let newNodes: Node[];
    if (parentId === null) {
      newNodes = [...nodes, newNode];
    } else {
      const addToParent = (list: Node[]): Node[] => {
        return list.map((n) => {
          if (n.id === parentId) {
            return { ...n, children: [...(n.children || []), newNode] };
          }
          if (n.children) return { ...n, children: addToParent(n.children) };
          return n;
        });
      };
      newNodes = addToParent(nodes);
    }

    setNodes(newNodes);
    setSelectedNode(newNode);
    await api.saveData(newNodes);
  };

  const handleDeleteNode = async (nodeId: string) => {
    const removeFromTree = (list: Node[]): Node[] => {
      return list
        .filter((n) => n.id !== nodeId)
        .map((n) => {
          if (n.children) return { ...n, children: removeFromTree(n.children) };
          return n;
        });
    };

    const newNodes = removeFromTree(nodes);
    setNodes(newNodes);
    if (selectedNode?.id === nodeId) setSelectedNode(null);
    await api.saveData(newNodes);
  };

  const handleRenameNode = async (nodeId: string, newLabel: string) => {
    const updateTree = (list: Node[]): Node[] => {
      return list.map((n) => {
        if (n.id === nodeId) return { ...n, label: newLabel };
        if (n.children) return { ...n, children: updateTree(n.children) };
        return n;
      });
    };

    const newNodes = updateTree(nodes);
    setNodes(newNodes);
    // If the renamed node is selected, update it too
    if (selectedNode?.id === nodeId) {
      setSelectedNode({ ...selectedNode, label: newLabel });
    }
    await api.saveData(newNodes);
  };

  const handleMoveNode = async (draggedId: string, targetId: string | null, beforeId?: string | null) => {
    // Find and remove the dragged node from its current position
    let draggedNode: Node | null = null;

    const removeNode = (list: Node[]): Node[] => {
      return list.filter((n) => {
        if (n.id === draggedId) {
          draggedNode = { ...n, parentId: targetId };
          return false;
        }
        if (n.children) {
          n.children = removeNode(n.children);
        }
        return true;
      });
    };

    let newNodes = removeNode([...nodes]);

    if (!draggedNode) return; // Node not found

    // Prevent dropping a folder into itself or its descendants
    if (targetId) {
      const isDescendant = (nodeId: string, potentialAncestorId: string): boolean => {
        const findNode = (list: Node[]): Node | undefined => {
          for (const n of list) {
            if (n.id === nodeId) return n;
            if (n.children) {
              const found = findNode(n.children);
              if (found) return found;
            }
          }
          return undefined;
        };

        const checkDescendants = (node: Node): boolean => {
          if (node.id === potentialAncestorId) return true;
          if (node.children) {
            return node.children.some(checkDescendants);
          }
          return false;
        };

        const ancestor = findNode(nodes);
        return ancestor ? checkDescendants(ancestor) : false;
      };

      if (draggedId === targetId || isDescendant(draggedId, targetId)) {
        return; // Cannot drop into self or descendant
      }
    }

    // Helper to insert at position
    const insertAtPosition = (list: Node[], nodeToInsert: Node, beforeNodeId: string | null | undefined): Node[] => {
      if (beforeNodeId === null || beforeNodeId === undefined) {
        // Insert at end
        return [...list, nodeToInsert];
      }
      const index = list.findIndex(n => n.id === beforeNodeId);
      if (index === -1) {
        // beforeId not found, insert at end
        return [...list, nodeToInsert];
      }
      // Insert before the found index
      const result = [...list];
      result.splice(index, 0, nodeToInsert);
      return result;
    };

    // Add the node to its new location
    if (targetId === null) {
      // Move to root
      newNodes = insertAtPosition(newNodes, draggedNode, beforeId);
    } else {
      // Move into a folder
      const addToParent = (list: Node[]): Node[] => {
        return list.map((n) => {
          if (n.id === targetId) {
            return { ...n, children: insertAtPosition(n.children || [], draggedNode!, beforeId) };
          }
          if (n.children) {
            return { ...n, children: addToParent(n.children) };
          }
          return n;
        });
      };
      newNodes = addToParent(newNodes);
    }

    setNodes(newNodes);
    await api.saveData(newNodes);
  };

  if (!loaded || !settings) {
    return (
      <div className="flex items-center justify-center h-screen bg-background">
        <div className="flex flex-col items-center gap-4 animate-fade-in">
          <Container className="w-12 h-12 text-primary animate-float" />
          <span className="text-muted-foreground font-medium">Loading warehouse...</span>
        </div>
      </div>
    );
  }


  return (
    <div className="flex h-screen w-screen overflow-hidden bg-industrial text-foreground">
      <CommandPalette nodes={nodes} onSelect={setSelectedNode} />
      <Sidebar
        nodes={nodes}
        onSelectNode={setSelectedNode}
        selectedNodeId={selectedNode?.id}
        onAddNode={handleAddNode}
        onDeleteNode={handleDeleteNode}
        onRenameNode={handleRenameNode}
        onMoveNode={handleMoveNode}
      />
      <main className="flex-1 overflow-hidden flex flex-col">
        {/* Header */}
        <header className="flex items-center justify-between px-4 py-3 border-b bg-card/50 backdrop-blur-sm">
          <div className="flex items-center gap-3">
            {/* <Container className="w-5 h-5 text-primary" /> */}
            <h1 className="text-sm font-semibold tracking-wide text-foreground/80">
              {selectedNode?.id === 'settings' ? 'SKLAD SETTINGS' : 'SNIPPETS'}
            </h1>
          </div>
          <div className="flex items-center gap-2">
            {isUnlocked ? (
              <button
                className="flex items-center gap-2 px-3 py-1.5 text-xs text-yellow-500 bg-yellow-500/10 rounded-md border border-yellow-500/30 hover:bg-yellow-500/20 transition-smooth font-medium"
                onClick={handleLockVault}
                title="Lock Vault Now"
              >
                <Lock className="w-3 h-3" />
                <span>Lock Vault</span>
              </button>
            ) : (
              settings.security.masterPasswordEnabled && (
                <button
                  className="flex items-center gap-2 px-3 py-1.5 text-xs text-primary bg-primary/10 rounded-md border border-primary/30 hover:bg-primary/20 transition-smooth font-bold shadow-glow-sm"
                  onClick={() => setShowLockModal(true)}
                  title="Unlock Vault"
                >
                  <Unlock className="w-3 h-3" />
                  <span>Unlock Vault</span>
                </button>
              )
            )}
            <button
              className="flex items-center gap-2 px-3 py-1.5 text-xs text-muted-foreground bg-secondary/50 rounded-md border border-border hover:bg-secondary transition-smooth"
              onClick={() => {
                const event = new KeyboardEvent('keydown', { key: 'k', ctrlKey: true });
                document.dispatchEvent(event);
              }}
            >
              <Search className="w-3 h-3" />
              <span>Search</span>
              <kbd className="px-1.5 py-0.5 text-[10px] bg-muted rounded font-mono">⌘K</kbd>
            </button>
            <ThemeToggle />
          </div>
        </header>

        {/* Content */}
        <div className={`flex-1 ${selectedNode?.id === 'settings' ? 'overflow-auto' : 'overflow-hidden'}`}>
          {selectedNode?.id === 'settings' ? (
            <Settings
              settings={settings}
              onSetupTrigger={() => setShowSetupModal(true)}
              onSettingsUpdate={async (newSettings) => {
                setSettings(newSettings);
                await api.saveSettings(newSettings);
              }}
              onResetTrigger={async () => {
                const [newNodes, newSettings] = await api.resetVault();
                setNodes(newNodes);
                setSettings(newSettings);
                setIsUnlocked(false);
              }}
            />
          ) : selectedNode ? (
            <SnippetEditor
              node={selectedNode}
              key={selectedNode.id}
              onSave={handleSaveNode}
              masterPasswordEnabled={settings.security.masterPasswordEnabled}
              isUnlocked={isUnlocked}
              onUnlockTrigger={() => setShowLockModal(true)}
            />
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-4">
              <Container className="w-16 h-16 text-muted-foreground/30" />
              <div className="text-center">
                <p className="font-medium">No item selected</p>
                <p className="text-sm text-muted-foreground/70 mt-1">Select a folder or snippet from the sidebar</p>
              </div>
            </div>
          )}
        </div>
      </main>

      {showLockModal && (
        <VaultLock
          mode="modal"
          onUnlock={async () => {
            setShowLockModal(false);
            await loadNodes();
            if (pendingCopyAction) {
              try {
                await api.copySnippet(pendingCopyAction.id);
                if (pendingCopyAction.autoHide) {
                  const window = getCurrentWebviewWindow();
                  await window.hide();
                }
              } catch (e) {
                console.error("Failed to copy pending snippet:", e);
              }
              setPendingCopyAction(null);
            }
          }}
          onCancel={() => {
            setShowLockModal(false);
            setPendingCopyAction(null);
          }}
          onReset={(newNodes, newSettings) => {
            setNodes(newNodes);
            setSettings(newSettings);
            setIsUnlocked(false);
            setShowLockModal(false);
          }}
          isInit={false}
        />
      )}

      {showSetupModal && (
        <VaultLock
          mode="modal"
          isInit={true}
          onUnlock={() => {
            setShowSetupModal(false);
            initializeApp(); // Refresh settings
          }}
          onCancel={() => setShowSetupModal(false)}
          onReset={(newNodes, newSettings) => {
            setNodes(newNodes);
            setSettings(newSettings);
            setIsUnlocked(false);
            setShowSetupModal(false);
          }}
        />
      )}
    </div>
  );
}

export default App;
