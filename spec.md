# SYSTEM SPECIFICATION: Sklad (Tray Snippet Manager)

## 1. Project Identity
*   **Name:** **Sklad**
*   **Binary Name:** `sklad`
*   **Tagline:** "Industrial-grade secure snippet warehouse for your system tray."
*   **Vibe:** Minimalist, Brutalist, Reliable. Think "Container Shipping" aesthetics.
*   **Distribution:** Open Source (MIT).

## 2. Tech Stack (Strict Constraints)
*   **Core:** Rust + **Tauri v2** (Beta/Stable).
*   **Frontend:** TypeScript + React + Vite.
*   **Package Manager:** **pnpm**.
*   **Styling:** Tailwind CSS + **shadcn/ui**.
*   **Icons:** Lucide React (Box, Container, Lock icons).
*   **State Persistence:** `tauri-plugin-store` (JSON file).
*   **Clipboard:** `tauri-plugin-clipboard-manager`.
*   **Encryption:** Rust crate `aes-gcm` + `argon2` (or `pbkdf2`).

## 3. Data Architecture (The "Warehouse" Structure)
Data is stored in `sklad.json` using a recursive tree structure.

### 3.1. TypeScript Interfaces
```typescript
type NodeType = 'folder' | 'snippet';

interface Node {
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

interface AppSettings {
  theme: 'dark' | 'light' | 'system';
  security: {
    lockTimeout: number; // Minutes to auto-lock (0 = disabled)
    clearClipboard: boolean; // Clear after 30s
  }
}
```

## 4. Functional Requirements

### 4.1. System Tray (Rust Logic)
The tray menu must be **dynamic** and **recursive**.
*   **Left Click:** Copy the *last used* snippet. Send a system notification: "Sklad: Copied [Label]".
*   **Right Click:** Open Context Menu.
    *   Render folders as Submenus.
    *   Render secret snippets with a Lock icon (🔒).
    *   If the vault is locked, clicking a secret snippet triggers the Password Prompt.

### 4.2. Security Model (Encryption)
*   **Vault Status:** `LOCKED` (Default on start) or `UNLOCKED` (Key in RAM).
*   **Master Password:** Hashed and stored in config.
*   **Flow:**
    1.  User clicks a Secret Snippet.
    2.  If `LOCKED` -> Open Modal "Enter Warehouse Key".
    3.  If Password OK -> Derive AES Key -> Decrypt -> Copy to Clipboard.

### 4.3. Management UI (React Window)
*   **Style:** Clean, Dark Mode. Font: `Inter` or `JetBrains Mono`.
*   **Sidebar:** Tree view of folders (The "Shelves").
*   **Main Area:** List of snippets or Editor form.
*   **Global Search (Cmd+K):** Instant fuzzy search across the entire warehouse.

## 5. Development Phases (Step-by-Step for AI)

**DO NOT** generate all code at once. Follow these phases strictly.

### Phase 1: Foundation
1.  Initialize Tauri v2 with React & pnpm.
2.  Set up Tailwind & Shadcn/UI.
3.  Define the `Node` struct in Rust and Types.
4.  Create a dummy `sklad.json` with nested folders for testing.

### Phase 2: The Logic (Rust Backend)
1.  Implement `DataManager`: Load/Save JSON using `tauri-plugin-store`.
2.  Implement `TrayGenerator`: Recursive function to build the native Tray Menu from the JSON tree.
3.  Implement basic "Copy" functionality.

### Phase 3: The Security (Encryption)
1.  Add `aes-gcm`.
2.  Implement `VaultState` in Rust (holding the key in `Mutex<Option<Key>>`).
3.  Implement `encrypt` and `decrypt` commands.
4.  Add the Master Password setup flow.

### Phase 4: The Interface (UI)
1.  Build the **Tree/Explorer** component.
2.  Build the **Snippet Editor** (Label, Value, Secret Toggle).
3.  Implement Search.

### Phase 5: CI/CD
1.  GitHub Actions for `.msi`, `.dmg`, `.deb`.