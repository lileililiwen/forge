import * as React from 'react';
import { ConfigProvider, theme as antdTheme } from 'antd';
import type { ThemeConfig } from 'antd';
import { semanticColorValues, tokenValues } from '@platform/design-tokens/dist/tokens';
import type { PlatformTheme } from '@platform/design-tokens/dist/tokens';

const light = semanticColorValues.light;
const dark = semanticColorValues.dark;

function toPixels(dimension: string): number {
  const value = Number.parseFloat(dimension);
  if (!Number.isFinite(value)) throw new Error(`Unsupported platform dimension: ${dimension}`);
  return dimension.endsWith('rem') ? value * 16 : value;
}

/** Build the platform Ant Design theme from generated semantic token values. */
export function createPlatformAntTheme(mode: PlatformTheme): ThemeConfig {
  const colors = mode === 'dark' ? dark : light;
  return {
    algorithm: mode === 'dark' ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
    token: {
      colorPrimary: colors.primary,
      colorPrimaryHover: colors.primaryHover,
      colorLink: colors.primary,
      colorLinkHover: colors.primaryHover,
      colorBgBase: colors.background,
      colorBgContainer: colors.surface,
      colorBgElevated: colors.surface,
      colorFillAlter: colors.muted,
      colorText: colors.foreground,
      colorTextSecondary: colors.mutedForeground,
      colorTextTertiary: colors.mutedForeground,
      colorTextLightSolid: colors.onPrimary,
      colorPrimaryText: colors.primary,
      colorBorder: colors.border,
      controlOutline: colors.focusRing,
      colorError: colors.danger,
      colorErrorText: colors.danger,
      colorSuccess: colors.success,
      colorSuccessText: colors.success,
      colorWarning: colors.warning,
      colorWarningText: colors.warning,
      fontFamily: tokenValues['--font-sans'],
      fontSize: toPixels(tokenValues['--font-body']),
      borderRadius: toPixels(tokenValues['--component-button-radius']),
      controlHeight: toPixels(tokenValues['--component-control-min-height']),
    },
  };
}

export interface PlatformThemeProviderProps {
  children?: React.ReactNode;
  mode?: PlatformTheme;
}

/** Provides the token-mapped Ant Design theme and matching CSS theme attribute. */
export function PlatformThemeProvider({ mode = 'light', children }: PlatformThemeProviderProps) {
  return (
    <ConfigProvider theme={createPlatformAntTheme(mode)}>
      <div className="platform-react-theme" data-theme={mode}>
        {children}
      </div>
    </ConfigProvider>
  );
}
