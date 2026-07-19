import { useState } from "react";
import {
  ArchiveRestore,
  FileWarning,
  FolderOpen,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
  ShieldAlert,
} from "lucide-react";
import { api } from "@/lib/api";
import { StorageIssue, StorageStatus } from "@/types";
import { Button } from "@/components/ui/button";

interface StorageRecoveryProps {
  status: StorageStatus;
  onRetry: () => Promise<void>;
}

type RecoveryAction = "restore" | "reset-data" | "reset-settings" | "retry" | "open";
type Confirmation = "data" | "settings" | null;

const actionClassName =
  "min-h-10 active:scale-[0.96] transition-[color,background-color,border-color,box-shadow,transform]";

function readableError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Recovery could not be completed. The original file was not replaced.";
}

function IssueCard({ issue }: { issue: StorageIssue }) {
  const isInvalid = issue.kind === "invalid_format";

  return (
    <section className="rounded-xl bg-muted/45 p-4 shadow-[0_0_0_1px_rgba(0,0,0,0.05)] dark:shadow-[0_0_0_1px_rgba(255,255,255,0.08)]">
      <div className="flex items-start gap-3">
        <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-destructive/10 text-destructive">
          <FileWarning aria-hidden="true" className="size-5" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <h2 className="font-mono text-sm font-semibold">{issue.fileName}</h2>
            <span className="rounded-full bg-destructive/10 px-2 py-0.5 text-xs font-medium text-destructive">
              {isInvalid ? "Invalid format" : "Unreadable"}
            </span>
          </div>
          <p className="mt-2 text-pretty text-sm leading-6 text-muted-foreground">
            {isInvalid
              ? "The JSON or its expected structure is invalid. Sklad will not load or overwrite this file until you choose a recovery action."
              : "Sklad cannot safely read this file. Check its permissions or disk availability, then retry."}
          </p>
          <code className="mt-3 block overflow-x-auto rounded-lg bg-background/75 px-3 py-2 font-mono text-xs leading-5 text-muted-foreground shadow-[inset_0_0_0_1px_rgba(0,0,0,0.05)] dark:shadow-[inset_0_0_0_1px_rgba(255,255,255,0.07)]">
            {issue.reason}
          </code>
        </div>
      </div>
    </section>
  );
}

