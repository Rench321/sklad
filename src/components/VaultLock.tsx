import { useState } from "react";
import { api } from "../lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Lock, Unlock, Container, AlertCircle, Eye, EyeOff } from "lucide-react";
import { cn } from "@/lib/utils";

interface VaultLockProps {
    onUnlock: () => void;
    onReset: (nodes: any[], settings: any) => void;
    onCancel?: () => void;
    isInit?: boolean;
    mode?: 'full' | 'modal';
}

export function VaultLock({ onUnlock, onReset, onCancel, isInit = false, mode = 'full' }: VaultLockProps) {
    const [password, setPassword] = useState("");
    const [error, setError] = useState("");
    const [loading, setLoading] = useState(false);
    const [showOptions, setShowOptions] = useState(false);
    const [confirmReset, setConfirmReset] = useState(false);
    const [showPassword, setShowPassword] = useState(false);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setError("");

        try {
            if (isInit) {
                await api.initVault(password);
            } else {
                const success = await api.unlockVault(password);
                if (!success) {
                    setError("Invalid password. Please try again.");
                    setLoading(false);
                    return;
                }
            }
            onUnlock();
        } catch (err) {
            setError("Invalid password. Please try again.");
        } finally {
            setLoading(false);
        }
    };

    const handleReset = async () => {
        setLoading(true);
        try {
            const [nodes, settings] = await api.resetVault();
            onReset(nodes, settings);
        } catch (err) {
            setError("Failed to reset vault.");
            setLoading(false);
        }
    };

    return (
        <div className={cn(
            "flex items-center justify-center relative overflow-hidden",
            mode === 'full' ? "min-h-screen bg-industrial" : "p-4 h-full w-full bg-background/80 backdrop-blur-xl animate-in fade-in duration-300 z-50 fixed inset-0"
        )}>
            {/* Background grid pattern */}
            <div className="absolute inset-0 bg-grid opacity-30" />

            {/* Floating container icons */}
            <div className="absolute inset-0 overflow-hidden pointer-events-none">
                <Container className="absolute top-[15%] left-[10%] w-24 h-24 text-primary/10 animate-float" style={{ animationDelay: '0s' }} />
                <Container className="absolute top-[60%] right-[15%] w-16 h-16 text-primary/8 animate-float" style={{ animationDelay: '2s' }} />
                <Container className="absolute bottom-[20%] left-[20%] w-20 h-20 text-primary/6 animate-float" style={{ animationDelay: '4s' }} />
            </div>

            {/* Main card */}
            <Card className="w-[380px] glass border-border/50 shadow-2xl animate-fade-in relative z-10">
                <CardHeader className="space-y-4 pb-4">
                    {/* Logo */}
                    <div className="flex justify-center">
                        <div className="p-4 rounded-2xl bg-primary/10 border border-primary/20 group hover:bg-primary/15 transition-smooth">
                            <Container className="w-10 h-10 text-primary transition-transform group-hover:scale-110" />
                        </div>
                    </div>

                    <div className="text-center space-y-2">
                        <CardTitle className="text-xl font-bold tracking-tight flex items-center justify-center gap-2">
                            {isInit ? <Unlock className="w-5 h-5 text-primary" /> : <Lock className="w-5 h-5 text-primary" />}
                            {isInit ? "Initialize Vault" : "Unlock Sklad"}
                        </CardTitle>
                        <CardDescription className="text-muted-foreground/80">
                            {isInit
                                ? "Set a master password for your snippet"
                                : "Enter your master password to access your snippets"}
                        </CardDescription>
                    </div>
                </CardHeader>

                <form onSubmit={handleSubmit}>
                    <CardContent className="space-y-4">
                        <div className="relative group/input">
                            <Input
                                id="password"
                                type={showPassword ? "text" : "password"}
                                placeholder="Master Password"
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                autoFocus
                                className="h-11 pr-10 bg-background/50 border-border/60 focus:border-primary focus:ring-primary/30 transition-all placeholder:text-muted-foreground/50"
                            />
                            <button
                                type="button"
                                onClick={() => setShowPassword(!showPassword)}
                                className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground/40 hover:text-primary transition-colors focus:outline-none"
                            >
                                {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                            </button>
                        </div>

                        {error && (
                            <div className="flex items-center gap-2 text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded-lg px-3 py-2 animate-fade-in">
                                <AlertCircle className="w-4 h-4 flex-shrink-0" />
                                <span>{error}</span>
                            </div>
                        )}
                    </CardContent>

                    <CardFooter className="flex flex-col gap-4">
                        <div className="flex flex-col gap-2 w-full">
                            <Button
                                type="submit"
                                className="w-full h-11 font-semibold bg-primary hover:bg-primary/90 text-primary-foreground shadow-lg hover:shadow-xl transition-all hover:glow-sm"
                                disabled={loading || !password}
                            >
                                {loading && !showOptions ? (
                                    <span className="flex items-center gap-2">
                                        <div className="w-4 h-4 border-2 border-primary-foreground/30 border-t-primary-foreground rounded-full animate-spin" />
                                        Processing...
                                    </span>
                                ) : (
                                    <span className="flex items-center gap-2">
                                        {isInit ? <Unlock className="w-4 h-4" /> : <Lock className="w-4 h-4" />}
                                        {isInit ? "Initialize" : "Unlock"}
                                    </span>
                                )}
                            </Button>

                            {mode === 'modal' && onCancel && (
                                <Button
                                    type="button"
                                    variant="outline"
                                    onClick={onCancel}
                                    className="w-full h-10 text-xs text-muted-foreground hover:text-foreground"
                                >
                                    Cancel
                                </Button>
                            )}
                        </div>

                        {!isInit && !showOptions && (
                            <div className="w-full flex flex-col gap-4">
                                {mode === 'full' && (
                                    <Button
                                        type="button"
                                        variant="ghost"
                                        className="w-full h-11 text-xs gap-3 hover:bg-primary/5 active:bg-primary/10 transition-all border border-dashed border-border/40 hover:border-primary/20 hover:text-foreground font-medium"
                                        onClick={onUnlock}
                                        disabled={loading}
                                    >
                                        <Unlock className="w-4 h-4 text-muted-foreground/60" />
                                        <span>Access Public Snippets (No Password)</span>
                                    </Button>
                                )}

                                <button
                                    type="button"
                                    onClick={() => setShowOptions(true)}
                                    className="text-[10px] text-muted-foreground/60 hover:text-primary transition-colors font-medium underline-offset-4 hover:underline mx-auto"
                                >
                                    Forgot Password?
                                </button>
                            </div>
                        )}

                        {showOptions && (
                            <div className="w-full space-y-3 pt-2 border-t border-border/40 animate-in fade-in slide-in-from-top-2 duration-300">
                                <div className="text-[10px] uppercase tracking-wider font-bold text-muted-foreground/50 px-1">
                                    Emergency Recovery
                                </div>

                                <Button
                                    type="button"
                                    variant="ghost"
                                    className={cn(
                                        "w-full h-10 text-xs justify-start gap-3 transition-colors",
                                        confirmReset ? "bg-destructive/10 text-destructive hover:bg-destructive/15" : "hover:bg-destructive/10 text-muted-foreground hover:text-destructive"
                                    )}
                                    onClick={() => confirmReset ? handleReset() : setConfirmReset(true)}
                                    disabled={loading}
                                >
                                    <AlertCircle className="w-3.5 h-3.5" />
                                    <span>
                                        {confirmReset ? "Click again to PERMANENTLY DELETE SECRETS" : "Reset Vault (Deletes All Secrets)"}
                                    </span>
                                </Button>

                                <button
                                    type="button"
                                    onClick={() => { setShowOptions(false); setConfirmReset(false); }}
                                    className="w-full text-[10px] text-muted-foreground/40 hover:text-muted-foreground transition-colors pt-1"
                                >
                                    Cancel
                                </button>
                            </div>
                        )}
                    </CardFooter>
                </form>
            </Card>

            {/* Version indicator */}
            <div className="absolute bottom-4 text-xs text-muted-foreground/40 font-mono">
                SKLAD v1.0
            </div>
        </div>
    );
}
