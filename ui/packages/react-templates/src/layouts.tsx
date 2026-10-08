import * as React from 'react';
import { Table } from 'antd';
import type { TableProps } from 'antd';
import { AppShell } from '@platform/react-shell/src/shell.tsx';
import type { NavigationItem, ShellUser } from '@platform/react-shell/src/shell.tsx';
import { Button, State } from '@platform/react-ui/src/components.tsx';

export interface PublicPageLayoutProps {
  children?: React.ReactNode;
  brand: string;
  title: string;
  navigation?: readonly Pick<NavigationItem, 'id' | 'label' | 'href'>[];
  footer?: React.ReactNode;
}

export function PublicPageLayout({ brand, title, navigation = [], footer, children }: PublicPageLayoutProps) {
  return (
    <div className="platform-page platform-page-public">
      <a className="platform-skip-link" href="#platform-main">Skip to content</a>
      <header className="platform-public-header">
        <a className="platform-brand" href="/" aria-label={`${brand} home`}>{brand}</a>
        <nav aria-label="Main navigation">
          {navigation.map((item) => <a key={item.id} href={item.href}>{item.label}</a>)}
        </nav>
      </header>
      <main id="platform-main" tabIndex={-1}>
        <header className="platform-page-heading"><h1>{title}</h1></header>
        {children}
      </main>
      {footer && <footer className="platform-public-footer">{footer}</footer>}
    </div>
  );
}

export interface DashboardLayoutProps {
  children?: React.ReactNode;
  brand: string;
  title: string;
  user: ShellUser;
  navigation: readonly NavigationItem[];
  currentPath: string;
  onNavigate?: (href: string) => void;
  actions?: React.ReactNode;
}

function DashboardLayout({ brand, title, user, navigation, currentPath, onNavigate, actions, children, variant }: DashboardLayoutProps & { variant: 'user' | 'admin' }) {
  const headingRef = React.useRef<HTMLElement>(null);
  const previousPath = React.useRef(currentPath);
  React.useEffect(() => {
    if (previousPath.current !== currentPath) headingRef.current?.focus();
    previousPath.current = currentPath;
  }, [currentPath]);

  return (
    <div className={`platform-page platform-page-${variant}`} data-platform-layout={variant}>
      <a className="platform-skip-link" href="#main-content">Skip to content</a>
      <header className="platform-dashboard-header">
        <a className="platform-brand" href="/">{brand}</a>
        <div className="platform-dashboard-context"><span>{user.displayName ?? 'Account'}</span>{actions}</div>
      </header>
      <AppShell user={user} navigation={navigation} currentPath={currentPath} onNavigate={onNavigate}>
        <header ref={headingRef} className="platform-page-heading" id="platform-page-heading" tabIndex={-1}>
          <h1>{title}</h1>
        </header>
        <div className="platform-page-content">{children}</div>
      </AppShell>
    </div>
  );
}

export function UserDashboardLayout(props: DashboardLayoutProps) {
  return <DashboardLayout {...props} variant="user" />;
}

export function AdminDashboardLayout(props: DashboardLayoutProps) {
  return <DashboardLayout {...props} variant="admin" />;
}

export interface DataTableProps<Row extends object> extends Omit<TableProps<Row>, 'locale' | 'loading' | 'dataSource'> {
  label: string;
  rows: readonly Row[];
  loading?: boolean;
  forbidden?: string;
  failure?: string;
  emptyMessage?: string;
  onRetry?: () => void;
}

/** Accessible advanced data table; platform primitives remain authoritative for other controls. */
export function DataTable<Row extends object>({ label, rows, loading = false, forbidden, failure, emptyMessage = 'No results.', onRetry, ...tableProps }: DataTableProps<Row>) {
  if (forbidden) {
    return <section className="platform-data-table-state" aria-label={label}><State kind="forbidden">{forbidden}</State></section>;
  }
  if (failure) {
    return (
      <section className="platform-data-table-state" aria-label={label}>
        <State kind="error">{failure}</State>
        {onRetry && <Button type="button" variant="outline" onClick={onRetry}>Try again</Button>}
      </section>
    );
  }
  if (!loading && rows.length === 0) {
    return <div className="platform-data-table-state" aria-label={label}><State kind="empty">{emptyMessage}</State></div>;
  }
  return (
    <div className="platform-data-table-region" role="region" aria-label={label} tabIndex={0} aria-busy={loading || undefined}>
      {loading && <span className="visually-hidden" role="status">Loading {label}.</span>}
      <Table<Row> {...tableProps} dataSource={rows as Row[]} loading={loading} scroll={{ x: 'max-content', ...(tableProps.scroll ?? {}) }} />
    </div>
  );
}
