import Navbar from '@/components/Navbar';
import Footer from '@/components/Footer';

export function generateStaticParams() {
  return [{ lang: 'en' }, { lang: 'zh' }];
}

export default function LangLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen flex flex-col">
      <Navbar />
      <main className="flex-1 pt-16">
        {children}
      </main>
      <Footer />
    </div>
  );
}
