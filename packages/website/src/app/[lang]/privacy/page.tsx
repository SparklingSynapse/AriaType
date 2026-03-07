'use client';

import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';

export default function PrivacyPage() {
  const { t } = useTranslation();

  return (
    <div className="min-h-screen py-24 px-6">
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <h1 className="text-4xl font-bold mb-8">{t('privacyPage.title')}</h1>
          <div className="prose prose-neutral dark:prose-invert max-w-none">
            <p className="text-lg leading-relaxed">{t('privacyPage.content')}</p>
          </div>
        </motion.div>
      </div>
    </div>
  );
}
