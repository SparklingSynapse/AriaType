import './globals.css';
import type { Metadata } from 'next';
import { AnalyticsProvider } from '@/components/AnalyticsProvider';

export const metadata: Metadata = {
  title: 'AriaType - Voice Layer for Your Desktop',
  description:
    'AriaType is the voice layer for your desktop, turning spoken thoughts into context-aware work right where your cursor is.',
  icons: { icon: '/logo.svg' },
  openGraph: {
    title: 'AriaType - Voice Layer for Your Desktop',
    description:
      'Voice-driven writing, input, and cross-app work for your desktop.',
    siteName: 'AriaType',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'AriaType - Voice Layer for Your Desktop',
    description:
      'Voice-driven writing, input, and cross-app work for your desktop.',
  },
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
          {children}
        </AnalyticsProvider>
      </body>
    </html>
  );
}
