'use client';

import { useEffect } from 'react';
import { I18nextProvider } from 'react-i18next';
import i18n from '@/i18n';

export function I18nProvider({
  children,
  lang,
}: {
  children: React.ReactNode;
  lang: string;
}) {
  if (i18n.language !== lang) {
    void i18n.changeLanguage(lang);
  }

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
}
