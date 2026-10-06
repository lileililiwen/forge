// Theme switching. The dark palette is not written here: [data-theme="dark"] already
// exists in the generated tokens.css, so setting the attribute is the whole mechanism
// and this file can hold no color value at all.

import * as React from 'react';
import { Button } from '@platform/react-ui';
import { groupFooterLinks, orderNavigation } from './branding';
import type { Branding, ThemeName } from './branding';

export const THEME_STORAGE_KEY = 'platform.theme';

export interface ThemeContextValue {
  theme: ThemeName;
  /** True when the resolved theme came from the stored choice rather than the system. */
  explicit: boolean;
  setTheme: (theme: ThemeName) => void;
  clearTheme: () => void;
}

const ThemeContext = React.createContext<ThemeContextValue | null>(null);

export interface ThemeStorage {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
}

function systemTheme(): ThemeName {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function isThemeName(value: unknown): value is ThemeName {
  return value === 'light' || value === 'dark';
}

function readStored(storage: ThemeStorage | undefined): ThemeName | null {
  try {
    const stored = storage?.getItem(THEME_STORAGE_KEY);
    return isThemeName(stored) ? stored : null;
  } catch {
    return null;
  }
}

export interface ThemeProviderProps {
  /** branding.json's `theme` is the default when nothing is stored. */
  defaultTheme?: ThemeName;
  storage?: ThemeStorage;
  children: React.ReactNode;
}

export function ThemeProvider({ defaultTheme, storage, children }: ThemeProviderProps) {
  const fallback = defaultTheme ?? 'light';
  const [explicitTheme, setExplicitTheme] = React.useState<ThemeName | null>(() => readStored(storage));
  const [system, setSystem] = React.useState<ThemeName>(() => systemTheme());

  React.useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    const onChange = () => setSystem(query.matches ? 'dark' : 'light');
    onChange();
    query.addEventListener('change', onChange);
    return () => query.removeEventListener('change', onChange);
  }, []);

  const theme: ThemeName = explicitTheme ?? system ?? fallback;

  React.useEffect(() => {
    if (typeof document !== 'undefined') document.documentElement.dataset.theme = theme;
  }, [theme]);

  const value = React.useMemo<ThemeContextValue>(
    () => ({
      theme,
      explicit: explicitTheme !== null,
      setTheme: (next) => {
        setExplicitTheme(next);
        try {
          storage?.setItem(THEME_STORAGE_KEY, next);
        } catch {
          // A storage failure must not stop the theme from changing for this page.
        }
      },
      clearTheme: () => {
        setExplicitTheme(null);
        try {
          storage?.removeItem(THEME_STORAGE_KEY);
        } catch {
          // Same as above: the resolved theme falls back to the system preference.
        }
      },
    }),
    [theme, explicitTheme, storage],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const value = React.useContext(ThemeContext);
  if (!value) throw new Error('useTheme must be used inside a ThemeProvider');
  return value;
}

export interface ThemeToggleProps {
  branding?: Branding;
}

export function ThemeToggle({ branding }: ThemeToggleProps) {
  const { theme, setTheme } = useTheme();
  const label = branding?.name ? `${branding.name} theme` : 'Theme';
  return (
    // aria-pressed reports the current selection, and the name is text, so the state is
    // never carried by color alone.
    <Button type="button" variant="ghost" aria-pressed={theme === 'dark'} aria-label={`${label}: ${theme}`} onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>
      {theme === 'dark' ? 'Dark' : 'Light'}
    </Button>
  );
}

/** The skip link is the first focusable element and moves focus to #main-content. */
export function SkipToContent() {
  return (
    <a className="visually-hidden platform-auth-skip" href="#main-content">
      Skip to content
    </a>
  );
}

export interface BrandingChromeProps {
  branding: Branding;
  /** Rendered as the accessible name of the logo, and used as the alt text fallback. */
  logoAlt?: string;
}

/** Logo and navigation, read from branding.json so a project changes data, not code. */
export function BrandHeader({ branding, logoAlt }: BrandingChromeProps) {
  const alt = logoAlt ?? branding.name;
  return (
    <header className="platform-auth-header">
      <img className="platform-auth-logo" src={branding.logo.src} alt={alt} />
      <nav className="platform-auth-nav" aria-label="Primary">
        {orderNavigation(branding.navigation).map((item) => (
          <a key={item.id} className="platform-auth-footer-link" href={item.href} data-icon={item.icon}>
            {item.label}
          </a>
        ))}
      </nav>
    </header>
  );
}

/** Footer links grouped by `group`, the 友情链接 surface, from branding.json. */
export function BrandFooter({ branding }: BrandingChromeProps) {
  return (
    <footer className="platform-auth-footer">
      {groupFooterLinks(branding.footerLinks).map((group) => (
        <div key={group.group} className="platform-auth-footer-group">
          <h2 className="platform-auth-footer-heading">{group.group}</h2>
          {group.links.map((link) => (
            <a key={link.id} className="platform-auth-footer-link" href={link.href}>
              {link.label}
            </a>
          ))}
        </div>
      ))}
    </footer>
  );
}
