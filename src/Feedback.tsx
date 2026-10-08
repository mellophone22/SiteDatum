import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

type Tone = "success" | "warning" | "error";

export function StatusNotice({ tone, children }: { tone: Tone; children: ReactNode }) {
  return <p className={`status ${tone}`} role={tone === "error" ? "alert" : "status"}>{children}</p>;
}

export function EmptyState({ title, children, action }: { title?: string; children: ReactNode; action?: ReactNode }) {
  return <div className="empty-state">
    {title && <strong className="empty-state-title">{title}</strong>}
    <p>{children}</p>
    {action && <div className="empty-state-action">{action}</div>}
  </div>;
}

export function FieldError({ id, children }: { id: string; children?: ReactNode }) {
  if (!children) return null;
  return <span id={id} className="field-error">{children}</span>;
}

export function LoadingState({ children = "Loading…" }: { children?: ReactNode }) {
  return <p className="empty-state loading-state" role="status" aria-live="polite">{children}</p>;
}

export type ConfirmationOptions = {
  title: string;
  description: ReactNode;
  confirmLabel: string;
  destructive?: boolean;
};

function ConfirmationDialog({ options, onDecision }: { options: ConfirmationOptions; onDecision: (confirmed: boolean) => void }) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const titleId = useRef(`confirmation-${crypto.randomUUID()}`);

  useEffect(() => {
    const previousFocus = document.activeElement as HTMLElement | null;
    cancelRef.current?.focus();
    return () => previousFocus?.focus();
  }, []);

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape") { event.preventDefault(); onDecision(false); return; }
    if (event.key !== "Tab") return;
    const buttons = Array.from(dialogRef.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
    if (!buttons.length) return;
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.shiftKey ? (index <= 0 ? buttons.length - 1 : index - 1) : (index === buttons.length - 1 ? 0 : index + 1);
    event.preventDefault(); buttons[next]?.focus();
  }

  return <div className="dialog-backdrop" onKeyDown={onKeyDown}>
    <div ref={dialogRef} className="confirmation-dialog" role="alertdialog" aria-modal="true" aria-labelledby={titleId.current}>
      <h2 id={titleId.current}>{options.title}</h2>
      <div className="confirmation-description">{options.description}</div>
      <div className="actions">
        <button ref={cancelRef} type="button" className="secondary" onClick={() => onDecision(false)}>Cancel</button>
        <button type="button" className={options.destructive ? "destructive-button" : undefined} onClick={() => onDecision(true)}>{options.confirmLabel}</button>
      </div>
    </div>
  </div>;
}

export function useConfirmation() {
  const [options, setOptions] = useState<ConfirmationOptions | null>(null);
  const resolver = useRef<((confirmed: boolean) => void) | null>(null);

  const decide = useCallback((confirmed: boolean) => {
    resolver.current?.(confirmed);
    resolver.current = null;
    setOptions(null);
  }, []);

  const confirmAction = useCallback((next: ConfirmationOptions) => new Promise<boolean>((resolve) => {
    resolver.current?.(false);
    resolver.current = resolve;
    setOptions(next);
  }), []);

  useEffect(() => () => resolver.current?.(false), []);

  return { confirmAction, confirmationDialog: options ? <ConfirmationDialog options={options} onDecision={decide} /> : null };
}
