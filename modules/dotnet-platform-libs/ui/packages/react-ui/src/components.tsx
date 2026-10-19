import * as React from 'react';

export type ButtonVariant = 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger';
export type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & { loading?: boolean; variant?: ButtonVariant };
export function Button({ loading = false, variant = 'primary', children, disabled, ...props }: ButtonProps) {
  return <button {...props} className={`platform-button platform-button-${variant} ${props.className ?? ''}`} disabled={disabled || loading} aria-busy={loading || undefined} data-loading={loading || undefined}>{loading ? 'Loading…' : children}</button>;
}

export type FieldProps = React.InputHTMLAttributes<HTMLInputElement> & { label: string; error?: string; helper?: string };
export function Field({ id, label, error, helper, ...props }: FieldProps) {
  const fieldId = id ?? React.useId(); const messageId = `${fieldId}-message`;
  return <label htmlFor={fieldId}>{label}<input {...props} id={fieldId} className={`platform-control ${props.className ?? ''}`} aria-invalid={error ? true : undefined} aria-describedby={error || helper ? messageId : undefined} />{(error || helper) && <span id={messageId} aria-live={error ? 'polite' : undefined}>{error ?? helper}</span>}</label>;
}

export function Card({ children, className = '' }: React.PropsWithChildren<{ className?: string }>) { return <section className={`platform-card ${className}`}>{children}</section>; }
export function Badge({ children, tone = 'default' }: React.PropsWithChildren<{ tone?: 'default' | 'success' | 'warning' | 'danger' }>) { return <span className="platform-badge" data-tone={tone}>{children}</span>; }
export type StateKind = 'loading' | 'error' | 'empty' | 'forbidden' | 'not-found';
export function State({ kind, children }: React.PropsWithChildren<{ kind: StateKind }>) { const role = kind === 'error' ? 'alert' : kind === 'loading' ? 'status' : undefined; return <div className="platform-state" data-kind={kind} role={role} aria-live={role ? 'polite' : undefined}>{children}</div>; }
export function Dialog({ open, title, onClose, children }: React.PropsWithChildren<{ open: boolean; title: string; onClose: () => void }>) { return <dialog open={open} className="platform-card" role="dialog" aria-modal="true" aria-labelledby="platform-dialog-title"><h2 id="platform-dialog-title">{title}</h2>{children}<Button type="button" variant="outline" onClick={onClose}>Close</Button></dialog>; }
export function Pagination({ page, pages, onChange }: { page: number; pages: number; onChange: (page: number) => void }) { return <nav aria-label="Pagination"><Button variant="outline" disabled={page <= 1} onClick={() => onChange(page - 1)}>Previous</Button><span aria-current="page">Page {page} of {pages}</span><Button variant="outline" disabled={page >= pages} onClick={() => onChange(page + 1)}>Next</Button></nav>; }
