'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { localizedPath } from '@/lib/routes';

export default function RootPage() {
  const router = useRouter();

  useEffect(() => {
    router.push(localizedPath('en'));
  }, [router]);

  return null;
}
