'use client';

import { useTranslation } from 'react-i18next';
import { Download, Check, Loader2, AlertCircle } from 'lucide-react';
import { motion } from 'framer-motion';
import { useDownload } from '@/hooks/useDownload';

function AppleIcon({ className }: { className?: string }) {
  return (
    <svg 
      xmlns="http://www.w3.org/2000/svg" 
      viewBox="0 0 814 1000" 
      className={className}
      width="28"
      height="28"
    >
      <path d="M788.1 340.9c-5.8 4.5-108.2 62.2-108.2 190.5 0 148.4 130.3 200.9 134.2 202.2-.6 3.2-20.7 71.9-68.7 141.9-42.8 61.6-87.5 123.1-155.5 123.1s-85.5-39.5-164-39.5c-76.5 0-103.7 40.8-165.9 40.8s-105.6-57-155.5-127C46.7 790.7 0 663 0 541.8c0-194.4 126.4-297.5 250.8-297.5 66.1 0 121.2 43.4 162.7 43.4 39.5 0 101.1-46 176.3-46 28.5 0 130.9 2.6 198.3 99.2zm-234-181.5c31.1-36.9 53.1-88.1 53.1-139.3 0-7.1-.6-14.3-1.9-20.1-50.6 1.9-110.8 33.7-147.1 75.8-28.5 32.4-55.1 83.6-55.1 135.5 0 7.8 1.3 15.6 1.9 18.1 3.2.6 8.4 1.3 13.6 1.3 45.4 0 102.5-30.4 135.5-71.3z"/>
    </svg>
  );
}

function MicrosoftIcon({ className }: { className?: string }) {
  return (
    <svg 
      xmlns="http://www.w3.org/2000/svg" 
      viewBox="0 0 88 88" 
      className={className}
      width="28"
      height="28"
    >
      <rect fill="#F25022" width="42" height="42"/>
      <rect x="46" fill="#7FBA00" width="42" height="42"/>
      <rect y="46" fill="#00A4EF" width="42" height="42"/>
      <rect x="46" y="46" fill="#FFB900" width="42" height="42"/>
    </svg>
  );
}

export default function DownloadClient() {
  const { t } = useTranslation();
  const { release, loading, unavailable, platform, defaultMacUrl, trackDownload } = useDownload('download_page');
  const macUniversalUrl = release?.platforms?.mac?.universal || '';
  const macArmUrl = release?.platforms?.mac?.aarch64 || '';
  const macIntelUrl = release?.platforms?.mac?.x86_64 || '';
  const macOptions = [
    { label: t('download.universal'), url: macUniversalUrl },
    { label: t('download.macArm'), url: macArmUrl },
    { label: t('download.macIntel'), url: macIntelUrl },
  ].filter((item) => item.url);
  const windowsExeUrl = release?.platforms?.windows?.exe || '';
  const windowsMsiUrl = release?.platforms?.windows?.msi || '';
  const windowsOptions = [
    { label: 'Windows (.exe)', url: windowsExeUrl },
    { label: 'Windows (.msi)', url: windowsMsiUrl },
  ].filter((item) => item.url);

  return (
    <div className="min-h-screen py-24 px-6">
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-center mb-16"
        >
          <h1 className="text-[clamp(2.5rem,5vw,4.5rem)] font-bold tracking-tight leading-[1.05] mb-4">{t('download.title')}</h1>
          <p className="text-xl text-muted-foreground">{t('download.subtitle')}</p>
        </motion.div>

        {loading && (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
          </div>
        )}

        {!loading && unavailable && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="flex flex-col items-center justify-center py-12 gap-3 text-muted-foreground"
          >
            <AlertCircle className="w-8 h-8" />
            <p>{t('download.noRelease')}</p>
          </motion.div>
        )}

        {!loading && release && (
          <div className="space-y-8">
            <div className="text-center">
              <span className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-secondary text-sm">
                <Check className="w-4 h-4 text-green-500" />
                {t('download.currentVersion')}: v{release.version}
              </span>
            </div>

            <div className="grid md:grid-cols-2 gap-6">
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.1 }}
                className="p-8 rounded-[1.5rem] bg-card border border-border"
              >
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-2xl bg-primary/10 flex items-center justify-center">
                    <AppleIcon className="text-foreground" />
                  </div>
                  <div>
                    <h3 className="font-semibold">{t('download.macos')}</h3>
                    <p className="text-sm text-muted-foreground">{t('download.requirementsMac')}</p>
                  </div>
                </div>
                <div className="space-y-2">
                  {macOptions.map((option) => (
                    <a
                      key={option.url}
                      href={option.url}
                      onClick={() => trackDownload(option.url)}
                      className={`flex items-center justify-between space-x-4 p-3 rounded-2xl border border-border transition-colors ${
                        option.url === defaultMacUrl
                          ? 'border-primary bg-primary/5'
                          : platform === 'mac'
                            ? 'border-border hover:bg-secondary'
                            : 'border-border hover:bg-secondary'
                      }`}
                    >
                      <span className="text-sm font-medium">{option.label}</span>
                      <Download className="w-4 h-4" />
                    </a>
                  ))}
                  {macOptions.length === 0 && (
                    <a
                      href={release.url}
                      onClick={() => trackDownload(release.url)}
                      className={`flex items-center justify-between space-x-4 p-3 rounded-2xl border border-border transition-colors ${
                        platform === 'mac' ? 'border-primary bg-primary/5' : 'border-border hover:bg-secondary'
                      }`}
                    >
                      <span className="text-sm font-medium">{t('download.macos')}</span>
                      <Download className="w-4 h-4" />
                    </a>
                  )}
                </div>
              </motion.div>

              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.2 }}
                className="p-8 rounded-[1.5rem] bg-card border border-border"
              >
                <div className="flex items-center gap-3 mb-4">
                  <div className="w-10 h-10 rounded-2xl bg-primary/10 flex items-center justify-center">
                    <MicrosoftIcon className="text-foreground" />
                  </div>
                  <div>
                    <h3 className="font-semibold">{t('download.windows')}</h3>
                    <p className="text-sm text-muted-foreground">{t('download.requirementsWin')}</p>
                  </div>
                </div>
                {windowsOptions.length > 0 ? (
                  <div className="space-y-2">
                    {windowsOptions.map((option) => (
                      <a
                        key={option.url}
                        href={option.url}
                        onClick={() => trackDownload(option.url)}
                        className="flex items-center justify-between space-x-4 p-3 rounded-2xl border border-border transition-colors hover:bg-secondary"
                      >
                        <span className="text-sm font-medium">{option.label}</span>
                        <Download className="w-4 h-4" />
                      </a>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground py-2">{t('download.comingSoon')}</p>
                )}
              </motion.div>
            </div>

            {release.notes && (
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.3 }}
                className="p-8 rounded-[1.5rem] bg-secondary/30"
              >
                <h3 className="font-semibold mb-3">{t('download.releaseNotes')}</h3>
                <pre className="text-sm text-muted-foreground whitespace-pre-wrap font-mono max-h-60 overflow-y-auto">
                  {release.notes}
                </pre>
              </motion.div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
