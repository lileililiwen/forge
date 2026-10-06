// Typed loader for branding.json. The document is validated here as well as by
// ui/schemas/branding.schema.json, because a component that receives an unvalidated
// document would render a blank header instead of reporting the bad field. Nothing here
// touches the filesystem or a Node API: the host loads branding.json and passes it in,
// which is what "consumed at render time" means for a React component.

export type ThemeName = 'light' | 'dark';

export interface BrandingNavigationItem {
  id: string;
  label: string;
  href: string;
  order: number;
  icon?: string;
}

export interface BrandingFooterLink {
  id: string;
  label: string;
  href: string;
  group: string;
}

export interface Branding {
  name: string;
  logo: { kind: 'path'; src: string } | { kind: 'data'; src: string };
  theme: ThemeName;
  navigation: BrandingNavigationItem[];
  footerLinks: BrandingFooterLink[];
  locale: string;
}

export class BrandingError extends Error {
  constructor(path: string, detail: string) {
    super(`branding.json ${path}: ${detail}`);
    this.name = 'BrandingError';
  }
}

function requireString(value: unknown, path: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new BrandingError(path, 'is required and must be a non-empty string');
  }
  return value;
}

function requireArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) throw new BrandingError(path, 'is required and must be an array');
  return value;
}

function parseLogo(value: unknown): Branding['logo'] {
  const path = 'logo';
  if (typeof value !== 'object' || value === null) {
    throw new BrandingError(path, 'is required and must be a path string or an inline data URI');
  }
  const record = value as Record<string, unknown>;
  const src = requireString(record.src, `${path}.src`);
  const kind = record.kind ?? (src.startsWith('data:') ? 'data' : 'path');
  if (kind !== 'path' && kind !== 'data') {
    throw new BrandingError(`${path}.kind`, `must be "path" or "data", received ${JSON.stringify(kind)}`);
  }
  if (kind === 'data' && !src.startsWith('data:')) {
    throw new BrandingError(`${path}.src`, 'is declared as inline data but is not a data URI');
  }
  if (kind === 'path' && src.startsWith('data:')) {
    throw new BrandingError(`${path}.kind`, 'is declared as a path but the source is an inline data URI');
  }
  return kind === 'data' ? { kind: 'data', src } : { kind: 'path', src };
}

function parseNavigation(value: unknown): BrandingNavigationItem[] {
  return requireArray(value, 'navigation').map((entry, index) => {
    const path = `navigation[${index}]`;
    if (typeof entry !== 'object' || entry === null) throw new BrandingError(path, 'must be an object');
    const record = entry as Record<string, unknown>;
    const order = record.order;
    if (typeof order !== 'number' || !Number.isFinite(order)) {
      throw new BrandingError(`${path}.order`, 'is required and must be a number');
    }
    const item: BrandingNavigationItem = {
      id: requireString(record.id, `${path}.id`),
      label: requireString(record.label, `${path}.label`),
      href: requireString(record.href, `${path}.href`),
      order,
    };
    if (record.icon !== undefined) item.icon = requireString(record.icon, `${path}.icon`);
    return item;
  });
}

function parseFooterLinks(value: unknown): BrandingFooterLink[] {
  return requireArray(value, 'footerLinks').map((entry, index) => {
    const path = `footerLinks[${index}]`;
    if (typeof entry !== 'object' || entry === null) throw new BrandingError(path, 'must be an object');
    const record = entry as Record<string, unknown>;
    return {
      id: requireString(record.id, `${path}.id`),
      label: requireString(record.label, `${path}.label`),
      href: requireString(record.href, `${path}.href`),
      group: requireString(record.group, `${path}.group`),
    };
  });
}

/** Navigation in the order the project declared, by the numeric `order` field. */
export function orderNavigation(items: BrandingNavigationItem[]): BrandingNavigationItem[] {
  return [...items].sort((left, right) => left.order - right.order);
}

/** Footer links grouped by `group`, groups in first-seen order. This is the 友情链接 surface. */
export function groupFooterLinks(links: BrandingFooterLink[]): { group: string; links: BrandingFooterLink[] }[] {
  const groups = new Map<string, BrandingFooterLink[]>();
  for (const link of links) {
    const existing = groups.get(link.group);
    if (existing) existing.push(link);
    else groups.set(link.group, [link]);
  }
  return [...groups].map(([group, grouped]) => ({ group, links: grouped }));
}

/** Validate a parsed branding.json document. Throws BrandingError naming the bad field. */
export function parseBranding(document: unknown): Branding {
  if (typeof document !== 'object' || document === null || Array.isArray(document)) {
    throw new BrandingError('branding.json', 'must be a JSON object');
  }
  const record = document as Record<string, unknown>;
  const theme = record.theme ?? 'light';
  if (theme !== 'light' && theme !== 'dark') {
    throw new BrandingError('theme', `must be "light" or "dark", received ${JSON.stringify(theme)}`);
  }
  return {
    name: requireString(record.name, 'name'),
    logo: parseLogo(record.logo),
    theme,
    navigation: parseNavigation(record.navigation),
    footerLinks: parseFooterLinks(record.footerLinks),
    locale: requireString(record.locale, 'locale'),
  };
}
