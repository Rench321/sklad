export type NodeType = 'folder' | 'snippet';

export interface BackupInfo {
    filename: string;
    timestamp: number;
    size: number;
}

export type StorageFile = 'data' | 'settings';
export type StorageIssueKind = 'invalid_format' | 'unreadable';

export interface StorageIssue {
    file: StorageFile;
    kind: StorageIssueKind;
    fileName: string;
    reason: string;
}

export interface StorageStatus {
    dataIssue: StorageIssue | null;
    settingsIssue: StorageIssue | null;
    newestValidBackup: BackupInfo | null;
    hasEncryptedSecrets: boolean;
}

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
        lockTimeout: number;
        clearClipboard: boolean;
        masterPasswordEnabled: boolean;
    };
    loggingEnabled: boolean;
    notificationsEnabled: boolean;
    launchAtStartup: boolean;
    autoSave: boolean;
    globalSearchShortcut: string;
    globalCreateShortcut: string;
    globalSearchAction?: 'copy' | 'open';
    trayClickAction?: 'copy_last' | 'open_app';
    trayMenuRootPosition?: 'top' | 'bottom';
    autoBackupEnabled?: boolean;
    autoBackupCount?: number;
}
