import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { api } from "@/lib/api";

export async function routeToStorageRecovery(): Promise<boolean> {
  const status = await api.getStorageStatus();
  if (!status.dataIssue && !status.settingsIssue) return false;

  const mainWindow = await WebviewWindow.getByLabel("main");
  if (mainWindow) {
    await mainWindow.show();
    await mainWindow.setFocus();
  }
  await getCurrentWebviewWindow().hide();
  return true;
}
