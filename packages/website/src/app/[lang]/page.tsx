"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "next/navigation";
import Link from "next/link";
import Image from "next/image";
import {
  Mic,
  Lock,
  Keyboard,
  Sparkles,
  Globe,
  Cpu,
  ShieldCheck,
  WifiOff,
  Check,
  X,
  Github,
  Users,
  Heart,
  Zap,
  ArrowRight,
  ChevronRight,
  Download,
  Play,
  Pause,
  Volume2,
  VolumeX,
  Brain,
  Layers,
  Fingerprint,
  Eye,
  EyeOff,
  Server,
  Laptop,
  CloudOff,
  FileCode,
  GitBranch,
  MessageSquare,
  Type,
  Wand2,
  Settings,
  Command,
  Mic2,
  Activity,
  Languages,
} from "lucide-react";
import { motion, useInView, AnimatePresence } from "framer-motion";
import useEmblaCarousel from "embla-carousel-react";
import Autoplay from "embla-carousel-autoplay";
import { Typewriter } from "@/components/Typewriter";
import { useAnalytics } from "@/lib/analytics";
import { AnalyticsEvents } from "@/lib/events";
import { HomeDownloadButton } from "@/components/HomeDownloadButton";

const container = {
  hidden: { opacity: 0 },
  show: { opacity: 1, transition: { staggerChildren: 0.08 } },
};
const item = { hidden: { opacity: 0, y: 20 }, show: { opacity: 1, y: 0 } };

// Counter animation component
function AnimatedCounter({
  target,
  increment,
  duration = 2000,
}: {
  target: number;
  increment: number;
  duration?: number;
}) {
  const [count, setCount] = useState(0);
  const ref = useRef<HTMLSpanElement>(null);
  const isInView = useInView(ref, { once: false, amount: 0.5 });

  useEffect(() => {
    if (!isInView) {
      setCount(0);
      return;
    }

    const startDelay = setTimeout(() => {
      const steps = Math.ceil(target / increment);
      const stepDuration = duration / steps;
      let currentStep = 0;

      const timer = setInterval(() => {
        currentStep++;
        const newValue = Math.min(currentStep * increment, target);
        setCount(newValue);
        if (newValue >= target) clearInterval(timer);
      }, stepDuration);

      return () => clearInterval(timer);
    }, 200);

    return () => clearTimeout(startDelay);
  }, [isInView, target, increment, duration]);

  return <span ref={ref}>{count}</span>;
}

