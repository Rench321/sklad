import { Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

export function ThemeToggle() {
    const [theme, setTheme] = useState<"light" | "dark">("dark");
    const [isAnimating, setIsAnimating] = useState(false);

    useEffect(() => {
        const stored = localStorage.getItem("sklad-theme") as "light" | "dark" | null;
        const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        const initialTheme = stored || (prefersDark ? "dark" : "light");
        setTheme(initialTheme);
        document.documentElement.classList.toggle("dark", initialTheme === "dark");
    }, []);

    const toggleTheme = () => {
        setIsAnimating(true);
        const newTheme = theme === "light" ? "dark" : "light";
        setTheme(newTheme);
        localStorage.setItem("sklad-theme", newTheme);
        document.documentElement.classList.toggle("dark", newTheme === "dark");
        setTimeout(() => setIsAnimating(false), 500);
    };

    return (
        <Button
            variant="ghost"
            size="icon"
            onClick={toggleTheme}
            className={cn(
                "h-8 w-8 rounded-lg hover:bg-primary/10 hover:text-primary transition-all",
                "relative overflow-hidden"
            )}
            title={theme === "light" ? "Switch to dark mode" : "Switch to light mode"}
        >
            <div className={cn(
                "transition-all duration-300",
                isAnimating && "animate-rotate"
            )}>
                {theme === "light" ? (
                    <Moon className="w-4 h-4" />
                ) : (
                    <Sun className="w-4 h-4 text-yellow-400" />
                )}
            </div>
        </Button>
    );
}
