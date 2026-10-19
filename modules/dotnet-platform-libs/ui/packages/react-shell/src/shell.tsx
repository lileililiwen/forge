import * as React from 'react';

export type NavigationItem = { id: string; label: string; href: string; permission?: string; admin?: boolean };
export type ShellUser = { authenticated: boolean; displayName?: string; permissions: readonly string[] };
export type ShellProps = React.PropsWithChildren<{ user: ShellUser; navigation: readonly NavigationItem[]; currentPath: string; onNavigate?: (href: string) => void }>;
export function visibleNavigation(user: ShellUser, navigation: readonly NavigationItem[]) { return navigation.filter(item => !item.permission || user.permissions.some(permission => permission.toLowerCase() === item.permission?.toLowerCase())); }
export function AppShell({ user, navigation, currentPath, onNavigate, children }: ShellProps) { const items = visibleNavigation(user, navigation); return <div className="platform-shell"><nav className="platform-nav" aria-label="Application navigation">{items.map(item => <a className="platform-nav-link" key={item.id} href={item.href} aria-current={item.href === currentPath ? 'page' : undefined} onClick={event => { if (onNavigate) { event.preventDefault(); onNavigate(item.href); } }}>{item.label}</a>)}</nav><main id="main-content">{children}</main></div>; }
export function AuthBoundary({ user, children, fallback }: React.PropsWithChildren<{ user: ShellUser; fallback?: React.ReactNode }>) { return user.authenticated ? <>{children}</> : <>{fallback ?? <p role="status">Sign in required.</p>}</>; }
