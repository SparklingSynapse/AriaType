'use client';

import { useTranslation } from 'react-i18next';
import { Mic, Keyboard, FileText, Lock, Sparkles, Globe, ArrowRight } from 'lucide-react';
import { motion } from 'framer-motion';
import Link from 'next/link';
import { useParams } from 'next/navigation';
import { useAnalytics } from '@/lib/analytics';
import { AnalyticsEvents } from '@/lib/events';

export default function FeaturesPage() {
  const { trackEvent } = useAnalytics();
  const { t } = useTranslation();
  const params = useParams();
  const lang = (params?.lang as string) || 'en';
  const steps = [
    { title: t('features.step0Title'), description: t('features.step0Description') },
    { title: t('features.step1Title'), description: t('features.step1Description') },
    { title: t('features.step2Title'), description: t('features.step2Description') },
  ];

  return (
    <div className="min-h-screen">
      {/* Feature cards */}
      <section className="py-24 px-6">
        <div className="max-w-6xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="text-center mb-16"
          >
            <h1 className="text-4xl md:text-5xl font-bold mb-4">{t('features.title')}</h1>
            <p className="text-xl text-muted-foreground max-w-2xl mx-auto">{t('features.subtitle')}</p>
          </motion.div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0 * 0.08 }}
              className="p-6 rounded-xl bg-card border border-border hover:border-border/80 hover:shadow-md transition-all duration-200"
            >
              <div className="w-11 h-11 rounded-lg flex items-center justify-center mb-4 bg-blue-500/10">
                <Lock className="w-5 h-5 text-blue-500" />
              </div>
              <h3 className="text-base font-semibold mb-1.5">{t('features.privacyTitle')}</h3>
              <p className="text-muted-foreground text-sm leading-relaxed">{t('features.privacyDescription')}</p>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 1 * 0.08 }}
              className="p-6 rounded-xl bg-card border border-border hover:border-border/80 hover:shadow-md transition-all duration-200"
            >
              <div className="w-11 h-11 rounded-lg flex items-center justify-center mb-4 bg-rose-500/10">
                <Mic className="w-5 h-5 text-rose-500" />
              </div>
              <h3 className="text-base font-semibold mb-1.5">{t('features.voiceTitle')}</h3>
              <p className="text-muted-foreground text-sm leading-relaxed">{t('features.voiceDescription')}</p>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 2 * 0.08 }}
              className="p-6 rounded-xl bg-card border border-border hover:border-border/80 hover:shadow-md transition-all duration-200"
            >
              <div className="w-11 h-11 rounded-lg flex items-center justify-center mb-4 bg-violet-500/10">
                <Keyboard className="w-5 h-5 text-violet-500" />
              </div>
              <h3 className="text-base font-semibold mb-1.5">{t('features.hotkeyTitle')}</h3>
              <p className="text-muted-foreground text-sm leading-relaxed">{t('features.hotkeyDescription')}</p>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 3 * 0.08 }}
              className="p-6 rounded-xl bg-card border border-border hover:border-border/80 hover:shadow-md transition-all duration-200"
            >
              <div className="w-11 h-11 rounded-lg flex items-center justify-center mb-4 bg-emerald-500/10">
                <FileText className="w-5 h-5 text-emerald-500" />
              </div>
              <h3 className="text-base font-semibold mb-1.5">{t('features.insertTitle')}</h3>
              <p className="text-muted-foreground text-sm leading-relaxed">{t('features.insertDescription')}</p>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 4 * 0.08 }}
              className="p-6 rounded-xl bg-card border border-border hover:border-border/80 hover:shadow-md transition-all duration-200"
            >
              <div className="w-11 h-11 rounded-lg flex items-center justify-center mb-4 bg-amber-500/10">
                <Sparkles className="w-5 h-5 text-amber-500" />
              </div>
              <h3 className="text-base font-semibold mb-1.5">{t('features.polishTitle')}</h3>
              <p className="text-muted-foreground text-sm leading-relaxed">{t('features.polishDescription')}</p>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 5 * 0.08 }}
              className="p-6 rounded-xl bg-card border border-border hover:border-border/80 hover:shadow-md transition-all duration-200"
            >
              <div className="w-11 h-11 rounded-lg flex items-center justify-center mb-4 bg-cyan-500/10">
                <Globe className="w-5 h-5 text-cyan-500" />
              </div>
              <h3 className="text-base font-semibold mb-1.5">{t('features.multilangTitle')}</h3>
              <p className="text-muted-foreground text-sm leading-relaxed">{t('features.multilangDescription')}</p>
            </motion.div>
          </div>
        </div>
      </section>

      {/* How It Works */}
      <section className="py-24 px-6 bg-secondary/30">
        <div className="max-w-4xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="text-center mb-14"
          >
            <h2 className="text-3xl font-bold mb-3">{t('features.howItWorksTitle')}</h2>
            <p className="text-muted-foreground text-lg">{t('features.howItWorksSubtitle')}</p>
          </motion.div>

          <div className="flex flex-col gap-6">
            {steps.map((step, index) => (
              <motion.div
                key={index}
                initial={{ opacity: 0, x: -20 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.1 }}
                className="flex items-start gap-6 p-6 rounded-xl bg-card border border-border"
              >
                <div className="w-10 h-10 rounded-full bg-primary text-primary-foreground flex items-center justify-center font-bold text-sm shrink-0">
                  {index + 1}
                </div>
                <div>
                  <h3 className="font-semibold mb-1">{step.title}</h3>
                  <p className="text-muted-foreground text-sm leading-relaxed">{step.description}</p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-24 px-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="max-w-2xl mx-auto text-center"
        >
          <h2 className="text-3xl font-bold mb-3">{t('features.ctaTitle')}</h2>
          <p className="text-muted-foreground mb-8">{t('features.ctaSubtitle')}</p>
          <Link
            href={`/${lang}/download`}
            onClick={() => trackEvent(AnalyticsEvents.CTA_CLICK, { location: 'features', label: 'download' })}
            className="inline-flex items-center gap-2 px-8 py-4 rounded-lg bg-primary text-primary-foreground font-medium hover:bg-primary/90 transition-colors"
          >
            {t('features.ctaButton')}
            <ArrowRight className="w-4 h-4" />
          </Link>
        </motion.div>
      </section>
    </div>
  );
}