// AI Polish Template Card
function AIPolishSection() {
  const { t } = useTranslation();
  const templates = [
    {
      name: "filler",
      desc: t("home.aiPolish0"),
      tagline: t("home.aiPolish0Tagline"),
      input: t("home.aiPolish0Input"),
      output: t("home.aiPolish0Output"),
      color: "bg-gradient-to-r from-blue-600 to-violet-600 text-white",
    },
    {
      name: "formal",
      desc: t("home.aiPolish1"),
      tagline: t("home.aiPolish1Tagline"),
      input: t("home.aiPolish1Input"),
      output: t("home.aiPolish1Output"),
      color: "bg-gradient-to-r from-blue-600 to-violet-600 text-white",
    },
    {
      name: "concise",
      desc: t("home.aiPolish2"),
      tagline: t("home.aiPolish2Tagline"),
      input: t("home.aiPolish2Input"),
      output: t("home.aiPolish2Output"),
      color: "bg-gradient-to-r from-blue-600 to-violet-600 text-white",
    },
    {
      name: "agent",
      desc: t("home.aiPolish3"),
      tagline: t("home.aiPolish3Tagline"),
      input: t("home.aiPolish3Input"),
      output: "Write a sorting algorithm in Python with time complexity analysis",
      color: "bg-gradient-to-r from-blue-600 to-violet-600 text-white",
    },
    {
      name: "custom",
      desc: t("home.aiPolish4"),
      tagline: t("home.aiPolish4Tagline"),
      input: t("home.aiPolish4Input"),
      output: t("home.aiPolish4Output"),
      color: "bg-gradient-to-r from-blue-600 to-violet-600 text-white",
    },
  ];
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [emblaRef, emblaApi] = useEmblaCarousel({ loop: true }, [
    Autoplay({ delay: 5000, stopOnInteraction: false }),
  ]);

  const scrollTo = useCallback(
    (index: number) => {
      if (emblaApi) emblaApi.scrollTo(index);
    },
    [emblaApi],
  );

  const onSelect = useCallback(() => {
    if (!emblaApi) return;
    setSelectedIndex(emblaApi.selectedScrollSnap());
  }, [emblaApi]);

  useEffect(() => {
    if (!emblaApi) return;
    onSelect();
    emblaApi.on("select", onSelect);
    emblaApi.on("reInit", onSelect);
  }, [emblaApi, onSelect]);

  return (
    <div className="space-y-6">
      <div className="overflow-hidden" ref={emblaRef}>
        <div className="flex">
          {templates.map((template, index) => (
            <div key={index} className="flex-[0_0_100%] min-w-0 px-2">
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                className="rounded-2xl bg-foreground text-background border border-foreground p-8"
              >
                <div className="space-y-6">
                  {/* Header */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className={`px-3 py-1.5 rounded-full text-sm font-mono font-bold ${template.color}`}>
                        {template.name}
                      </div>
                      <div className="text-xl md:text-2xl font-bold text-white">
                        {template.desc}
                      </div>
                    </div>
                    <div className="text-sm text-white">
                      {index + 1} / {templates.length}
                    </div>
                  </div>

                  {/* Tagline */}
                  <p className="text-sm text-white">
                    {template.tagline}
                  </p>

                  {/* Example */}
                  <div className="grid md:grid-cols-2 gap-6">
                    <div>
                      <div className="text-xs font-medium text-white mb-3 uppercase tracking-wide">
                        {t("home.input")}
                      </div>
                      <div className="text-base p-4 rounded-xl bg-background border border-border text-foreground leading-relaxed min-h-[80px] flex items-center">
                        {template.input}
                      </div>
                    </div>
                    <div>
                      <div className="text-xs font-medium text-white mb-3 uppercase tracking-wide">
                        {t("home.output")}
                      </div>
                      <div className="text-base p-4 rounded-xl bg-background border border-border text-foreground font-medium leading-relaxed min-h-[80px] flex items-center">
                        {template.output}
                      </div>
                    </div>
                  </div>
                </div>
              </motion.div>
            </div>
          ))}
        </div>
      </div>

      {/* Dots Indicator */}
      <div className="flex items-center justify-center gap-2">
        {templates.map((_, index) => (
          <button
            key={index}
            onClick={() => scrollTo(index)}
            className={`h-2 rounded-full transition-all ${
              index === selectedIndex
                ? "w-8 bg-foreground"
                : "w-2 bg-border hover:bg-foreground/50"
            }`}
            aria-label={`Go to template ${index + 1}`}
          />
        ))}
      </div>
    </div>
  );
}

export default function HomePage() {
  const { trackEvent } = useAnalytics();
  const { t } = useTranslation();
  const params = useParams();
  const lang = (params?.lang as string) || "en";

  const typewriterPhrases = [
    t("hero.typewriter0"),
    t("hero.typewriter1"),
    t("hero.typewriter2"),
    t("hero.typewriter3"),
    t("hero.typewriter4"),
    t("hero.typewriter5"),
    t("hero.typewriter6"),
  ];

  const steps = [
    {
      title: t("features.step0Title"),
      description: t("features.step0Description"),
    },
    {
      title: t("features.step1Title"),
      description: t("features.step1Description"),
    },
    {
      title: t("features.step2Title"),
      description: t("features.step2Description"),
    },
  ];

  return (
    <div className="min-h-screen">
      {/* Hero */}
      <section className="relative pt-32 pb-20 px-6 overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-b from-foreground/[0.02] via-foreground/[0.01] to-transparent" />

        <motion.div
          variants={container}
          initial="hidden"
          animate="show"
          className="max-w-4xl mx-auto text-center relative"
        >
          <motion.div
            variants={item}
            className="mb-6 flex flex-wrap items-center justify-center gap-2"
          >
            <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-blue-500 text-white  text-xs font-semibold">
              <Cpu className="w-3 h-3" />
              {t("hero.badge0")}
            </span>
            <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-500 text-white text-xs font-semibold">
              <ShieldCheck className="w-3 h-3" />
              {t("hero.badge1")}
            </span>
            <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-violet-500 text-white text-xs font-semibold">
              <WifiOff className="w-3 h-3" />
              {t("hero.badge2")}
            </span>
            <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-amber-500 text-white  border border-foreground/20 text-xs font-semibold">
              <Sparkles className="w-3.5 h-3.5" />
              {t("hero.beta")}
            </span>
          </motion.div>

          <motion.h1
            variants={item}
            className="text-5xl md:text-7xl font-bold tracking-tight mb-6 leading-tight"
          >
            {t("hero.title")}
            <br />
            <span className="text-primary inline-block min-w-[200px] md:min-w-[300px] h-[1.2em] text-left">
              <Typewriter phrases={typewriterPhrases} />
            </span>
          </motion.h1>

          <motion.p
            variants={item}
            className="text-xl text-muted-foreground max-w-2xl mx-auto mb-10"
          >
            {t("hero.description")}
          </motion.p>

          <motion.div
            variants={item}
            className="flex flex-col sm:flex-row items-center justify-center gap-4"
          >
            <HomeDownloadButton lang={lang} />
          </motion.div>
        </motion.div>

        {/* App screenshot */}
        <motion.div
          initial={{ opacity: 0, y: 48 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.45, duration: 0.8, ease: [0.16, 1, 0.3, 1] }}
          className="relative mt-16 max-w-5xl mx-auto py-8 px-6"
        >
          <div className="absolute -inset-8 bg-foreground/[0.03] blur-3xl rounded-3xl pointer-events-none" />
          <div
            className="absolute inset-6 rounded-2xl "
            style={{
              transform: "rotate(-4deg) translate(-1.5%, 2%) scale(1.1)",
            }}
          >
            <Image
              src="/home-dark.png"
              alt="AriaType dark"
              width={1824}
              height={1426}
              className="w-full"
            />
          </div>
          <div
            className="relative rounded-2xl overflow-hidden"
            style={{ transform: "rotate(1.5deg)" }}
          >
            <video
              className="w-full h-auto block"
              src="/showcase.mp4"
              poster="/home-light.png"
              autoPlay
              muted
              loop
              playsInline
              aria-label={t("hero.demoAlt")}
            />
          </div>
        </motion.div>
      </section>

      {/* Speed Comparison */}
      <section className="py-36 px-6 bg-secondary/20">
        <div className="max-w-5xl mx-auto">
          <div className="text-center mb-12">
            <motion.h2
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-3"
            >
              {t("home.speedTitle")}
            </motion.h2>
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: 0.1 }}
              className="text-lg text-muted-foreground max-w-2xl mx-auto"
            >
              {t("home.speedSubtitle")}
            </motion.p>
          </div>

          <div className="grid md:grid-cols-2 gap-5 max-w-3xl mx-auto">
            <motion.div
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ delay: 0.2 }}
              className="relative p-8 rounded-2xl bg-card border border-border"
            >
              <div className="relative">
                <Keyboard className="w-8 h-8 text-muted-foreground mb-4" />
                <h3 className="text-sm font-medium mb-2 text-muted-foreground">
                  {t("home.speedTyping")}
                </h3>
                <div className="flex items-baseline gap-2">
                  <span className="text-6xl md:text-7xl font-bold tabular-nums">
                    <AnimatedCounter
                      target={40}
                      increment={2}
                      duration={2000}
                    />
                  </span>
                  <span className="text-xl text-muted-foreground">
                    {t("home.speedWpm")}
                  </span>
                </div>
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, x: 20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ delay: 0.3 }}
              className="relative p-8 rounded-2xl bg-foreground text-background border border-foreground"
            >
              <div className="absolute top-4 right-4">
                <div className="px-3 py-1.5 rounded-full bg-gradient-to-r from-blue-600 to-violet-600 text-white text-sm font-bold">
                  3.75× faster
                </div>
              </div>
              <div className="relative">
                <Mic className="w-8 h-8 mb-4" />
                <h3 className="text-sm font-medium mb-2 opacity-90">
                  {t("home.speedVoice")}
                </h3>
                <div className="flex items-baseline gap-2">
                  <span className="text-6xl md:text-7xl font-bold tabular-nums">
                    <AnimatedCounter
                      target={150}
                      increment={9}
                      duration={2000}
                    />
                  </span>
                  <span className="text-xl opacity-80">
                    {t("home.speedWpm")}
                  </span>
                </div>
              </div>
            </motion.div>
          </div>

          <motion.p
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true }}
            transition={{ delay: 0.4 }}
            className="text-center text-xs text-muted-foreground mt-5"
          >
            {t("home.speedNote")}
          </motion.p>
        </div>
      </section>

      {/* Feature 1: 语音变文字 */}
      <section className="py-36 px-6">
        <div className="max-w-5xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="grid md:grid-cols-2 gap-12 items-center"
          >
            <div className="md:text-right">
              <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-4">
                {t("home.feature0Title")}
              </h2>
              <p className="text-lg text-muted-foreground leading-relaxed">
                {t("home.feature0Desc")}
              </p>
            </div>
            <div className="space-y-3">
              {[
                { icon: Mic, label: t("home.feature0Item0Label"), desc: t("home.feature0Item0Desc"), type: "text" },
                {
                  icon: Command,
                  label: t("home.feature0Item1Label"),
                  desc: t("home.feature0Item1Desc"),
                  type: "text",
                },
                {
                  icon: Laptop,
                  label: t("home.feature0Item2Label"),
                  desc: t("home.feature0Item2Desc"),
                  type: "apps",
                  apps: [
                    { name: "VS Code", icon: "https://api.iconify.design/vscode-icons:file-type-vscode.svg" },
                    { name: "Slack", icon: "https://api.iconify.design/logos:slack-icon.svg" },
                    { name: "Notion", icon: "https://api.iconify.design/logos:notion-icon.svg" },
                    { name: "Chrome", icon: "https://api.iconify.design/logos:chrome.svg" },
                    { name: "Figma", icon: "https://api.iconify.design/logos:figma.svg" },
                    { name: "Discord", icon: "https://api.iconify.design/logos:discord-icon.svg" },
                    { name: "Telegram", icon: "https://api.iconify.design/logos:telegram.svg" },
                    { name: "Safari", icon: "https://api.iconify.design/logos:safari.svg" },
                    { name: "Firefox", icon: "https://api.iconify.design/logos:firefox.svg" },
                    { name: "GitHub", icon: "https://api.iconify.design/logos:github-icon.svg" },
                  ],
                },
              ].map((item, i) => (
                <motion.div
                  key={i}
                  initial={{ opacity: 0, x: 20 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  viewport={{ once: true }}
                  transition={{ delay: i * 0.1 }}
                  className="flex items-center gap-4 p-4 rounded-xl bg-card border border-border/50 hover:border-border transition-all"
                >
                  <item.icon className="w-5 h-5 text-foreground/70" />
                  <div className="flex-1">
                    <div className="font-medium mb-1">{item.label}</div>
                    {item.type === "text" ? (
                      <div className="text-sm text-muted-foreground">
                        {item.desc}
                      </div>
                    ) : (
                      <div className="space-y-2">
                        <div className="text-sm text-muted-foreground">
                          {item.desc}
                        </div>
                        <div className="flex items-center gap-2 flex-wrap">
                          {item.apps?.map((app, j) => (
                            <div
                              key={j}
                              className="w-8 h-8 rounded-lg bg-white hover:bg-white flex items-center justify-center transition-colors"
                              title={app.name}
                            >
                              <img
                                src={app.icon}
                                alt={app.name}
                                className="w-5 h-5"
                                loading="lazy"
                              />
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                </motion.div>
              ))}
            </div>
          </motion.div>
        </div>
      </section>

      {/* Feature 2: AI 润色 */}
      <section className="py-36 px-6 bg-secondary/20">
        <div className="max-w-5xl mx-auto">
          <div className="text-center mb-12">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
            >
              <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-4">
                {t("home.feature1Title")}
              </h2>
              <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
                {t("home.feature1Desc")}
              </p>
            </motion.div>
          </div>
          <AIPolishSection />
        </div>
      </section>

      {/* Feature 3: 多语言 */}
      <section className="py-36 px-6">
        <div className="max-w-5xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="grid md:grid-cols-2 gap-12 items-center"
          >
            <div className="grid grid-cols-5 gap-3">
              {[
                { flag: "🇺🇸" },
                { flag: "🇬🇧" },
                { flag: "🇨🇳" },
                { flag: "🇭🇰" },
                { flag: "🇯🇵" },
                { flag: "🇰🇷" },
                { flag: "🇩🇪" },
                { flag: "🇫🇷" },
                { flag: "🇪🇸" },
                { flag: "🇮🇹" },
                { flag: "🇵🇹" },
                { flag: "🇷🇺" },
                { flag: "🇸🇦" },
                { flag: "🇮🇳" },
                { flag: "🇹🇭" },
                { flag: "🇻🇳" },
                { flag: "🇮🇩" },
                { flag: "🇲🇾" },
                { flag: "🇵🇭" },
                { flag: "🇳🇱" },
                { flag: "🇵🇱" },
                { flag: "🇹🇷" },
                { flag: "🇺🇦" },
                { flag: "🇬🇷" },
                { flag: "..." },
              ].map((lang, i) => (
                <motion.div
                  key={i}
                  initial={{ opacity: 0, scale: 0.8 }}
                  whileInView={{ opacity: 1, scale: 1 }}
                  viewport={{ once: true }}
                  transition={{ delay: i * 0.015 }}
                  className="flex items-center justify-center hover:scale-110 transition-transform"
                >
                  <div className="text-5xl">{lang.flag}</div>
                </motion.div>
              ))}
            </div>
            <div>
              <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-4">
                {t("home.feature2Title")}
              </h2>
              <p className="text-lg text-muted-foreground leading-relaxed mb-6">
                {t("home.feature2Desc")}
              </p>

              {/* Feature highlights - compact version */}
              <div className="space-y-2.5">
                <motion.div
                  initial={{ opacity: 0, x: 20 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  viewport={{ once: true }}
                  className="inline-flex items-center gap-2.5"
                >
                  <div className="w-7 h-7 rounded-full bg-primary/10 flex items-center justify-center shrink-0">
                    <Globe className="w-3.5 h-3.5 text-primary" />
                  </div>
                  <div>
                    <span className="font-semibold text-sm">{t("home.feature2Point0")}</span>
                    <span className="text-xs text-muted-foreground ml-2">
                      {t("home.feature2Point0Desc")}
                    </span>
                  </div>
                </motion.div>

                <motion.div
                  initial={{ opacity: 0, x: 20 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  viewport={{ once: true }}
                  transition={{ delay: 0.1 }}
                  className="inline-flex items-center gap-2.5"
                >
                  <div className="w-7 h-7 rounded-full bg-primary/10 flex items-center justify-center shrink-0">
                    <Zap className="w-3.5 h-3.5 text-primary" />
                  </div>
                  <div>
                    <span className="font-semibold text-sm">{t("home.feature2Point1")}</span>
                    <span className="text-xs text-muted-foreground ml-2">
                      {t("home.feature2Point1Desc")}
                    </span>
                  </div>
                </motion.div>

                <motion.div
                  initial={{ opacity: 0, x: 20 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  viewport={{ once: true }}
                  transition={{ delay: 0.2 }}
                  className="inline-flex items-center gap-2.5"
                >
                  <div className="w-7 h-7 rounded-full bg-primary/10 flex items-center justify-center shrink-0">
                    <Languages className="w-3.5 h-3.5 text-primary" />
                  </div>
                  <div>
                    <span className="font-semibold text-sm">{t("home.feature2Point2")}</span>
                    <span className="text-xs text-muted-foreground ml-2">
                      {t("home.feature2Point2Desc")}
                    </span>
                  </div>
                </motion.div>
              </div>
            </div>
          </motion.div>
        </div>
      </section>

      {/* Feature 4: 双引擎 */}
      {/* <section className="py-36 px-6 bg-secondary/20">
        <div className="max-w-4xl mx-auto">
          <div className="text-center mb-12">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
            >
              <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-4">
                {t("home.feature3Title")}
              </h2>
              <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
                {t("home.feature3Desc")}
              </p>
            </motion.div>
          </div>
          <div className="grid md:grid-cols-2 gap-6">
            {[
              {
                name: "Whisper",
                provider: t("home.feature3Engine0"),
                icon: Globe,
                feature: t("home.feature3Engine0Feature"),
                highlights: [t("home.feature3Engine0Tag0"), t("home.feature3Engine0Tag1"), t("home.feature3Engine0Tag2")],
              },
              {
                name: "SenseVoice",
                provider: t("home.feature3Engine1"),
                icon: Zap,
                feature: t("home.feature3Engine1Feature"),
                highlights: [t("home.feature3Engine1Tag0"), t("home.feature3Engine1Tag1"), t("home.feature3Engine1Tag2")],
              },
            ].map((engine, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: i * 0.1 }}
                className="p-6 rounded-xl bg-card border border-border hover:border-foreground/20 transition-all text-center"
              >
                <div className="flex flex-col items-center mb-4">
                  <div className="w-14 h-14 rounded-full bg-primary/10 flex items-center justify-center mb-4">
                    <engine.icon className="w-7 h-7 text-primary" />
                  </div>
                  <div className="font-mono text-2xl md:text-3xl font-bold mb-2">
                    {engine.name}
                  </div>
                  <div className="text-sm text-muted-foreground">
                    {engine.provider}
                  </div>
                </div>
                <p className="text-base mb-4 text-muted-foreground">
                  {engine.feature}
                </p>
                <div className="flex flex-wrap justify-center gap-2">
                  {engine.highlights.map((highlight, j) => (
                    <span
                      key={j}
                      className="px-2.5 py-1 text-xs rounded-full bg-secondary text-foreground"
                    >
                      {highlight}
                    </span>
                  ))}
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section> */}

      {/* Feature 5: 专业领域 */}
      <section className="py-36 px-6">
        <div className="max-w-5xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="grid md:grid-cols-2 gap-12 items-center"
          >
            <div className="md:text-right">
              <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-4">
                {t("home.feature4Title")}
              </h2>
              <p className="text-lg text-muted-foreground leading-relaxed mb-5">
                {t("home.feature4Desc")}
              </p>
              <p className="text-sm text-muted-foreground">
                {t("home.feature4CustomVocab")}
              </p>
            </div>
            <div>
              <div className="space-y-3">
                {[
                  {
                    icon: FileCode,
                    label: t("home.feature4Domain0"),
                    tags: [t("home.feature4Domain0Tag0"), t("home.feature4Domain0Tag1"), t("home.feature4Domain0Tag2"), t("home.feature4Domain0Tag3"), t("home.feature4Domain0Tag4")],
                  },
                  {
                    icon: ShieldCheck,
                    label: t("home.feature4Domain1"),
                    tags: [t("home.feature4Domain1Tag0"), t("home.feature4Domain1Tag1"), t("home.feature4Domain1Tag2"), t("home.feature4Domain1Tag3")],
                  },
                  {
                    icon: Heart,
                    label: t("home.feature4Domain2"),
                    tags: [t("home.feature4Domain2Tag0"), t("home.feature4Domain2Tag1"), t("home.feature4Domain2Tag2"), t("home.feature4Domain2Tag3")],
                  },
                ].map((item, i) => (
                  <motion.div
                    key={i}
                    initial={{ opacity: 0, x: 20 }}
                    whileInView={{ opacity: 1, x: 0 }}
                    viewport={{ once: true }}
                    transition={{ delay: i * 0.1 }}
                    className="flex items-start gap-3 p-4 rounded-xl bg-background text-foreground border border-border hover:border-foreground/20 transition-all"
                  >
                    <div className="w-8 h-8 rounded-full bg-foreground flex items-center justify-center shrink-0 mt-0.5">
                      <item.icon className="w-4 h-4 text-background" strokeWidth={2} />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium mb-1.5 text-sm">{item.label}</div>
                      <div className="flex flex-wrap gap-1">
                        {item.tags.map((tag, j) => (
                          <span
                            key={j}
                            className="text-xs text-muted-foreground"
                          >
                            {tag}
                            {j < item.tags.length - 1 && " · "}
                          </span>
                        ))}
                      </div>
                    </div>
                  </motion.div>
                ))}
              </div>
            </div>
          </motion.div>
        </div>
      </section>

      {/* Feature 6: 隐私至上 */}
      <section className="py-36 px-6 bg-secondary/20">
        <div className="max-w-4xl mx-auto">
          <div className="text-center mb-12">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
            >
              <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-4">
                {t("home.feature5Title")}
              </h2>
              <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
                {t("home.feature5Desc")}
              </p>
            </motion.div>
          </div>
          <div className="grid md:grid-cols-2 gap-6">
            {[
              {
                icon: WifiOff,
                label: t("home.feature5Point0"),
                desc: t("home.feature5Point0Desc"),
                highlight: t("home.feature5Point0Tag"),
                color: "bg-blue-500",
              },
              {
                icon: Fingerprint,
                label: t("home.feature5Point1"),
                desc: t("home.feature5Point1Desc"),
                highlight: t("home.feature5Point1Tag"),
                color: "bg-purple-500",
              },
              {
                icon: Lock,
                label: t("home.feature5Point2"),
                desc: t("home.feature5Point2Desc"),
                highlight: t("home.feature5Point2Tag"),
                color: "bg-orange-500",
              },
              {
                icon: Github,
                label: t("home.feature5Point3"),
                desc: t("home.feature5Point3Desc"),
                highlight: t("home.feature5Point3Tag"),
                color: "bg-green-500",
                href: "https://github.com/SparklingSynapse/aria-type",
              },
            ].map((item, i) => {
              const content = (
                <>
                  <div className={`w-12 h-12 rounded-full ${item.color} flex items-center justify-center shrink-0`}>
                    <item.icon className="w-6 h-6 text-white" strokeWidth={2} />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <h3 className="text-lg font-bold">{item.label}</h3>
                      <span className="px-2.5 py-1 text-xs rounded-full bg-secondary text-foreground font-semibold">
                        {item.highlight}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground leading-relaxed">
                      {item.desc}
                    </p>
                  </div>
                </>
              );

              return (
                <motion.div
                  key={i}
                  initial={{ opacity: 0, y: 20 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  viewport={{ once: true }}
                  transition={{ delay: i * 0.1 }}
                >
                  {item.href ? (
                    <Link
                      href={item.href}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() =>
                        trackEvent(AnalyticsEvents.CTA_CLICK, {
                          location: "privacy",
                          label: "github",
                        })
                      }
                      className="flex items-center gap-4 p-6 rounded-xl bg-card border border-border hover:border-foreground/20 transition-all"
                    >
                      {content}
                    </Link>
                  ) : (
                    <div className="flex items-center gap-4 p-6 rounded-xl bg-card border border-border hover:border-foreground/20 transition-all">
                      {content}
                    </div>
                  )}
                </motion.div>
              );
            })}
          </div>
        </div>
      </section>

      {/* Free & Open Source */}
      <section className="py-36 px-6">
        <div className="max-w-4xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="text-center mb-10"
          >
            <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-4">
              {t("home.freeTitle")}
            </h2>
            <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
              {t("home.freeDesc")}
            </p>
          </motion.div>
          <div className="grid md:grid-cols-2 gap-4">
            {[
              {
                label: t("home.freePoint0"),
                desc: t("home.freePoint0Desc"),
              },
              {
                label: t("home.freePoint1"),
                desc: t("home.freePoint1Desc"),
              },
              {
                label: t("home.freePoint2"),
                desc: t("home.freePoint2Desc"),
              },
              {
                label: t("home.freePoint3"),
                desc: t("home.freePoint3Desc"),
              },
            ].map((point, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: i * 0.1 }}
                className="flex items-center gap-4 p-5 rounded-xl bg-foreground hover:bg-foreground transition-all"
              >
                <div className="w-10 h-10 rounded-full bg-gradient-to-r from-blue-600 to-violet-600 flex items-center justify-center shrink-0">
                  <Check className="w-5 h-5 text-white" strokeWidth={2.5} />
                </div>
                <div>
                  <div className="text-lg font-bold mb-1 text-white">{point.label}</div>
                  <div className="text-sm text-white/80">
                    {point.desc}
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: 0.4 }}
            className="mt-8 text-center"
          >
            <Link
              href="https://github.com/SparklingSynapse/aria-type"
              target="_blank"
              rel="noopener noreferrer"
              onClick={() =>
                trackEvent(AnalyticsEvents.CTA_CLICK, {
                  location: "free-open-source",
                  label: "github",
                })
              }
              className="inline-flex items-center gap-2 px-8 py-4 rounded-lg bg-background text-foreground font-medium hover:bg-background/80 transition-colors"
            >
              <Github className="w-5 h-5" />
              {t("home.viewOnGitHub")}
            </Link>
          </motion.div>
        </div>
      </section>

      {/* How It Works */}
      <section className="py-36 px-6 bg-secondary/20">
        <div className="max-w-4xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="text-center mb-14"
          >
            <h2 className="text-4xl md:text-5xl font-extrabold tracking-tight leading-tight mb-3">
              {t("features.howItWorksTitle")}
            </h2>
            <p className="text-muted-foreground text-lg">
              {t("features.howItWorksSubtitle")}
            </p>
          </motion.div>

          <div className="flex flex-col gap-6">
            {steps.map((step, index) => (
              <motion.div
                key={index}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.1 }}
                className="flex items-start gap-6 p-6 rounded-xl bg-card border border-border"
              >
                <div className="w-12 h-12 rounded-full bg-foreground flex items-center justify-center font-bold text-xl text-background shrink-0">
                  {index + 1}
                </div>
                <div>
                  <h3 className="text-lg font-bold mb-2">{step.title}</h3>
                  <p className="text-muted-foreground text-sm leading-relaxed">
                    {step.description}
                  </p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-36 px-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="max-w-2xl mx-auto text-center"
        >
          <h2 className="text-5xl md:text-6xl font-bold tracking-tight mb-3">{t("features.ctaTitle")}</h2>
          <p className="text-muted-foreground mb-8">
            {t("features.ctaSubtitle")}
          </p>
          <HomeDownloadButton
            lang={lang}
            className="bg-gradient-to-r from-blue-600 to-violet-600 hover:from-blue-700 hover:to-violet-700"
          />
        </motion.div>
      </section>
    </div>
  );
}
