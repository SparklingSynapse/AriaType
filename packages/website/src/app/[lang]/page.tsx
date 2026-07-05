"use client";

import Link from "next/link";
import { useTranslation } from "react-i18next";
import { useParams } from "next/navigation";
import { motion } from "framer-motion";
import { Bold, FileText, Globe, Italic, Keyboard, Lock, Mail, Mic, Paperclip, SendHorizontal, Sparkles, Underline } from "lucide-react";
import { HomeDownloadButton } from "@/components/HomeDownloadButton";

const reveal = {
  hidden: { opacity: 0, y: 16 },
  visible: { opacity: 1, y: 0 },
};

const transition = {
  duration: 0.6,
  ease: [0.16, 1, 0.3, 1],
};

function SectionLabel({ children }: { children: string }) {
  return (
    <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
      {children}
    </p>
  );
}

function ContextVisual({ t }: { t: (key: string) => string }) {
  return (
    <div className="relative min-h-[460px] min-w-0 overflow-hidden rounded-3xl border border-border bg-card shadow-sm md:min-h-0" style={{ aspectRatio: "4 / 3" }}>
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_22%_18%,rgba(96,165,250,0.18),transparent_34%),radial-gradient(circle_at_82%_8%,rgba(192,132,252,0.14),transparent_32%),linear-gradient(135deg,rgba(255,255,255,0.85),rgba(231,229,228,0.32))]" />
      <div className="pointer-events-none absolute inset-x-10 top-0 h-px bg-gradient-to-r from-transparent via-foreground/10 to-transparent" />
      <div className="relative flex h-full flex-col p-6 md:p-8">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5">
              <span className="h-2.5 w-2.5 rounded-full bg-red-400/85 ring-1 ring-inset ring-black/5" />
              <span className="h-2.5 w-2.5 rounded-full bg-amber-400/85 ring-1 ring-inset ring-black/5" />
              <span className="h-2.5 w-2.5 rounded-full bg-green-400/85 ring-1 ring-inset ring-black/5" />
            </div>
            <div className="flex items-center gap-1.5 rounded-full border border-border/60 bg-background/70 px-2 py-0.5 text-[11px] font-medium text-muted-foreground backdrop-blur">
              <Mail className="h-3 w-3" />
              <span className="text-foreground/80">{t("homePage.visual.windowTitle")}</span>
            </div>
          </div>
          <div className="flex items-center gap-1.5 rounded-full border border-border bg-background/80 px-3 py-1 text-xs text-muted-foreground shadow-sm backdrop-blur">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.6)]" />
            {t("homePage.visual.contextBadge")}
          </div>
        </div>

        <div className="mt-6 overflow-hidden rounded-2xl border border-border bg-background/90 shadow-sm backdrop-blur">
          <div className="flex items-center justify-between border-b border-border/60 bg-background/60 px-4 py-2.5">
            <div className="flex items-center gap-2 text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
              <FileText className="h-3.5 w-3.5" />
              {t("homePage.visual.activeField")}
            </div>
            <div className="flex items-center gap-1 text-[11px] text-muted-foreground">
              <span className="h-1 w-1 rounded-full bg-emerald-500 shadow-[0_0_4px_rgba(16,185,129,0.7)]" />
              {t("homePage.visual.metaTimestamp")}
            </div>
          </div>
          <div className="p-4">
            <div className="flex items-center gap-2">
              <span className="flex h-6 w-6 items-center justify-center rounded-full bg-gradient-to-br from-blue-500 to-indigo-500 text-[10px] font-semibold text-white shadow-sm">
                M
              </span>
              <span className="text-sm font-medium text-foreground">
                {t("homePage.visual.metaRecipient")}
              </span>
            </div>
            <p className="mt-3 text-sm leading-6 text-muted-foreground">
              {t("homePage.visual.roughSpeech")}
            </p>
            <div className="mt-3 rounded-2xl border border-border/70 bg-card shadow-sm">
              <p className="px-4 py-3 text-sm leading-6 text-foreground">
                {t("homePage.visual.contextOutput")}
                <span className="ml-0.5 inline-block h-4 w-px animate-pulse bg-foreground/80 align-middle" />
              </p>
              <div className="flex items-center justify-between border-t border-border/60 px-3 py-1.5">
                <div className="flex items-center gap-1 text-muted-foreground">
                  <Bold className="h-3 w-3" />
                  <Italic className="h-3 w-3" />
                  <Underline className="h-3 w-3" />
                  <span className="mx-1 h-3 w-px bg-border" />
                  <Paperclip className="h-3 w-3" />
                </div>
                <div className="flex items-center gap-1 rounded-full bg-foreground/90 px-2 py-0.5 text-[10px] font-medium text-background">
                  <SendHorizontal className="h-2.5 w-2.5" />
                  <span>{t("homePage.visual.shortcutHint")}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="mt-auto grid grid-cols-3 gap-2 text-xs">
          <div className="rounded-2xl border border-border/70 bg-background/80 p-2.5 backdrop-blur">
            <div className="flex items-center justify-between text-muted-foreground">
              <span>{t("homePage.visual.appLabel")}</span>
              <span className="flex items-center gap-1 rounded-full bg-emerald-500/12 px-1.5 py-px text-[9px] font-medium text-emerald-600 dark:text-emerald-400">
                <span className="h-1 w-1 rounded-full bg-emerald-500" />
                {t("homePage.visual.detectedTag")}
              </span>
            </div>
            <div className="mt-1 font-medium text-foreground">{t("homePage.visual.appValue")}</div>
          </div>
          <div className="rounded-2xl border border-border/70 bg-background/80 p-2.5 backdrop-blur">
            <div className="text-muted-foreground">{t("homePage.visual.fieldLabel")}</div>
            <div className="mt-1 font-medium text-foreground">{t("homePage.visual.fieldValue")}</div>
          </div>
          <div className="rounded-2xl border border-border/70 bg-background/80 p-2.5 backdrop-blur">
            <div className="text-muted-foreground">{t("homePage.visual.toneLabel")}</div>
            <div className="mt-1 font-medium text-foreground">{t("homePage.visual.toneValue")}</div>
          </div>
        </div>
      </div>
    </div>
  );
}

