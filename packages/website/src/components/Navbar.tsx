'use client';

import { useTranslation } from 'react-i18next';
import Link from 'next/link';
import Image from 'next/image';
import { usePathname } from 'next/navigation';
import { Globe, ChevronDown, Github } from 'lucide-react';
import { useState, useEffect, useRef } from 'react';
import { useAnalytics } from '@/lib/analytics';
import { AnalyticsEvents } from '@/lib/events';

export default function Navbar() {
  const { trackEvent } = useAnalytics();
  const { t, i18n } = useTranslation();
  const pathname = usePathname();
  const [isLangOpen, setIsLangOpen] = useState(false);
  const langRef = useRef<HTMLDivElement>(null);

  const currentLang = pathname.startsWith('/zh') ? 'zh' : 'en';
  const navItems = [
    { href: `/${currentLang}`, label: t('nav.home') },
    { href: `/${currentLang}/download`, label: t('nav.download') },
  ];

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (langRef.current && !langRef.current.contains(event.target as Node)) {
        setIsLangOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const switchLanguage = (lang: string) => {
    trackEvent(AnalyticsEvents.LANGUAGE_SWITCH, { from: currentLang, to: lang });
    const newPath = pathname.replace(/^\/(en|zh)/, `/${lang}`);
    window.location.href = newPath === pathname ? `/${lang}` : newPath;
    setIsLangOpen(false);
  };

  return (
    <nav className="fixed top-0 left-0 right-0 z-50 border-b border-border bg-background/80 backdrop-blur-md">
      <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between space-x-4">
        <Link href={`/${currentLang}`} className="flex items-center gap-2">
          <Image src="/logo.svg" alt="AriaType" width={32} height={32} className="rounded-lg" />
          <span className="font-semibold text-lg">{t('app.name')}</span>
        </Link>

        <div className="flex items-center gap-8">
          <div className="hidden md:flex items-center gap-6">
            {navItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                onClick={() => trackEvent(AnalyticsEvents.NAV_CLICK, { label: item.label, href: item.href })}
                className={`text-base transition-colors hover:text-foreground ${
                  pathname === item.href
                    ? 'text-foreground font-semibold'
                    : 'text-foreground/80'
                }`}
              >
                {item.label}
              </Link>
            ))}
          </div>

          <Link
            href="https://github.com/SparklingSynapse/aria-type"
            target="_blank"
            rel="noopener noreferrer"
            onClick={() => trackEvent(AnalyticsEvents.NAV_CLICK, { label: 'GitHub', href: 'https://github.com/SparklingSynapse/aria-type' })}
            className="text-foreground/80 hover:text-foreground transition-colors"
          >
            <Github className="w-5 h-5" />
          </Link>

          <div className="relative" ref={langRef}>
            <button
              onClick={() => setIsLangOpen(!isLangOpen)}
              className="flex items-center gap-1.5 text-base text-foreground/80 hover:text-foreground transition-colors"
            >
              <Globe className="w-4 h-4" />
              <span className="uppercase">{currentLang}</span>
              <ChevronDown className={`w-3 h-3 transition-transform ${isLangOpen ? 'rotate-180' : ''}`} />
            </button>

            {isLangOpen && (
              <div className="absolute right-0 mt-2 w-24 rounded-lg border border-border bg-card overflow-hidden">
                <button
                  onClick={() => switchLanguage('en')}
                  className={`w-full px-4 py-2 text-left text-sm hover:bg-card-hover transition-colors ${
                    currentLang === 'en' ? 'bg-secondary' : ''
                  }`}
                >
                  English
                </button>
                <button
                  onClick={() => switchLanguage('zh')}
                  className={`w-full px-4 py-2 text-left text-sm hover:bg-card-hover transition-colors ${
                    currentLang === 'zh' ? 'bg-secondary' : ''
                  }`}
                >
                  中文
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </nav>
  );
}