export function StorageRecovery({ status, onRetry }: StorageRecoveryProps) {
  const [busyAction, setBusyAction] = useState<RecoveryAction | null>(null);
  const [confirmation, setConfirmation] = useState<Confirmation>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const isBusy = busyAction !== null;

  const runAction = async (
    action: RecoveryAction,
    operation: () => Promise<void>,
  ) => {
    setBusyAction(action);
    setError(null);
    setNotice(null);
    try {
      await operation();
    } catch (operationError) {
      setError(readableError(operationError));
    } finally {
      setBusyAction(null);
    }
  };

  const restoreBackup = () => {
    const backup = status.newestValidBackup;
    if (!backup) return;

    void runAction("restore", async () => {
      await api.restoreBackup(backup.filename);
      setNotice("The newest valid backup was restored.");
      await onRetry();
    });
  };

  const resetData = () => {
    void runAction("reset-data", async () => {
      const quarantine = await api.resetCorruptData();
      setConfirmation(null);
      setNotice(`The original data was preserved as ${quarantine}.`);
      await onRetry();
    });
  };

  const resetSettings = () => {
    void runAction("reset-settings", async () => {
      const quarantine = await api.resetCorruptSettings();
      setConfirmation(null);
      setNotice(`The original settings were preserved as ${quarantine}.`);
      await onRetry();
    });
  };

  const retry = () => {
    void runAction("retry", onRetry);
  };

  const openDirectory = () => {
    void runAction("open", () => api.openDataDirectory());
  };

  return (
    <main className="flex min-h-screen w-screen items-center justify-center overflow-auto bg-industrial p-6 text-foreground">
      <div className="w-full max-w-2xl rounded-2xl bg-card p-6 shadow-xl ring-1 ring-black/5 dark:ring-white/10">
        <header className="flex items-start gap-4">
          <div className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-primary/12 text-primary shadow-[0_0_0_1px_rgba(0,0,0,0.04)] dark:shadow-[0_0_0_1px_rgba(255,255,255,0.08)]">
            <ShieldAlert aria-hidden="true" className="size-6" />
          </div>
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.18em] text-primary">
              Recovery mode
            </p>
            <h1 className="mt-1 text-balance text-2xl font-semibold tracking-tight">
              Storage needs attention
            </h1>
            <p className="mt-2 max-w-xl text-pretty text-sm leading-6 text-muted-foreground">
              Your files have not been replaced. Normal editing and automatic backups are paused until storage is healthy again.
            </p>
          </div>
        </header>

        <div className="mt-6 space-y-3">
          {status.dataIssue && <IssueCard issue={status.dataIssue} />}
          {status.settingsIssue && <IssueCard issue={status.settingsIssue} />}
        </div>

        {status.dataIssue?.kind === "invalid_format" && (
          <section className="mt-5 rounded-xl bg-primary/6 p-4 shadow-[0_0_0_1px_rgba(0,0,0,0.04)] dark:shadow-[0_0_0_1px_rgba(255,255,255,0.07)]">
            <h2 className="text-sm font-semibold">Snippet recovery</h2>
            {status.newestValidBackup ? (
              <>
                <p className="mt-1 text-pretty text-sm leading-6 text-muted-foreground">
                  A valid backup is available: <span className="font-mono text-foreground">{status.newestValidBackup.filename}</span>.
                </p>
                <Button
                  className={`mt-3 ${actionClassName}`}
                  disabled={isBusy}
                  onClick={restoreBackup}
                  size="lg"
                >
                  {busyAction === "restore" ? (
                    <LoaderCircle aria-hidden="true" className="animate-spin motion-reduce:animate-none" />
                  ) : (
                    <ArchiveRestore aria-hidden="true" />
                  )}
                  Restore newest valid backup
                </Button>
              </>
            ) : (
              <p className="mt-1 text-pretty text-sm leading-6 text-muted-foreground">
                No valid snippet backup was found. You can inspect the file manually or create a fresh library after preserving it in quarantine.
              </p>
            )}

            {confirmation === "data" ? (
              <div className="mt-4 rounded-lg bg-destructive/8 p-3 shadow-[0_0_0_1px_rgba(220,38,38,0.18)]">
                <p className="text-pretty text-sm font-medium">
                  Create a fresh library? The damaged file will first be copied to quarantine.
                </p>
                <div className="mt-3 flex flex-wrap gap-2">
                  <Button
                    className={actionClassName}
                    disabled={isBusy}
                    onClick={() => setConfirmation(null)}
                    variant="outline"
                  >
                    Cancel
                  </Button>
                  <Button
                    className={actionClassName}
                    disabled={isBusy}
                    onClick={resetData}
                    variant="destructive"
                  >
                    {busyAction === "reset-data" && (
                      <LoaderCircle aria-hidden="true" className="animate-spin motion-reduce:animate-none" />
                    )}
                    Quarantine and reset data
                  </Button>
                </div>
              </div>
            ) : (
              <Button
                className={`mt-3 ${actionClassName}`}
                disabled={isBusy}
                onClick={() => setConfirmation("data")}
                variant="outline"
              >
                <RotateCcw aria-hidden="true" />
                Create fresh library
              </Button>
            )}
          </section>
        )}

        {status.settingsIssue?.kind === "invalid_format" && (
          <section className="mt-5 rounded-xl bg-muted/35 p-4 shadow-[0_0_0_1px_rgba(0,0,0,0.04)] dark:shadow-[0_0_0_1px_rgba(255,255,255,0.07)]">
            <h2 className="text-sm font-semibold">Settings recovery</h2>
            <p className="mt-1 text-pretty text-sm leading-6 text-muted-foreground">
              Sklad does not currently back up settings. Resetting restores defaults only after preserving the damaged file.
            </p>
            {status.hasEncryptedSecrets && (
              <p className="mt-3 text-pretty rounded-lg bg-destructive/8 px-3 py-2 text-sm leading-6 text-destructive shadow-[0_0_0_1px_rgba(220,38,38,0.18)]">
                Encrypted snippets were detected. Resetting settings can make them unavailable because the original password verifier and derivation salt are stored in this file.
              </p>
            )}

            {confirmation === "settings" ? (
              <div className="mt-4 rounded-lg bg-destructive/8 p-3 shadow-[0_0_0_1px_rgba(220,38,38,0.18)]">
                <p className="text-pretty text-sm font-medium">
                  Reset settings to defaults and preserve the original file in quarantine?
                </p>
                <div className="mt-3 flex flex-wrap gap-2">
                  <Button
                    className={actionClassName}
                    disabled={isBusy}
                    onClick={() => setConfirmation(null)}
                    variant="outline"
                  >
                    Cancel
                  </Button>
                  <Button
                    className={actionClassName}
                    disabled={isBusy}
                    onClick={resetSettings}
                    variant="destructive"
                  >
                    {busyAction === "reset-settings" && (
                      <LoaderCircle aria-hidden="true" className="animate-spin motion-reduce:animate-none" />
                    )}
                    Quarantine and reset settings
                  </Button>
                </div>
              </div>
            ) : (
              <Button
                className={`mt-3 ${actionClassName}`}
                disabled={isBusy}
                onClick={() => setConfirmation("settings")}
                variant="outline"
              >
                <RotateCcw aria-hidden="true" />
                Reset settings
              </Button>
            )}
          </section>
        )}

        <div aria-live="polite" className="mt-5 min-h-5 text-sm">
          {error && <p className="text-pretty text-destructive">{error}</p>}
          {!error && notice && <p className="text-pretty text-muted-foreground">{notice}</p>}
        </div>

        <footer className="mt-3 flex flex-wrap gap-2 border-t border-border/60 pt-4">
          <Button
            className={actionClassName}
            disabled={isBusy}
            onClick={retry}
            variant="secondary"
          >
            {busyAction === "retry" ? (
              <LoaderCircle aria-hidden="true" className="animate-spin motion-reduce:animate-none" />
            ) : (
              <RefreshCw aria-hidden="true" />
            )}
            Retry
          </Button>
          <Button
            className={actionClassName}
            disabled={isBusy}
            onClick={openDirectory}
            variant="ghost"
          >
            <FolderOpen aria-hidden="true" />
            Open data folder
          </Button>
        </footer>
      </div>
    </main>
  );
}
