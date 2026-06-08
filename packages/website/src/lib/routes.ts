export type Locale = 'en' | 'zh';

const SUPPORTED_LOCALES = new Set<Locale>(['en', 'zh']);

export function normalizeLocale(lang: string | null | undefined): Locale {
  return SUPPORTED_LOCALES.has(lang as Locale) ? (lang as Locale) : 'en';
}

export function localeFromPathname(pathname: string | null | undefined): Locale {
  const match = pathname?.match(/^\/(en|zh)(?:\/|$)/u);
  return normalizeLocale(match?.[1]);
}

export function localizedPath(lang: string | null | undefined, path = ''): string {
  const locale = normalizeLocale(lang);
  const cleanPath = path.replace(/^\/+|\/+$/gu, '');

  return cleanPath ? `/${locale}/${cleanPath}/` : `/${locale}/`;
}

export function switchLocalePath(pathname: string, nextLang: string): string {
  const withoutLocale = pathname.replace(/^\/(?:en|zh)(?=\/|$)/u, '');
  return localizedPath(nextLang, withoutLocale);
}

export function sameRoute(left: string, right: string): boolean {
  const normalize = (value: string) => value.replace(/\/+$/u, '') || '/';
  return normalize(left) === normalize(right);
}
