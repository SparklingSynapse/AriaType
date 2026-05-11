import type { Metadata } from 'next';
import Navbar from '@/components/Navbar';
import Footer from '@/components/Footer';
import { I18nProvider } from '@/components/I18nProvider';

type LangParams = { lang: string };

export function generateStaticParams() {
  return [{ lang: 'en' }, { lang: 'zh' }];
}

const metadataByLang = {
  en: {
    title: 'AriaType - Voice Layer for Your Desktop',
    description:
      'AriaType is the voice layer for your desktop, turning spoken thoughts into context-aware work right where your cursor is.',
  },
  zh: {
    title: 'AriaType - 桌面语音工作层',
    description:
      'AriaType 是桌面上的语音工作层，把你说出口的想法变成贴合上下文的内容，直接落到当前光标位置。',
  },
} as const;

export async function generateMetadata({
  params,
}: {
  params: Promise<LangParams>;
}): Promise<Metadata> {
  const { lang } = await params;
  const metadata =
    lang === 'zh' ? metadataByLang.zh : metadataByLang.en;

  return {
    title: metadata.title,
    description: metadata.description,
    openGraph: {
      title: metadata.title,
      description: metadata.description,
      siteName: 'AriaType',
      type: 'website',
    },
    twitter: {
      card: 'summary_large_image',
      title: metadata.title,
      description: metadata.description,
    },
  };
}

export default async function LangLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<LangParams>;
}) {
  const { lang } = await params;

  return (
    <I18nProvider lang={lang}>
      <div className="min-h-screen flex flex-col">
        <Navbar />
        <main className="flex-1 pt-16">
          {children}
        </main>
        <Footer />
      </div>
    </I18nProvider>
  );
}
