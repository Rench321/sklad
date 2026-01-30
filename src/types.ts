export type NodeType = 'folder' | 'snippet';

export interface Node {
    id: string;             // UUID v4
    type: NodeType;
    label: string;          // Name of the folder or snippet
    parentId: string | null; // null for root
    createdAt: number;      // Timestamp

    // Fields for Folder
    children?: Node[];      // Recursive children

    // Fields for Snippet
    value?: string;         // Plain text (if public)
    encryptedValue?: string;// Hex string (if private)
    isSecret?: boolean;     // Requires unlock to copy?
}

export interface AppSettings {
    theme: 'dark' | 'light' | 'system';
    security: {
        lockTimeout: number; // Minutes to auto-lock (0 = disabled)
        clearClipboard: boolean; // Clear after 30s
        masterPasswordEnabled: boolean;
    };
    notificationsEnabled: boolean;
}
