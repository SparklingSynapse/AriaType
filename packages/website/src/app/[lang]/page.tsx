"use client";

import { useRef, useState } from "react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import { useParams } from "next/navigation";
import { motion } from "framer-motion";
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

export default function HomePage() {
  const { t } = useTranslation();
  const { lang } = useParams() as { lang: string };
  const heroVideoRef = useRef<HTMLVideoElement>(null);
  const [heroPlaying, setHeroPlaying] = useState(false);

  const playHeroVideo = () => {
    heroVideoRef.current?.play();
    setHeroPlaying(true);
  };

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
              href="https://github.com/SparklingSynapse/AriaType"
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
            className="relative overflow-hidden rounded-3xl shadow-sm"
            style={{ aspectRatio: "4 / 3" }}
          >
            <video
              ref={heroVideoRef}
              muted
              playsInline
              src="/illustration/showcase.mp4"
              className="h-full w-full object-cover"
              onPlay={() => setHeroPlaying(true)}
              onPause={() => setHeroPlaying(false)}
              onEnded={() => setHeroPlaying(false)}
            />
            {!heroPlaying && (
              <button
                onClick={playHeroVideo}
                className="absolute inset-0 flex items-center justify-center bg-foreground/10 transition-colors hover:bg-foreground/15"
                aria-label={t("homePage.heroVideoPlay")}
              >
                <span className="flex h-16 w-16 items-center justify-center rounded-full bg-foreground/60 backdrop-blur-sm">
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                    <path d="M8 5.14v13.72a1 1 0 0 0 1.5.86l11-6.86a1 1 0 0 0 0-1.72l-11-6.86a1 1 0 0 0-1.5.86Z" />
                  </svg>
                </span>
              </button>
            )}
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

            <motion.img
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={{ ...transition, delay: 0.08 }}
              src="/illustration/img-1.webp"
              alt={t("homePage.featureImageAlt")}
              className="w-full rounded-3xl object-cover"
              style={{ aspectRatio: "4 / 3" }}
            />
          </div>
        </div>
      </section>

      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-6xl px-6">
          <div className="grid items-center gap-16 lg:grid-cols-2">
            <motion.img
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={transition}
              src="/illustration/img-2.webp"
              alt={t("homePage.featureImageAlt")}
              className="w-full rounded-3xl object-cover"
              style={{ aspectRatio: "4 / 3" }}
            />

            <motion.div
              variants={reveal}
              initial="hidden"
              whileInView="visible"
              viewport={{ once: true, margin: "-60px" }}
              transition={{ ...transition, delay: 0.08 }}
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
