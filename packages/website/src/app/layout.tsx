import './globals.css';
import type { Metadata } from 'next';
import { AnalyticsProvider } from '@/components/AnalyticsProvider';
import { I18nProvider } from '@/components/I18nProvider';

export const metadata: Metadata = {
  title: 'AriaType — Voice to Text at Your Cursor',
  description: 'Press a hotkey, speak, and your words appear at the cursor. Local-first, privacy-first, no account required, works offline.',
  icons: { icon: '/logo.svg' },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>
        <AnalyticsProvider>
          <I18nProvider>{children}</I18nProvider>
        </AnalyticsProvider>
      </body>
    </html>
  );
}
