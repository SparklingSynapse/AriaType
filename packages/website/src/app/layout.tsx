import './globals.css';
import type { Metadata } from 'next';
import { AnalyticsProvider } from '@/components/AnalyticsProvider';
import { I18nProvider } from '@/components/I18nProvider';

export const metadata: Metadata = {
  title: 'AriaType - Open-Source AI Voice-to-Text Input',
  description:
    'AriaType is an open-source AI voice-to-text input for macOS and a powerful Typeless alternative. Hold a hotkey, speak naturally, and your words appear right where you are already working.',
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
