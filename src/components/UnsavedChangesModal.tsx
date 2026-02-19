import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { AlertTriangle } from "lucide-react";

interface UnsavedChangesModalProps {
    open: boolean;
    onSave: () => void;
    onDiscard: () => void;
    onCancel: () => void;
}

export function UnsavedChangesModal({ open, onSave, onDiscard, onCancel }: UnsavedChangesModalProps) {
    return (
        <Dialog open={open} onOpenChange={(open) => !open && onCancel()}>
            <DialogContent className="sm:max-w-[425px]">
                <DialogHeader>
                    <div className="flex items-center gap-2 text-yellow-500 mb-2">
                        <AlertTriangle className="h-5 w-5" />
                        <DialogTitle>Unsaved Changes</DialogTitle>
                    </div>
                    <DialogDescription>
                        You have unsaved changes in the current snippet. What would you like to do?
                    </DialogDescription>
                </DialogHeader>
                <DialogFooter className="flex gap-2 sm:gap-0 mt-4">
                    <Button variant="outline" onClick={onCancel}>
                        Cancel
                    </Button>
                    <Button variant="destructive" onClick={onDiscard}>
                        Discard
                    </Button>
                    <Button onClick={onSave} className="bg-primary hover:bg-primary/90">
                        Save Changes
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