function LayerVisual({ t }: { t: (key: string) => string }) {
  const items = [
    {
      icon: Mic,
      title: t("homePage.visual.noiseTitle"),
      detail: t("homePage.visual.noiseDetail"),
      accent: "bg-blue-500/12 text-blue-600 dark:text-blue-300",
      dot: "bg-blue-500 text-blue-500",
    },
    {
      icon: Sparkles,
      title: t("homePage.visual.polishTitle"),
      detail: t("homePage.visual.polishDetail"),
      accent: "bg-purple-500/12 text-purple-600 dark:text-purple-300",
      dot: "bg-purple-500 text-purple-500",
    },
    {
      icon: Lock,
      title: t("homePage.visual.localTitle"),
      detail: t("homePage.visual.localDetail"),
      accent: "bg-emerald-500/12 text-emerald-600 dark:text-emerald-300",
      dot: "bg-emerald-500 text-emerald-500",
    },
    {
      icon: Globe,
      title: t("homePage.visual.languageTitle"),
      detail: t("homePage.visual.languageDetail"),
      accent: "bg-amber-500/12 text-amber-600 dark:text-amber-300",
      dot: "bg-amber-500 text-amber-500",
    },
  ];

  const bars = [
    { key: 0, durations: [1.1, 0.9, 1.3], heights: [22, 38, 50, 34] },
    { key: 1, durations: [0.8, 1.2, 1.0], heights: [32, 52, 28, 44] },
    { key: 2, durations: [1.4, 0.7, 1.1], heights: [50, 34, 46, 22] },
    { key: 3, durations: [0.9, 1.3, 0.8], heights: [28, 42, 32, 50] },
    { key: 4, durations: [1.2, 1.0, 0.9], heights: [42, 26, 50, 34] },
  ];

  return (
    <div className="relative min-h-[520px] min-w-0 overflow-hidden rounded-3xl border border-border bg-card shadow-sm md:min-h-0" style={{ aspectRatio: "4 / 3" }}>
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_10%,rgba(74,222,128,0.16),transparent_32%),radial-gradient(circle_at_15%_85%,rgba(96,165,250,0.10),transparent_30%),linear-gradient(160deg,rgba(28,25,23,0.04),rgba(231,229,228,0.45))]" />
      <div className="relative flex h-full flex-col p-5 md:p-6">
        <div className="mx-auto flex flex-col items-center gap-2.5">
          <div className="flex items-center gap-2 rounded-full border border-border bg-background/90 px-3 py-1.5 shadow-sm backdrop-blur">
            <Keyboard className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-xs font-medium text-foreground md:text-sm">{t("homePage.visual.shortcut")}</span>
            <span className="ml-1 flex items-center gap-1 border-l border-border pl-2">
              <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded-md border border-border bg-secondary px-1.5 font-mono text-[10px] font-medium text-foreground shadow-[0_1px_0_rgba(0,0,0,0.04)]">
                ⌥
              </kbd>
              <kbd className="inline-flex h-5 min-w-12 items-center justify-center rounded-md border border-border bg-secondary px-1.5 font-mono text-[10px] font-medium text-foreground shadow-[0_1px_0_rgba(0,0,0,0.04)]">
                Space
              </kbd>
            </span>
          </div>
        </div>

        <div className="relative mx-auto mt-6 flex items-center justify-center">
          <motion.span
            aria-hidden
            className="absolute h-20 w-20 rounded-full bg-emerald-400/35 blur-md md:h-24 md:w-24"
            animate={{ scale: [1, 1.25, 1], opacity: [0.45, 0.12, 0.45] }}
            transition={{ duration: 1.8, ease: "easeInOut", repeat: Infinity }}
          />
          <motion.span
            aria-hidden
            className="absolute h-20 w-20 rounded-full border border-emerald-400/40 md:h-24 md:w-24"
            animate={{ scale: [1, 1.35, 1], opacity: [0.6, 0, 0.6] }}
            transition={{ duration: 2, ease: "easeOut", repeat: Infinity }}
          />
          <div className="relative flex h-20 w-20 items-center justify-center rounded-full border border-border bg-foreground shadow-[0_8px_24px_-8px_rgba(0,0,0,0.35),inset_0_1px_0_rgba(255,255,255,0.08)] md:h-24 md:w-24">
            <div className="flex items-end gap-1">
              {bars.map((bar) => (
                <motion.span
                  key={bar.key}
                  className="w-1.5 rounded-full bg-primary-foreground/95"
                  animate={{ height: bar.heights }}
                  transition={{
                    duration: bar.durations[0],
                    ease: "easeInOut",
                    repeat: Infinity,
                    repeatType: "mirror",
                  }}
                />
              ))}
            </div>
            <motion.span
              aria-hidden
              className="absolute -top-1.5 -right-1.5 flex h-3 w-3 items-center justify-center rounded-full bg-red-500 shadow-[0_0_0_3px_rgba(255,255,255,0.85)]"
              animate={{ opacity: [1, 0.4, 1] }}
              transition={{ duration: 1.2, repeat: Infinity }}
            >
              <span className="h-1.5 w-1.5 rounded-full bg-red-500/90" />
            </motion.span>
          </div>
        </div>

        <div className="mt-6 grid grid-cols-1 gap-2 sm:grid-cols-2">
          {items.map((item) => {
            const Icon = item.icon;
            return (
              <div
                key={item.title}
                className="group rounded-2xl border border-border/70 bg-background/80 p-3 backdrop-blur transition-colors hover:bg-background/95"
              >
                <div className="flex items-center justify-between">
                  <span className={`inline-flex h-7 w-7 items-center justify-center rounded-xl ${item.accent}`}>
                    <Icon className="h-3.5 w-3.5" />
                  </span>
                  <span className={`h-1.5 w-1.5 rounded-full ${item.dot} shadow-[0_0_6px_currentColor]`} />
                </div>
                <div className="mt-2 text-sm font-medium text-foreground">{item.title}</div>
                <div className="mt-0.5 text-xs leading-4 text-muted-foreground">{item.detail}</div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default function HomePage() {
  const { t } = useTranslation();
  const { lang } = useParams() as { lang: string };

  const steps = [
    {
      number: "01",
      title: t("homePage.steps.triggerTitle"),
      description: t("homePage.steps.triggerDescription"),
    },
    {
      number: "02",
      title: t("homePage.steps.speakTitle"),
      description: t("homePage.steps.speakDescription"),
    },
    {
      number: "03",
      title: t("homePage.steps.insertTitle"),
      description: t("homePage.steps.insertDescription"),
    },
  ];

  const featuresA = [
    {
      title: t("homePage.principles.cursorTitle"),
      description: t("homePage.principles.cursorDescription"),
    },
    {
      title: t("homePage.principles.privateTitle"),
      description: t("homePage.principles.privateDescription"),
    },
    {
      title: t("homePage.principles.desktopTitle"),
      description: t("homePage.principles.desktopDescription"),
    },
  ];

  const featuresB = [
    {
      title: t("homePage.controls.engineTitle"),
      description: t("homePage.controls.engineDescription"),
    },
    {
      title: t("homePage.controls.polishTitle"),
      description: t("homePage.controls.polishDescription"),
    },
  ];

  return (
    <div>
      <section className="pb-16 pt-32 md:pb-24 md:pt-44">
        <div className="mx-auto max-w-4xl px-6 text-center">
          <motion.div
            variants={reveal}
            initial="hidden"
            animate="visible"
            transition={{ ...transition, duration: 0.7 }}
            className="space-y-8"
          >
            <SectionLabel>{t("homePage.heroEyebrow")}</SectionLabel>
            <h1 className="text-[clamp(2.25rem,5.5vw,4.25rem)] font-semibold leading-[1.08] tracking-[-0.04em] text-foreground">
              {t("homePage.heroTitle")}
            </h1>
            <p className="mx-auto max-w-2xl text-lg leading-8 text-muted-foreground">
              {t("homePage.heroDescription")}
            </p>
          </motion.div>

          <motion.div
            variants={reveal}
            initial="hidden"
            animate="visible"
            transition={{ ...transition, delay: 0.12 }}
            className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row"
          >
            <HomeDownloadButton lang={lang} />
            <Link
              href="https://github.com/joe223/AriaType"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex h-11 items-center justify-center rounded-full border border-border bg-card px-6 text-sm font-medium text-foreground transition-colors hover:bg-secondary"
            >
              {t("homePage.heroSecondaryCta")}
            </Link>
          </motion.div>
        </div>
      </section>

      <section className="pb-20 md:pb-28">
        <div className="mx-auto max-w-5xl px-6">
          <motion.div
            variants={reveal}
            initial="hidden"
            animate="visible"
            transition={{ ...transition, delay: 0.18, duration: 0.8 }}
            className="relative overflow-hidden rounded-2xl shadow-sm"
          >
            <img
              src="/illustration/showcase.png"
              alt={t("hero.demoAlt")}
              className="block h-auto w-full"
            />
          </motion.div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-4xl px-6">
          <motion.div
            variants={reveal}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-60px" }}
            transition={transition}
            className="text-center"
          >
            <SectionLabel>{t("homePage.workflowEyebrow")}</SectionLabel>
            <h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
              {t("homePage.workflowTitle")}
            </h2>
          </motion.div>

          <div className="mt-16 grid gap-12 md:grid-cols-3 md:gap-8">
            {steps.map((step, index) => (
              <motion.div
                key={step.number}
                variants={reveal}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: "-40px" }}
                transition={{ ...transition, delay: index * 0.08 }}
                className="text-center md:text-left"
              >
                <span className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
                  {step.number}
                </span>
                <h3 className="mt-4 text-xl font-semibold tracking-[-0.03em] text-foreground">
                  {step.title}
                </h3>
                <p className="mt-3 text-sm leading-7 text-muted-foreground">
                  {step.description}
                </p>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-6xl px-6">
          <div className="grid items-center gap-16 lg:grid-cols-2">
            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={transition}
              className="min-w-0"
            >
              <SectionLabel>{t("homePage.controlsEyebrow")}</SectionLabel>
              <h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
                {t("homePage.controlsTitle")}
              </h2>
              <p className="mt-4 text-base leading-8 text-muted-foreground">
                {t("homePage.controlsDescription")}
              </p>
              <div className="mt-10 space-y-8">
                {featuresA.map((feature) => (
                  <div key={feature.title} className="flex gap-3">
                    <span className="mt-2 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground/40" />
                    <div>
                      <h3 className="text-base font-medium text-foreground">
                        {feature.title}
                      </h3>
                      <p className="mt-1.5 text-sm leading-7 text-muted-foreground">
                        {feature.description}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </motion.div>

            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={{ ...transition, delay: 0.08 }}
              className="min-w-0"
            >
              <ContextVisual t={t} />
            </motion.div>
          </div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-6xl px-6">
          <div className="grid items-center gap-16 lg:grid-cols-2">
            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={transition}
              className="min-w-0"
            >
              <LayerVisual t={t} />
            </motion.div>

            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={{ ...transition, delay: 0.08 }}
              className="min-w-0"
            >
              <SectionLabel>{t("homePage.summaryEyebrow")}</SectionLabel>
              <h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
                {t("homePage.summaryTitle")}
              </h2>
              <p className="mt-4 text-base leading-8 text-muted-foreground">
                {t("homePage.summaryDescription")}
              </p>
              <div className="mt-10 space-y-8">
                {featuresB.map((feature) => (
                  <div key={feature.title} className="flex gap-3">
                    <span className="mt-2 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground/40" />
                    <div>
                      <h3 className="text-base font-medium text-foreground">
                        {feature.title}
                      </h3>
                      <p className="mt-1.5 text-sm leading-7 text-muted-foreground">
                        {feature.description}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </motion.div>
          </div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-3xl px-6 text-center">
          <motion.div
            variants={reveal}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-60px" }}
            transition={transition}
            className="space-y-6"
          >
            <SectionLabel>{t("homePage.closingEyebrow")}</SectionLabel>
            <h2 className="text-3xl font-semibold tracking-[-0.04em] text-foreground md:text-4xl">
              {t("homePage.closingTitle")}
            </h2>
            <p className="text-lg leading-8 text-muted-foreground">
              {t("homePage.closingDescription")}
            </p>
            <div className="pt-4">
              <HomeDownloadButton lang={lang} />
              <p className="mt-3 text-sm text-muted-foreground">
                {t("homePage.closingFootnote")}
              </p>
            </div>
          </motion.div>
        </div>
      </section>
    </div>
  );
}
