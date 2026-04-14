'use client';

import { useTranslation } from 'react-i18next';
import Link from 'next/link';
import Image from 'next/image';
import { usePathname } from 'next/navigation';
import { Globe, ChevronDown } from 'lucide-react';
import { useState, useEffect, useRef } from 'react';
import { useAnalytics } from '@/lib/analytics';
import { AnalyticsEvents } from '@/lib/events';

export default function Navbar() {
  const { trackEvent } = useAnalytics();
  const { t } = useTranslation();
  const pathname = usePathname();
  const [isLangOpen, setIsLangOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);
  const langRef = useRef<HTMLDivElement>(null);

  const currentLang = pathname.startsWith('/zh') ? 'zh' : 'en';
  const navItems = [
    { href: `/${currentLang}`, label: t('nav.home') },
    { href: `/${currentLang}/download`, label: t('nav.download') },
  ];

  useEffect(() => {
    function handleScroll() {
      setScrolled(window.scrollY > 20);
    }
    window.addEventListener('scroll', handleScroll, { passive: true });
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

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
    <nav
      className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
        scrolled
          ? 'bg-background/90 backdrop-blur-sm border-b border-border/50'
          : 'bg-transparent'
      }`}
    >
        <div className="max-w-5xl mx-auto px-6 h-14 flex items-center justify-between">
          <Link
            href={`/${currentLang}`}
            className="flex items-center gap-2.5 group"
          >
            <Image
              src="/logo.svg"
              alt="AriaType"
              width={28}
              height={28}
              className="rounded-md group-hover:opacity-80 transition-opacity"
            />
            <span className="font-medium text-sm tracking-tight">{t('app.name')}</span>
          </Link>

          <div className="hidden md:flex items-center gap-8">
            {navItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                onClick={() => trackEvent(AnalyticsEvents.NAV_CLICK, { label: item.label, href: item.href })}
                className={`nav-link text-sm tracking-wide ${
                  pathname === item.href
                    ? 'active text-foreground'
                    : 'text-foreground/70 hover:text-foreground'
                }`}
              >
                {item.label}
              </Link>
            ))}
          </div>

          <div className="flex items-center gap-4">
            <Link
              href="https://github.com/joe223/AriaType"
              target="_blank"
              rel="noopener noreferrer"
              onClick={() => trackEvent(AnalyticsEvents.NAV_CLICK, { label: 'GitHub', href: 'https://github.com/joe223/AriaType' })}
              className="text-foreground/60 hover:text-foreground transition-colors"
              aria-label="GitHub"
            >
              <svg className="w-4.5 h-4.5" viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
              </svg>
            </Link>

            <div className="relative" ref={langRef}>
              <button
                onClick={() => setIsLangOpen(!isLangOpen)}
                className="flex items-center gap-1.5 rounded-md px-2 py-1 text-sm text-foreground/60 hover:bg-secondary hover:text-foreground transition-colors"
              >
                <Globe className="w-3.5 h-3.5" />
                <span className="uppercase text-xs tracking-wider">{currentLang}</span>
                <ChevronDown className={`w-3 h-3 transition-transform duration-200 ${isLangOpen ? 'rotate-180' : ''}`} />
              </button>

              <div
                className={`absolute right-0 mt-1.5 w-32 overflow-hidden rounded-2xl border border-border bg-card shadow-lg transition-all duration-200 origin-top-right ${
                  isLangOpen
                    ? 'scale-100 opacity-100'
                    : 'pointer-events-none scale-95 opacity-0'
                }`}
              >
                <div className="p-1">
                  <button
                    onClick={() => switchLanguage('en')}
                    className={`w-full rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                      currentLang === 'en'
                        ? 'bg-background font-medium text-foreground'
                        : 'text-muted-foreground hover:bg-background-hover hover:text-foreground'
                    }`}
                  >
                    English
                  </button>
                  <button
                    onClick={() => switchLanguage('zh')}
                    className={`w-full rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                      currentLang === 'zh'
                        ? 'bg-background font-medium text-foreground'
                        : 'text-muted-foreground hover:bg-background-hover hover:text-foreground'
                    }`}
                  >
                    中文
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
    </nav>
  );
}
