# Local STT Model Research Report

Evaluation of local STT models available via [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) for AriaType's desktop voice typing use case. Focus: Chinese-English bilingual accuracy, inference speed on consumer hardware, and model size constraints.

**Date**: 2026-04-08
**Status**: Active research — not yet integrated

---

## Executive Summary

| Rank | Model | Chinese Accuracy | English Accuracy | Speed (macOS M4) | Size | Languages | Recommendation |
|------|-------|-----------------|-----------------|-------------------|------|-----------|----------------|
| 🥇 | **Qwen3-ASR 0.6B** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 8.0 tok/s | ~1.2 GB (INT8) | 30 langs + 22 dialects | **Primary — best accuracy-coverage trade-off** |
| 🥈 | **SenseVoice Small** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 27.4 tok/s | ~240 MB (INT8) | 5 (zh/en/ja/ko/yue) | Speed champion, smallest footprint |
| 🥉 | **Paraformer Bilingual** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ~6.7 tok/s (est.) | ~226 MB (INT8) | 2 (zh/en) | Best Chinese CER, streaming native |
| 4 | **FireRedAsr v2** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | N/A | ~1.9 GB (INT8) | 2 (zh/en) + 20 dialects | Industrial Chinese, widest dialect set |
| 5 | **Whisper Small** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ~1.2 tok/s (sherpa-onnx) | ~490 MB | 99+ languages | **Current AriaType default**, broadest language net |
| 6 | **Whisper Turbo** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 1.9 tok/s (CoreML) | ~1.5 GB | 99+ languages | Best Whisper speed-accuracy, needs CoreML |
| 7 | **Whisper Large-v3** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ~0.16 tok/s (sherpa-onnx) | ~2.9 GB | 99+ languages | Best Whisper accuracy, too slow for voice typing |
| 8 | **Parakeet TDT 0.6B v3** | ❌ | ⭐⭐⭐⭐⭐ | **171.6 tok/s** (CoreML) | ~640 MB (INT8) | 25 EU langs | Fastest on Apple Silicon, no Chinese |
| 9 | **Distil-Whisper large-v3.5** | ❌ | ⭐⭐⭐⭐⭐ | N/A (est. 6x Large-v3) | ~1.5 GB (FP16) | EN only | Best English speed-accuracy, 0% hallucination overhead |
| 10 | **Distil-Whisper small.en** | ❌ | ⭐⭐⭐ | N/A | ~332 MB (FP16) | EN only | Smallest Whisper distillation, fast English |
| 11 | **Dolphin** | ⭐⭐⭐ | ⭐⭐⭐ | N/A | ~365 MB (INT8) | 40 Eastern + 22 dialects | Broadest Eastern language coverage |
| 12 | **Moonshine v2** | ⭐⭐ | ⭐⭐⭐⭐ | 92.2 tok/s (Tiny) | ~95-280 MB | 8 (per-language models) | English speed king, no Chinese value |
| 13 | **FunASR Nano** | ⭐⭐⭐⭐ | ⭐⭐⭐ | N/A | ~200-600 MB | 7+ Chinese dialects | Dialect specialist, high VRAM |

---

## All Variants Comparison Matrix

Every model variant available in sherpa-onnx, sorted by model family. Bold = recommended for AriaType.

### Size & Accuracy

| Model Family | Variant | Params | Size | Chinese CER | English WER | Languages | Streaming |
|---|---|---|---|---|---|---|---|
| **SenseVoice** | **Small (INT8)** | 234M | ~240 MB | **1.70%** (AISHELL-1) | 2.40% (LibriSpeech) | 5 (zh/en/ja/ko/yue) | Simulated (VAD) |
| **Whisper** | Tiny | 39M | ~75 MB | — | 4.00% (clean) | 99+ | ❌ |
| **Whisper** | Base | 74M | ~140 MB | — | 3.00% (clean) | 99+ | ❌ |
| **Whisper** | **Small** | **244M** | **~490 MB** | **2.10%** | **2.50%** (clean) | **99+** | ❌ |
| **Whisper** | Medium | 769M | ~1.5 GB | 1.90% | 2.00% (clean) | 99+ | ❌ |
| **Whisper** | Large-v3 | 1,550M | ~2.9 GB | 1.80% | **1.80%** (clean) | 99+ | ❌ |
| **Whisper** | Turbo | 809M | ~1.5 GB | ~1.80% | ~1.80% (clean) | 99+ | ❌ |
| **Distil-Whisper** | small.en | 166M | ~332 MB (FP16) | — | 3.48% (clean) | EN only | ❌ |
| **Distil-Whisper** | medium.en | 394M | ~788 MB (FP16) | — | 3.69% (clean) | EN only | ❌ |
| **Distil-Whisper** | large-v2 | 756M | ~1.5 GB (FP16) | — | 2.94% (clean) ¹ | EN only | ❌ |
| **Distil-Whisper** | large-v3 | 756M | ~1.5 GB (FP16) | — | 2.43% (clean) ² | EN only | ❌ |
| **Distil-Whisper** | large-v3.5 | 756M | ~1.5 GB (FP16) | — | **2.37%** (clean) | EN only | ❌ |
| **Moonshine** | Tiny (v2) | 34M | ~95 MB | — | 4.49% (clean) | EN only | ✅ Native |
| **Moonshine** | Small (v2) | 123M | ~135 MB | — | 2.49% (clean) | EN only | ✅ Native |
| **Moonshine** | Medium (v2) | 245M | ~280 MB | — | **2.08%** (clean) | EN only | ✅ Native |
| **Moonshine** | Base (v1, per-lang) | 61M | ~135 MB | — | — | 8 (zh/en/ja/ko/ar/es/uk/vi) | ✅ Native |
| **Qwen3-ASR** | **0.6B (INT8)** | **600M** | **~937 MB** | **5.97%** (WenetSpeech) | **2.11%** (LibriSpeech) | **30 + 22 dialects** | ✅ |
| **Qwen3-ASR** | 1.7B | 1,700M | ~3 GB (est.) | 4.97% (WenetSpeech) | 1.63% (LibriSpeech) | 30 + 22 dialects | ✅ |
| **Paraformer** | Bilingual (streaming) | ~440M | ~226 MB (INT8) | ~2-3% | ~4-6% | 2 (zh/en) | ✅ Native |
| **Paraformer** | Bilingual (offline) | ~440M | ~825 MB (FP32) | ~2-3% | ~4-6% | 2 (zh/en) | ❌ |
| **Paraformer** | Trilingual (streaming) | ~440M | ~226 MB (INT8) | ~2-3% | ~4-6% | 3 (zh/en/yue) | ✅ Native |
| **FireRedAsr** | v2 CTC (INT8) | ~220M | ~1.9 GB | ~3-5% | ~5-7% | 2 + 20 dialects | ✅ |
| **FireRedAsr** | v2 AED (INT8) | ~220M | ~1.9 GB | ~3-5% (better) | ~5-7% (better) | 2 + 20 dialects | ✅ |
| **FireRedAsr** | v1 Large | — | — | ~3-5% | ~5-7% | 2 + 3 dialects | ✅ |
| **Dolphin** | Small (INT8) | — | ~200 MB (est.) | ~4-6% | ~6-8% | 40 Eastern + 22 dialects | ✅ |
| **Dolphin** | Base (INT8) | — | ~365 MB | ~4-6% | ~6-8% | 40 Eastern + 22 dialects | ✅ |
| **Zipformer** | Bilingual (streaming) | ~30-80M | ~30-80 MB | ~5-8% | ~5-8% | 2 (zh/en) | ✅ Native |
| **Zipformer** | Chinese XL (streaming, INT8) | — | — | ~3-5% | — | 1 (zh) | ✅ Native |
| **FunASR Nano** | — | — | ~200-600 MB | ~3-5% | ~5-7% | 7+ dialects | ✅ |
| **Parakeet TDT** | 0.6B v2 | 600M | ~640 MB (INT8) | — | **1.69%** (clean) | EN only | ❌ |
| **Parakeet TDT** | 0.6B v3 | 600M | ~640 MB (INT8) | — | 1.93% (clean) | 25 EU | ❌ |
| **Parakeet TDT** | 1.1B | 1,100M | ~1.2 GB (est.) | — | **1.39%** (clean) | EN only | ❌ |
| **Parakeet TDT** | CTC 110M | 110M | **126 MB** (INT8) | — | — | EN only | ❌ |

> ¹ distil-large-v2 LibriSpeech clean WER from HF Open ASR Leaderboard (2.94%). Model card eval snippet shows 2.98%. Paper only reports avg OOD WER across 4 datasets (10.1%), not per-dataset.
> ² distil-large-v3 LibriSpeech clean WER from model card evaluation output (2.43%). NOT 2.54% — that number does not appear in any official source.

### Speed (MacBook M3 / M4)

> M3 latency from Northflank/VoicePing benchmarks and model papers. M4 tok/s from VoicePing benchmarks. These are different hardware — M4 is ~20-30% faster than M3 for the same model.
> ¹ Moonshine v2 M3 latency from paper Table 2 (arXiv:2602.12241v1). The Moonshine README reports lower latency (34/73/107ms) for streaming variants using a different benchmark setup.

| Model Family | Variant | MacBook M3 Latency | MacBook M4 tok/s | Compute Load |
|---|---|---|---|---|
| **Moonshine** | Tiny (34M) | **50 ms** ¹ | 92.2 | 8.03% |
| **Moonshine** | Small (123M) | **148 ms** ¹ | — | 17.97% |
| **Moonshine** | Medium (245M) | **258 ms** ¹ | — | 28.95% |
| **SenseVoice** | Small (234M) | **~70 ms** | 27.4 | — |
| **Whisper** | Tiny (39M) | 289 ms | — | 8.46% |
| **Whisper** | Base (74M) | 553 ms | — | 16.19% |
| **Whisper** | **Small (244M)** | **1,940 ms** | **~1.2** | **56.84%** |
| **Distil-Whisper** | small.en (166M) | ~350 ms (est.) | — | — |
| **Distil-Whisper** | medium.en (394M) | ~700 ms (est.) | — | — |
| **Distil-Whisper** | large-v3.5 (756M) | ~1,800 ms (est.) | — | — |
| **Qwen3-ASR** | 0.6B | — | 8.0 | — |
| **Whisper** | Large-v3 (1.5B) | 11,286 ms | — | 330.65% |
| **Whisper** | Turbo (809M) | ~3,000 ms | 1.9 | — |

### Speed (Windows CPU: Intel i5-1035G1)

| Model Family | Variant | tok/s | RTF |
|---|---|---|---|
| **Moonshine** | Tiny | **50.6** | **0.040** |
| **SenseVoice** | Small | **47.6** | **0.042** |
| **Moonshine** | Base | 41.2 | 0.049 |
| **Qwen3-ASR** | 0.6B | 1.6 ² | 1.214 ² |
| **Whisper** | Tiny (whisper.cpp) | 9.5 | 0.211 |
| **Whisper** | Small (whisper.cpp) | 1.0 | 1.933 |
| **Paraformer** | Bilingual | — | 0.15 (INT8) |

### Speed (Android: Samsung Galaxy S10, Exynos 9820)

| Model Family | Variant | tok/s | RTF |
|---|---|---|---|
| **Moonshine** | Tiny | **42.55** | **0.05** |
| **SenseVoice** | Small | 33.62 | 0.06 |
| **Whisper** | Tiny | 27.08 | 0.07 |
| **Whisper** | Small | 4.70 | 0.41 |
| **Qwen3-ASR** | 0.6B | 3.65 | 0.53 |

---

## Evaluation Criteria

All models are evaluated on sherpa-onnx (ONNX Runtime). Metrics from:

- [VoicePing Offline STT Benchmark (Feb 2026)](https://voiceping.net/en/blog/research-offline-speech-transcription-benchmark/) — cross-platform RTF and tok/s
- [Northflank STT Model Comparison (Jan 2026)](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks) — accuracy and latency
- [Qwen3-ASR Technical Report (arXiv:2601.21337)](https://arxiv.org/html/2601.21337v1) — Qwen3-ASR benchmarks
- Official model papers and repos

---

## 1. SenseVoice Small (FunAudioLLM / Alibaba)

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [FunAudioLLM/SenseVoice](https://github.com/FunAudioLLM/SenseVoice) |
| Architecture | Non-autoregressive Transformer (Paraformer lineage) |
| Parameters | ~234M |
| Model Size (INT8) | ~240 MB |
| Languages | 5: Chinese (Mandarin), English, Japanese, Korean, Cantonese |
| Chinese Dialects | Cantonese only |
| Streaming | Simulated (via VAD chunking), not native streaming |
| License | Apache 2.0 |
| sherpa-onnx | ✅ First-class support |

### Accuracy

| Dataset | Metric | SenseVoice Small | Whisper Large-v3 | Whisper Small |
|---------|--------|-------------------|------------------|---------------|
| AISHELL-1 (Chinese) | CER% | **1.70** | 1.80 | 2.10 |
| AISHELL-2 (Chinese) | CER% | **1.75** | 1.90 | 2.30 |
| WenetSpeech (Chinese) | CER% | **2.10** | 2.50 | 3.20 |
| LibriSpeech (English) | WER% | **2.40** | 2.30 | 2.50 ¹ |
| Common Voice (English) | WER% | **4.20** | 4.50 | 5.40 ¹ |

**Chinese accuracy beats Whisper Large-v3** on all Chinese benchmarks despite being 6x smaller. English accuracy is comparable — Whisper Large-v3 wins on LibriSpeech (2.30% vs 2.40%), but SenseVoice wins on Common Voice (4.20% vs 4.50%).

### Speed

| Platform | Metric | Value |
|----------|--------|-------|
| Windows CPU (i5-1035G1) | tok/s | **47.6** |
| Windows CPU (i5-1035G1) | RTF | **0.042** |
| Android (Exynos 9820) | tok/s | 33.62 |
| Android (Exynos 9820) | RTF | 0.06 |
| macOS M4 | tok/s | 27.4 |
| RK3588 Cortex-A76 (4t) | RTF | 0.049 |

### Special Features

- ✅ Emotion Recognition (7 emotions: happy, sad, angry, neutral, fearful, disgusted, surprised)
- ✅ Audio Event Detection (8 events: BGM, speech, applause, laughter, cry, sneeze, breath, cough)
- ✅ Inverse Text Normalization (ITN)
- ✅ Language auto-detection
- ✅ Timestamp generation

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| Fastest overall inference (RTF 0.042 on Windows) | Only 5 languages, no dialect breadth |
| Beats Whisper Large-v3 on Chinese CER | No native streaming |
| Smallest footprint (~240 MB) | Cantonese only dialect |
| Unique emotion + audio event detection | No hotword support |
| Already used in AriaType (migration path clear) | — |

---

## 2. Whisper (OpenAI) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [openai/whisper](https://github.com/openai/whisper) |
| Architecture | Autoregressive Transformer encoder-decoder |
| Languages | **99+ languages** (auto-detect) |
| Chinese Dialects | Mandarin only |
| Streaming | ❌ Fixed 30-second window, no native streaming |
| License | MIT |
| sherpa-onnx | ✅ (Tiny, Large-v3 available; others via conversion) |

### Variant Comparison

| Variant | Params | Size (FP16) | M3 Latency | Chinese CER (AISHELL-1) | English WER (LibriSpeech clean) |
|---------|--------|-------------|------------|--------------------------|---------------------------------|
| Tiny | 39M | ~75 MB | 289 ms | — | 4.00% ³ |
| Base | 74M | ~140 MB | 553 ms | — | 3.00% ³ |
| **Small** ⬅️ AriaType | **244M** | **~490 MB** | **1,940 ms** | **2.10%** | **2.50%** ¹ |
| Medium | 769M | ~1.5 GB | ~5,000 ms (est.) | 1.90% ³ | 2.00% ³ |
| Turbo | 809M | ~1.5 GB | ~3,000 ms (est.) | ~1.80% | ~1.80% |
| Large-v3 | 1,550M | ~2.9 GB | 11,286 ms | 1.80% ³ | **1.80%** ² |

> ¹ Whisper Small WER varies by source: Whisper paper reports ~3.3% (multilingual), 2.50% is from AriaType's sherpa-onnx/whisper.cpp evaluation. The `.en` variant achieves ~2.8%.
> ² Whisper Large-v3 WER varies by evaluation: 1.80% from Whisper paper, 1.51% from Qwen3-ASR paper (different decoding settings), 2.70% from HF Open ASR Leaderboard (greedy, no LM). Cross-paper comparisons should use same evaluation methodology.
> ³ Whisper paper (Radford et al., 2022) Table 8 reports higher greedy-decoding WER: Tiny 7.6%, Base 5.0%, Medium 2.9%, Large 2.7%. Our numbers use sherpa-onnx eval with beam search + temperature fallback, which produces better results. Chinese CER numbers (AISHELL-1, WenetSpeech) are from sherpa-onnx evaluation — the original Whisper paper does NOT report Chinese benchmarks.
> ³ Whisper Small Common Voice WER: 5.40% from sherpa-onnx eval, 9.00% from SenseVoice paper (likely different test split).

### Key Observations

1. **Tiny and Base** — too inaccurate for voice typing (WER >3%), only useful for low-resource embedded devices
2. **Small** — AriaType's current default. Good balance of size (490 MB) and accuracy, but 1.94s latency on M3 is slow
3. **Medium** — marginal accuracy improvement over Small (CER 1.90% vs 2.10%), but 3x larger and ~2.5x slower
4. **Turbo** — similar accuracy to Large-v3 at half the size. Best "quality" option if you accept 3s latency
5. **Large-v3** — best English WER (1.80%), but 11.3s latency makes it unusable for real-time voice typing

### Whisper Small vs SenseVoice Small (Head-to-Head)

| Dataset | Whisper Small | SenseVoice Small | Winner |
|---------|---------------|-------------------|--------|
| AISHELL-1 (Chinese) | 2.10 | **1.70** | SenseVoice |
| AISHELL-2 (Chinese) | 2.30 | **1.75** | SenseVoice |
| WenetSpeech (Chinese) | 3.20 | **2.10** | SenseVoice |
| LibriSpeech (English) | 2.50 | **2.40** | SenseVoice |
| Common Voice (English) | 9.00 | **4.20** | SenseVoice |
| Model size | 490 MB | **240 MB** | SenseVoice |
| M3 latency | 1,940 ms | **~70 ms** | SenseVoice |
| Language coverage | **99+** | 5 | Whisper |

**SenseVoice Small wins on every accuracy + speed metric.** Whisper Small only wins on language breadth (99+ vs 5).

### Speed

| Variant | MacBook M3 Latency | Compute Load | Android tok/s (sherpa-onnx) | Android RTF |
|---------|-------------------|--------------|-----------------------------|-------------|
| Tiny | 289 ms | 8.46% | 27.08 | 0.07 |
| Small | 1,940 ms | 56.84% | 4.70 | 0.41 |
| Large-v3 | 11,286 ms | 330.65% | — | — |
| Turbo | ~3,000 ms | — | — | — |

| Platform | Engine | Model | tok/s | RTF |
|----------|--------|-------|-------|-----|
| Windows CPU | sherpa-onnx | Tiny | 27.08 | 0.07 |
| Windows CPU | sherpa-onnx | Small | 4.70 | 0.41 |
| Android | whisper.cpp | Tiny | 0.55 | 3.52 |
| macOS M4 | CoreML | Turbo | 1.9 | — |

**sherpa-onnx is 51x faster** than whisper.cpp for the same Whisper Tiny model on Android.

### Architecture Limitations (All Variants)

1. **Fixed 30-second input window** — wastes compute on zero-padding for short utterances (AriaType's typical 3-10s recordings)
2. **No KV caching** — recomputes identical audio on every call
3. **Language quality cliff** — only 33/82 languages achieve sub-20% WER
4. **No streaming** — must wait for full 30s chunk processing
5. **Autoregressive decoding** — generates tokens sequentially, inherently slower than non-autoregressive models

### AriaType Migration Note

Whisper Small is the current default. Any replacement must:
1. Offer at least 99+ language auto-detect OR explicitly target Chinese-English users
2. Provide a clear speed improvement on macOS Apple Silicon
3. Maintain or improve Chinese accuracy (CER ≤ 2.10%)

---

## 3. Distil-Whisper (Hugging Face) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [huggingface/distil-whisper](https://github.com/huggingface/distil-whisper) |
| Architecture | Whisper encoder + 2-layer distilled decoder (4 layers for small.en) |
| Paper | [Robust Knowledge Distillation via Hypothesis Ensemble (arXiv:2311.00430)](https://arxiv.org/abs/2311.00430) |
| Languages | **English only** (all official variants) |
| Chinese Dialects | ❌ None |
| Streaming | ❌ Offline only, fixed 30-second window |
| License | MIT |
| sherpa-onnx | ✅ All 5 variants available as pre-exported ONNX |

> **Critical**: All official Distil-Whisper variants are trained on English data only. Even distil-large-v2/v3/v3.5 (which inherit the multilingual encoder from Whisper large) were distilled exclusively on English. They **cannot transcribe Chinese**. The Hugging Face team explicitly recommends [Whisper Turbo](https://huggingface.co/openai/whisper-large-v3-turbo) for multilingual use.

### Variant Comparison

| Variant | Teacher | Params | Dec Layers | Size (FP16) | Size (INT8 sherpa-onnx) | Speedup vs Teacher |
|---------|---------|--------|------------|-------------|------------------------|--------------------|
| **distil-small.en** | whisper-small.en | 166M | 4 | ~332 MB | ~965 MB | 5.6x |
| **distil-medium.en** | whisper-medium.en | 394M | 2 | ~788 MB | ~2.15 GB | 6.8x |
| **distil-large-v2** | whisper-large-v2 | 756M | 2 | ~1.5 GB | ~4.01 GB | 5.8x |
| **distil-large-v3** | whisper-large-v3 | 756M | 2 | ~1.5 GB | ~4.01 GB | 6.3x |
| **distil-large-v3.5** | whisper-large-v3 | 756M | 2 | ~1.5 GB | ~4.01 GB | ~1.5x vs Turbo |

> **sherpa-onnx download sizes are much larger than FP16 model weights** because the download bundles include both INT8 and FP32 weight files, ONNX graph structure, tokenizer, and vocabulary. The actual INT8 encoder + decoder alone are much smaller (see sherpa-onnx Model Files table below).

### Accuracy — Short-Form (English only, WER%)

| Dataset | distil-small.en | distil-medium.en | distil-large-v3 | distil-large-v3.5 | Whisper Large-v3 |
|---------|-----------------|------------------|-----------------|-------------------|------------------|
| LibriSpeech clean | 3.48% | 3.69% | 2.43% ² | **2.37%** | **1.80%** |
| LibriSpeech other | 7.73% | 8.35% | 5.19% | **5.04%** | **3.97%** |
| Tedlium | 4.54% | 4.84% | 3.86% | **3.64%** | 6.84% |
| GigaSpeech | 10.87% | 11.30% | 10.08% | **9.84%** | 9.76% |
| Earnings22 | 13.15% | 12.99% | 11.79% | **11.29%** | — |
| SPGISpeech | 3.82% | 3.83% | 3.27% | **2.87%** | — |
| **Avg OOD WER** | **12.1%** | **11.1%** | **9.7%** | **7.08%** | **8.4%** |

### Accuracy — Long-Form (English only, WER%)

| Algorithm | distil-large-v2 | distil-large-v3 | distil-large-v3.5 | Whisper Large-v3 |
|-----------|-----------------|-----------------|-------------------|------------------|
| Sequential | 15.6% | **10.8%** | **10.04%** | 11.0% |
| Chunked | **11.6%** | **10.9%** | — | 11.0% |

> distil-large-v3.5 achieves **lower long-form WER (10.04%) than the full Whisper Large-v3 (11.0%)** — likely because distillation reduces hallucination errors. From the paper: distil models have 1.3x fewer 5-gram duplicates and 1.2% lower insertion error rate.

### Speed

| Variant | Speedup vs Teacher | RTFx (long-form avg) | Notes |
|---------|--------------------|-----------------------|-------|
| distil-small.en | 5.6x faster than small.en | — | Smallest, fastest |
| distil-medium.en | 6.8x faster than medium.en | — | Good balance |
| distil-large-v2 | 5.8x faster than large-v2 | 31.72 | Proven, stable |
| distil-large-v3 | 6.3x faster than large-v3 | 48.64 | Best sequential compat |
| distil-large-v3.5 | ~1.5x faster than Turbo | **49.34** | Best accuracy + speed |

> **RTFx = Real-Time Factor (higher = faster)**. An RTFx of 49 means processing 49 seconds of audio per wall-clock second. From paper Table 5: distil-large-v2 with chunked batching is **57.5x faster** than Whisper large-v2 on long-form.

### Speculative Decoding

Distil-Whisper's primary design goal is to serve as a **draft model for speculative decoding** alongside full Whisper:

| Draft Model | Target Model | Speedup | Accuracy Loss |
|-------------|-------------|---------|---------------|
| distil-large-v2 | Whisper large-v2 | **2x** | **0%** (bitwise identical output) |
| distil-large-v3 | Whisper large-v3 | **2x** | **0%** |
| distil-large-v3.5 | Whisper large-v3 | **2x** | **0%** |

> Speculative decoding mathematically guarantees identical outputs to the full model. The distil model proposes tokens, the full model verifies them in batch. Only 8% parameter overhead (2 decoder layers added to full model).

### sherpa-onnx Model Files

| Variant | Encoder (INT8) | Decoder (INT8) | Total INT8 (all files) |
|---------|----------------|-----------------|------------|
| distil-small.en | 103 MB | 195 MB | ~965 MB ¹ |
| distil-medium.en | 328 MB | 245 MB | ~2.15 GB ¹ |
| distil-large-v2 | 667 MB | 315 MB | ~4.01 GB ² |
| distil-large-v3 | 668 MB | 315 MB | ~4.01 GB ² |
| distil-large-v3.5 | 668 MB | 315 MB | ~4.01 GB ² |

> ¹ Total includes FP32 weights, vocabulary, tokenizer, and other metadata files alongside INT8 encoder/decoder.
> ² Large models split into `.onnx` (graph structure) + `.weights` (raw parameters) files due to ONNX file size limits.

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| 5.8–6.8x faster than full Whisper | **English only — no Chinese at all** |
| Lower hallucination than full Whisper | No streaming support |
| Speculative decoding (2x full Whisper, 0% loss) | INT8 ONNX sizes are large (~4 GB for large) |
| distil-large-v3.5 beats Whisper large-v3 on long-form | Only offline inference |
| MIT license, production-ready | Not useful for AriaType's Chinese use case |

### Why Distil-Whisper Is Not Recommended for AriaType

Distil-Whisper is an excellent English STT model family. However:

1. **No Chinese support** — all variants trained on English data only
2. **sherpa-onnx INT8 sizes (0.97–4.01 GB)** are significantly larger than SenseVoice (240 MB) or even Whisper Small FP16 (490 MB)
3. **No streaming** — same fixed 30-second window limitation as regular Whisper
4. AriaType's primary differentiator is Chinese-English bilingual quality — Distil-Whisper contributes nothing on the Chinese side
5. **Useful only if**: AriaType adds speculative decoding in the future, where distil-large-v3 could accelerate Whisper large-v3 with zero accuracy loss

---

## 4. Moonshine (Useful Sensors) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [usefulsensors/moonshine](https://github.com/usefulsensors/moonshine) |
| Architecture | Ergodic streaming encoder + AR decoder |
| Languages | EN only (v2); 8 per-lang models available (v1, separate paper) |
| Chinese Dialects | None |
| Streaming | ✅ **Native streaming** (sliding-window attention, 80ms lookahead) |
| License | MIT |
| sherpa-onnx | ✅ Full support |

### Variant Comparison

#### Streaming Variants (v2) — Native Streaming

| Variant | Params | Size (Quantized) | Languages | LibriSpeech Clean WER | M3 Latency |
|---------|--------|-------------------|-----------|----------------------|------------|
| Tiny | 34M | ~95 MB | EN only | 4.49% | **50 ms** ¹ |
| Small | 123M | ~135 MB | EN only | 2.49% | **148 ms** ¹ |
| Medium | 245M | ~280 MB | EN only | **2.08%** | **258 ms** ¹ |

#### Per-Language Variants (v1 Base) — 8 Languages

> Per-language models are v1 derivatives from a separate paper (*Flavors of Moonshine*, arXiv:2509.02523), not v2. The v2 paper focuses exclusively on English and states: "We plan to train Moonshine v2 variants for additional languages."

| Language | Params | Size (Quantized) |
|----------|--------|-------------------|
| English | 61M | ~135 MB |
| Chinese | 61M | ~135 MB |
| Japanese | 61M | ~135 MB |
| Korean | 61M | ~135 MB |
| Arabic | 61M | ~135 MB |
| Spanish | 61M | ~135 MB |
| Ukrainian | 61M | ~135 MB |
| Vietnamese | 61M | ~135 MB |

#### v1 Variants (English Only, Legacy)

| Variant | Params | Size (Quantized) | LibriSpeech Clean WER |
|---------|--------|-------------------|----------------------|
| Tiny | 27M | ~125 MB | ~5.5% |
| Base | 61M | ~290 MB | ~3.5% |

### Key Observations

1. **Tiny (34M)** — fastest model in this report at 34ms. WER 4.49% is acceptable for casual dictation
2. **Small (123M)** — excellent speed/accuracy balance at 73ms. WER 2.49% beats Whisper Small (2.50%)
3. **Medium (245M)** — best Moonshine accuracy at WER 2.08%, still only 107ms. **Beats Whisper Small** in both speed (107ms vs 1,940ms) and accuracy (2.08% vs 2.50%)
4. **Per-language Base (61M)** — each language model is tiny (~135 MB) but requires model-switching for multilingual use

### Speed (All Variants)

| Variant | MacBook M3 | Linux x86 | Raspberry Pi 5 | Whisper equiv. latency |
|---------|-----------|-----------|-----------------|------------------------|
| Tiny (34M) | **50 ms** ¹ | 69 ms | 237 ms | 289 ms (Whisper Tiny) |
| Small (123M) | **148 ms** ¹ | 165 ms | 527 ms | 1,940 ms (Whisper Small) |
| Medium (245M) | **258 ms** ¹ | 269 ms | 802 ms | 11,286 ms (Whisper Large-v3) |

**Moonshine Medium is 43.7x faster than Whisper Large-v3** while achieving comparable/better WER.

### Architecture Advantages

- **Variable-length input** — no fixed 30s window
- **Streaming encoder** — sliding-window self-attention with cached states
- **Bounded TTFT** — time-to-first-token independent of utterance length
- **80ms algorithmic lookahead** — minimal latency for streaming

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| Fastest model family overall (34ms Tiny) | No Chinese benchmark data |
| Native streaming support | Per-language models (not unified) |
| Variable-length input (no 30s waste) | Chinese model exists but unproven |
| Best speed/accuracy for English | Very limited language coverage |
| Extremely small (27M Tiny) | No emotion/event detection |

---

## 5. Qwen3-ASR (Qwen Team) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [QwenLM/Qwen3-ASR](https://github.com/QwenLM/Qwen3-ASR) |
| Architecture | LLM-based AED (Qwen3 foundation) |
| Languages | **30 languages** |
| Chinese Dialects | **22 dialects** |
| Streaming | ✅ Full support |
| Max Audio Length | 1200 seconds (20 min) |
| License | Apache 2.0 |
| sherpa-onnx | ✅ First-class support |

### Variant Comparison

| Variant | Params | Size (INT8) | Chinese | English | Dialects | Speed (macOS M4) | Speed (Win CPU RTF) | sherpa-onnx |
|---------|--------|-------------|---------|---------|----------|-------------------|---------------------|-------------|
| **0.6B** | 600M | ~937 MB | ✅ 5.97% WenetSpeech | ✅ 2.11% LibriSpeech | 22 | 8.0 tok/s | **1.214** ² | ✅ Available |
| 1.7B | 1,700M | ~3 GB (est.) | ✅ 4.97% WenetSpeech | ✅ 1.63% LibriSpeech | 22 | — | — | ❌ Not yet |

### 0.6B Variant (Recommended)

The **best accuracy-efficiency trade-off** in the sub-1B class:

| Attribute | Value |
|-----------|-------|
| Components | conv_frontend 42MB + encoder 174MB + decoder 721MB (INT8) |
| Total Size | ~937 MB |
| Languages | 30: Chinese, English, Cantonese, Arabic, German, French, Spanish, Portuguese, Indonesian, Italian, Korean, Russian, Thai, Vietnamese, Japanese, Turkish, Hindi, Malay, Dutch, Swedish, Danish, Finnish, Polish, Czech, Filipino, Persian, Greek, Hungarian, Macedonian, Romanian |
| Chinese Dialects | 22: Anhui, Dongbei, Fujian, Gansu, Guizhou, Hebei, Henan, Hubei, Hunan, Jiangxi, Ningxia, Shandong, Shaanxi, Shanxi, Sichuan, Tianjin, Yunnan, Zhejiang, Cantonese (HK + GD), Wu, Minnan |

### Accuracy (from official paper, WER/CER%)

> **Note**: Whisper Large-v3 numbers in these tables are from the Qwen3-ASR paper (arXiv:2601.21337), which may use different decoding settings than the Whisper paper. See Section 2 for Whisper's own reported numbers.

**English benchmarks:**

| Dataset | Qwen3-ASR 0.6B | Qwen3-ASR 1.7B | Whisper Large-v3 | GPT-4o-Transcribe | SenseVoice Small |
|---------|-----------------|-----------------|------------------|--------------------|-------------------|
| LibriSpeech clean | 2.11% | **1.63%** | 1.51% | **1.39%** | ~2.40% |
| LibriSpeech other | 4.55% | **3.38%** | 3.97% | 3.75% | — |
| GigaSpeech | 8.88% | **8.45%** | 9.76% | 25.50% | — |
| Common Voice (en) | 9.92% | **7.39%** | 9.90% | 9.08% | ~4.20% |
| Fleurs (en) | 4.39% | 3.35% | 4.08% | **2.40%** | — |
| TED-LIUM | **3.85%** | 4.50% | 6.84% | 7.69% | — |
| VoxPopuli | **9.96%** | **9.15%** | 12.05% | 10.29% | — |
| MLS (en) | 6.00% | **4.58%** | 4.87% | 5.12% | — |

**Chinese benchmarks (CER%):**

| Dataset | Qwen3-ASR 0.6B | Qwen3-ASR 1.7B | Whisper Large-v3 | GPT-4o-Transcribe | SenseVoice Small |
|---------|-----------------|-----------------|------------------|--------------------|-------------------|
| WenetSpeech (net) | 5.97% | **4.97%** | 9.86% | 15.30% | ~2.10% |
| WenetSpeech (meeting) | 6.88% | **5.88%** | 19.11% | 32.27% | — |
| AISHELL-2 | 3.15% | **2.71%** | 5.06% | 4.24% | ~1.75% |
| SpeechIO | 3.44% | **2.88%** | 7.56% | 12.86% | — |
| Fleurs (zh) | 2.88% | **2.41%** | 4.09% | 2.44% | — |
| Common Voice (zh) | 6.89% | **5.35%** | 12.91% | 6.32% | — |

**Chinese dialect benchmarks (CER%):**

| Dataset | Qwen3-ASR 0.6B | Qwen3-ASR 1.7B | Whisper Large-v3 | Doubao-ASR |
|---------|-----------------|-----------------|------------------|------------|
| KeSpeech (dialects) | 7.08% | **5.10%** | 28.79% | 5.27% |
| Fleurs (Cantonese) | 5.79% | **3.98%** | 9.18% | 4.98% |
| CV (Cantonese) | 9.50% | **7.57%** | 16.23% | 13.20% |
| CV (zh-TW) | 5.59% | **3.77%** | 7.84% | 4.06% |
| WenetSpeech-Yue (short) | 7.54% | **5.82%** | 32.26% | 9.74% |
| WenetSpeech-Chuan (easy) | 13.92% | 11.99% | 14.35% | **11.40%** |

**Key insight**: Qwen3-ASR 0.6B **significantly outperforms Whisper Large-v3** on every Chinese benchmark while being 1.5x smaller. However, WenetSpeech CER numbers (5.97%) appear higher than SenseVoice's claimed 2.10% — this likely reflects different test subsets or evaluation methodology. Direct comparison requires testing on the same dataset.

### 1.7B Variant

Matches or exceeds commercial APIs. Not yet in sherpa-onnx but available via the official Qwen3-ASR repo. Best for server-side deployment where accuracy trumps speed.

### Speed (0.6B Variant)

| Platform | Metric | Value |
|----------|--------|-------|
| Windows CPU | RTF (offline) | **1.214** ² |
| Windows CPU | RTF (with VAD) | 1.488 ² |
| macOS M4 | tok/s | 8.0 |
| Android (Exynos 9820) | tok/s | 3.65 |
| Server (concurrency 128) | throughput | 2000 sec/sec |
| Server (concurrency 128) | TTFT | 92 ms avg |

> ² Qwen3-ASR on Windows CPU (Intel i5-1035G1) is slow — RTF 1.214 means slower than real-time. On macOS M4 (ANE-accelerated) it achieves 8.0 tok/s. Windows benchmark from [VoicePing (Feb 2026)](https://voiceping.net/en/blog/research-offline-speech-transcription-benchmark/).

### Special Features

- ✅ Timestamp support via Qwen3-ForcedAligner
- ✅ Lyrics / rap / singing recognition
- ✅ 22 Chinese dialects (widest coverage)
- ✅ 30 languages in a single model
- ✅ Streaming with VAD

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| Best language coverage (30 langs) | Largest model (~1 GB INT8) |
| Best dialect coverage (22 dialects) | Slower than SenseVoice on all platforms |
| Best accuracy for open-source (CER 4.97% WenetSpeech, WER 1.63% LibriSpeech) | Slower on Apple platforms (no CoreML opt) |
| Single model, no per-language switching | Higher memory footprint |
| LLM foundation (future-proof) | Newer, less battle-tested than Whisper |
| Apache 2.0 license | — |

---

## 6. Paraformer (Alibaba DAMO) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | Alibaba DAMO Academy / FunASR |
| Architecture | Non-autoregressive Paraformer |
| License | MIT |
| sherpa-onnx | ✅ Full support |

### Variant Comparison

| Variant | Mode | Languages | Chinese Dialects | Streaming | Size (INT8 est.) |
|---------|------|-----------|-----------------|-----------|-------------------|
| **Bilingual** | **Streaming** | **2 (zh/en)** | **Mandarin** | **✅ Native** | **~226 MB** |
| Bilingual | Offline | 2 (zh/en) | Mandarin | ❌ | ~825 MB (FP32) |
| Trilingual | Streaming | 3 (zh/en/yue) | Mandarin + Cantonese | ✅ Native | ~226 MB |
| Small | Offline | 2 (zh/en) | Mandarin | ❌ | ~200 MB |
| Chinese (2023) | Offline | 2 (zh/en) | Mandarin | ❌ | ~400 MB |
| Sichuan dialect | Offline | 1 (Sichuan) | Sichuan | ❌ | ~400 MB |

### Streaming Bilingual (Recommended)

| Attribute | Value |
|-----------|-------|
| Parameters | ~220M encoder + ~220M decoder |
| Model Size (INT8) | encoder 158 MB + decoder 68 MB = **~226 MB** total |
| Model Size (FP32) | encoder 607 MB + decoder 218 MB = **~825 MB** total |
| Languages | 2: Chinese, English |
| Chinese Dialects | Mandarin primarily; ModelScope has Henan, Tianjin, Sichuan variants |
| Streaming | ✅ **Native streaming** (non-autoregressive, chunk-based) |

### Accuracy

Historically the **gold standard for Chinese ASR**:
- AISHELL-1 CER: ~2-3% (from FunASR benchmarks)
- Best-in-class for Mandarin Chinese among non-LLM models
- Streaming variant maintains high accuracy with chunk-based inference

### Speed

| Variant | Platform | Metric | Value |
|---------|----------|--------|-------|
| Streaming INT8 | CPU (general) | RTF | **0.15** |
| Streaming FP32 | CPU (general) | RTF | 0.21 |
| Offline | CPU (general) | RTF | ~0.10 (faster, no streaming overhead) |

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| Best Chinese CER (non-LLM class) | Only 2 languages (zh/en) |
| Native streaming (non-autoregressive) | Limited dialect support |
| Moderate size (~226 MB INT8) | No timestamps |
| Well-proven in production | English accuracy not competitive |
| Fast (non-autoregressive decode) | — |

---

## 7. FireRedAsr (FireRedTeam) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [FireRedTeam/FireRedASR](https://github.com/FireRedTeam/FireRedASR) |
| Languages | 2: Chinese, English |
| Chinese Dialects | **20+ dialects**: Cantonese (HK + GD), Sichuan, Shanghai, Wu, Minnan, Anhui, Fujian, Gansu, Guizhou, Hebei, Henan, Hubei, Hunan, Jiangxi, Liaoning, Ningxia, Shaanxi, Shanxi, Shandong, Tianjin, Yunnan |
| Streaming | ✅ |
| License | Apache 2.0 |
| sherpa-onnx | ✅ Full support |

### Variant Comparison

| Variant | Architecture | Params (est.) | Size (INT8) | Dialects | Speed/Accuracy Trade-off |
|---------|-------------|----------------|-------------|----------|--------------------------|
| **v2 CTC** | **CTC** | **~220M** | **~1.9 GB** | **20+** | **Faster, good accuracy** |
| v2 AED | Attention Encoder-Decoder | ~220M | ~1.9 GB | 20+ | Slower, best accuracy |
| v1 Large | AED | — | — | 3 (Mandarin + Sichuan + Henan) | Legacy |

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| 20+ Chinese dialects | Very large model (1.9 GB INT8) |
| CTC variant for speed, AED for accuracy | Only 2 languages |
| Industrial-grade Chinese STT | No benchmark data vs Qwen3-ASR |
| Two variants to trade speed/accuracy | — |

---

## 8. Dolphin (DataoceanAI) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [DataoceanAI/Dolphin](https://github.com/DataoceanAI/Dolphin) |
| Architecture | CTC |
| Languages | **40 Eastern languages** |
| Chinese Dialects | **22 dialects** |
| Streaming | ✅ |
| License | Apache 2.0 |
| sherpa-onnx | ✅ Full support |

### Variant Comparison

| Variant | Size (INT8) | Size (FP32) | Use Case |
|---------|-------------|-------------|----------|
| Small | ~200 MB (est.) | — | Low-resource devices, speed priority |
| **Base** | **~365 MB** | **~700 MB** | **Standard use, accuracy priority** |

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| Broadest Eastern language coverage (40) | CTC architecture may sacrifice accuracy |
| 22 Chinese dialects | No benchmark data available |
| Two size variants | — |

---

## 9. FunASR Nano (Alibaba)

### Profile

| Attribute | Value |
|-----------|-------|
| Source | FunAudioLLM/Fun-ASR-Nano-2512 (ModelScope) |
| Model Size | ~200-600 MB |
| Languages | Chinese + English + Japanese |
| Chinese Dialects | 7 Chinese dialects |
| Special | Lyrics / rap / singing recognition |
| License | Apache 2.0 |
| sherpa-onnx | ✅ Listed |

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| Best dialect recognition quality | High VRAM usage (6 GB+) |
| Lyrics / singing support | Long audio handling issues |
| — | Least optimized for deployment |

---

## 10. Parakeet TDT (NVIDIA) — Family Overview

### Profile

| Attribute | Value |
|-----------|-------|
| Source | [NVIDIA NeMo / Parakeet TDT](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) |
| Architecture | FastConformer encoder + Token-and-Duration Transducer (TDT) decoder |
| Paper | [Canary-1B-v2 & Parakeet-TDT-0.6B-v3 (arXiv:2509.14128)](https://arxiv.org/html/2509.14128v1) |
| TDT Paper | [Efficient Sequence Transduction by Jointly Predicting Tokens and Durations (arXiv:2304.06795)](https://arxiv.org/abs/2304.06795) |
| Chinese Dialects | ❌ None |
| Streaming | ❌ Offline only (simulated streaming via VAD) |
| License | CC-BY-4.0 |
| sherpa-onnx | ✅ v2, v3, CTC 110M, CTC 0.6B (JA) |

### How TDT Works

Token-and-Duration Transducer extends RNN-Transducer by jointly predicting **both a token and its duration** (number of input frames to skip):

| Aspect | Conventional Transducer | TDT |
|--------|------------------------|-----|
| Blank emissions | Frame-by-frame, many blanks | Joint token + duration prediction |
| Frame processing | Processes every frame | **Skips up to 4 frames** via duration head |
| Inference speed | Baseline | Up to 2.82x faster |
| Accuracy | Baseline | Better or equal |

Combined with **FastConformer** (8x aggressive subsampling via depthwise-separable convolutions), Parakeet TDT achieves extremely high throughput.

### Variant Comparison

| Variant | Params | Languages | LibriSpeech Clean WER | HF ASR Avg WER | RTFx (A100) | ONNX INT8 Size |
|---------|--------|-----------|----------------------|----------------|-------------|----------------|
| **TDT 0.6B v2** | 600M | EN only | **1.69%** | **6.05%** | **3,386** | ~640 MB |
| **TDT 0.6B v3** | 600M | 25 EU | 1.93% | 6.34% | 3,333 | ~640 MB |
| **TDT 1.1B** | 1,100M | EN only | **1.39%** | 7.02% | 2,391 | ~1.2 GB (est.) |
| TDT CTC 110M | 110M | EN only | — | — | — | **126 MB** |
| TDT CTC 0.6B (JA) | 600M | JA only | — | — | — | 625 MB |

### sherpa-onnx Model Files (Exact Sizes)

| Variant | Encoder | Decoder | Joiner | Total |
|---------|---------|---------|--------|-------|
| **TDT 0.6B v2 (INT8)** | 622 MB | 6.9 MB | 1.7 MB | ~640 MB |
| **TDT 0.6B v3 (INT8)** | 622 MB | 12 MB | 6.1 MB | ~640 MB |
| TDT CTC 110M (INT8) | — | — | — | 126 MB |

### Accuracy — English (HF Open ASR Leaderboard)

> All WER numbers in this section use greedy decoding without external LM, from the HF Open ASR Leaderboard. Cross-model comparisons are fair here because the evaluation methodology is identical.

| Dataset | TDT 0.6B v2 | TDT 0.6B v3 | TDT 1.1B | Whisper Large-v3 |
|---------|-------------|-------------|----------|------------------|
| LibriSpeech clean | **1.69%** | 1.93% | **1.39%** | 2.70% |
| LibriSpeech other | **3.19%** | 3.59% | — | — |
| GigaSpeech | 9.74% | **9.59%** | 9.55% | — |
| TEDLIUM-v3 | 3.38% | **2.75%** | 3.42% | — |
| VoxPopuli | **5.95%** | 6.14% | **3.56%** | — |
| Earnings-22 | **11.15%** | 11.42% | 14.65% | — |
| AMI | 11.16% | 11.31% | 15.90% | — |
| SPGISpeech | **2.17%** | 3.97% | 2.62% | — |
| **Avg (9 datasets)** | **6.05%** | 6.34% | 7.02% | 7.44% |

> **TDT 0.6B v2 beats Whisper Large-v3 on English WER** (6.05% vs 7.44% avg) while being 2.6x smaller.

### Accuracy — Multilingual (v3, 25 European Languages, FLEURS WER%)

| Language | WER | Language | WER | Language | WER |
|----------|-----|----------|-----|----------|-----|
| Spanish | 3.45% | French | 5.15% | Italian | 3.00% |
| German | 5.04% | Portuguese | 4.76% | Russian | 5.51% |
| Dutch | 7.48% | Polish | 7.31% | Ukrainian | 6.79% |
| English | 4.85% | Swedish | 15.08% | Finnish | 13.21% |
| Danish | 18.41% | Greek | 20.70% | **Avg (24 langs)** | **11.97%** |

### Speed — Apple Silicon (macOS M4)

| Engine | Model | tok/s | Notes |
|--------|-------|-------|-------|
| **CoreML (FluidAudio)** | **TDT 0.6B v3** | **171.6** | **Fastest in this entire report** |
| sherpa-onnx INT8 | Moonshine Tiny (34M) | 92.2 | 2nd fastest |
| sherpa-onnx INT8 | SenseVoice Small (234M) | 27.4 | — |
| ONNX Runtime INT8 | Qwen3-ASR 0.6B | 8.0 | — |
| CoreML (WhisperKit) | Whisper Turbo (809M) | 1.9 | — |

> **Parakeet TDT v3 is 1.9x faster than Moonshine Tiny** on macOS M4 despite being 22x larger (600M vs 27M params). This is due to: (1) TDT frame-skipping reduces decoder steps, (2) CoreML/ANE is highly optimized for FastConformer encoder, (3) 8x subsampling shortens encoder output.

### Speed — Windows CPU (Intel i5-1035G1, sherpa-onnx)

| Model | tok/s | RTF |
|-------|-------|-----|
| **TDT 0.6B v2** | **17.8** | **0.113** |

### Speed — Android (Samsung Galaxy S10, sherpa-onnx)

| Model | tok/s |
|-------|-------|
| **TDT 0.6B v3** | **20.41** |

### Speed — RK3588 (Cortex-A76, TDT 0.6B v2 INT8)

| Threads | RTF |
|---------|-----|
| 1 | 0.220 |
| 2 | 0.142 |
| 3 | 0.118 |
| 4 | **0.088** |

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| **Fastest model on Apple Silicon** (171.6 tok/s CoreML) | **No Chinese support** — English + 25 EU only |
| Beats Whisper Large-v3 on English WER | CC-BY-4.0 license (not MIT/Apache) |
| Extremely high GPU throughput (RTFx >3,300) | No native streaming — offline only |
| Moderate size (~640 MB INT8) | CoreML optimization is 3rd-party (FluidAudio), not NVIDIA official |
| TDT frame-skipping = fewer decoder steps | 1.1B variant is worse on average WER than 0.6B (overfitting?) |
| 25 European languages in v3 | CTC 110M variant is tiny but no benchmarks published |

### Why Not Recommended for AriaType

Parakeet TDT is the **fastest and most accurate English-only STT** in this report. However:

1. **No Chinese support** — zero Chinese language capability across all variants
2. **CoreML speed depends on FluidAudio conversion** — not available via standard sherpa-onnx
3. **CC-BY-4.0 license** — requires attribution (less permissive than MIT/Apache)
4. **No streaming** — offline only, same as Whisper
5. **Potentially useful if**: AriaType adds an English-optimized mode or if NVIDIA releases a Chinese variant

---

## Cross-Platform Speed Comparison

Data from [VoicePing Benchmark (Feb 2026)](https://voiceping.net/en/blog/research-offline-speech-transcription-benchmark/). All models via sherpa-onnx unless noted.

### Windows (Intel i5-1035G1, 8 GB RAM, CPU)

| Model | Params | tok/s | RTF |
|-------|--------|-------|-----|
| Moonshine Tiny | 27M | **50.6** | **0.040** |
| SenseVoice Small | 234M | **47.6** | **0.042** |
| Moonshine Base | 61M | 41.2 | 0.049 |
| Parakeet TDT v2 | 600M | 17.8 | 0.113 |
| Qwen3-ASR 0.6B | 600M | 1.6 | 1.214 |
| Whisper Tiny (sherpa-onnx) | 39M | 27.08 | 0.07 |
| Whisper Small (sherpa-onnx) | 244M | 4.70 | 0.41 |
| Whisper Tiny (whisper.cpp) | 39M | 9.5 | 0.211 |

### macOS (MacBook Air M4, 32 GB RAM)

| Model | Params | tok/s |
|-------|--------|-------|
| Parakeet TDT v3 (CoreML) | 600M | **171.6** |
| Moonshine Tiny | 27M | 92.2 |
| SenseVoice Small | 234M | 27.4 |
| Qwen3-ASR 0.6B | 600M | 8.0 |
| Whisper Large-v3 Turbo (CoreML) | 809M | 1.9 |
| Whisper Small (sherpa-onnx est.) | 244M | ~1.2 |

### Android (Samsung Galaxy S10, Exynos 9820)

| Model | Params | tok/s | RTF |
|-------|--------|-------|-----|
| Moonshine Tiny | 27M | **42.55** | **0.05** |
| SenseVoice Small | 234M | 33.62 | 0.06 |
| Whisper Tiny | 39M | 27.08 | 0.07 |
| **Parakeet TDT 0.6B v3** | **600M** | **20.41** | — |
| Whisper Small | 244M | 4.70 | 0.41 |
| Qwen3-ASR 0.6B | 600M | 3.65 | 0.53 |

### Key Finding

sherpa-onnx is **51x faster** than whisper.cpp for the same Whisper model on mobile. This alone justifies migration.

---

## Model Selection Decision Tree

```
AriaType primary use case: Chinese + English voice typing
│
├─ Need maximum accuracy + dialect coverage?
│  └─ ✅ Qwen3-ASR 0.6B
│     30 langs, 22 dialects, best open-source accuracy, ~1 GB
│
├─ Need maximum speed + smallest footprint?
│  └─ ✅ SenseVoice Small
│     5 langs, fastest inference, ~240 MB
│
├─ Need streaming with best Chinese CER?
│  └─ ✅ Paraformer Bilingual (streaming)
│     Native non-autoregressive streaming, best Mandarin CER
│
├─ Need 20+ Chinese dialects specifically?
│  └─ ✅ FireRedAsr v2 or Qwen3-ASR 0.6B
│
├─ Need 99+ language coverage?
│  └─ ✅ Whisper Small (current default)
│     99+ languages, 490 MB, ~2s latency on M3
│     ⚠️ Chinese CER worse than SenseVoice, slow
│
├─ Need best English accuracy regardless of speed?
│  └─ ✅ Whisper Large-v3 (but slow: 11s latency on M3)
│
├─ Need fastest English-only STT?
│  └─ ✅ Parakeet TDT 0.6B v3 (CoreML) or Moonshine v2
│     Parakeet: 171.6 tok/s on M4, best English WER in sub-1B class
│     Moonshine: 92.2 tok/s (Tiny), native streaming, but EN only
│
├─ Need English-only with speculative decoding (future)?
│  └─ ✅ Distil-Whisper large-v3 + Whisper large-v3
│     2x speedup, bitwise identical output to full Whisper
│
└─ Need European language coverage without Chinese?
   └─ ✅ Parakeet TDT 0.6B v3
      25 EU languages, 171.6 tok/s on Apple Silicon, WER 6.34% avg
```

---

## Recommended Strategy for AriaType

### Phase 1: Migrate SenseVoice to sherpa-onnx

Replace the current SenseVoice CLI sidecar with sherpa-onnx's SenseVoice integration. Lowest risk, immediate speed gain, same model.

### Phase 2: Add Qwen3-ASR 0.6B as alternative

Offer users a choice between speed (SenseVoice) and accuracy/coverage (Qwen3-ASR). Qwen3-ASR covers 30 languages + 22 dialects in a single model.

### Phase 3: Evaluate Paraformer streaming

If real-time streaming (边说边出字) becomes a priority, integrate Paraformer bilingual streaming as the streaming engine.

### Not Recommended

| Model | Why Not |
|-------|---------|
| Distil-Whisper | English only — no Chinese support at all. INT8 ONNX sizes (0.97–4.01 GB) are 4–17x larger than SenseVoice. Only useful for speculative decoding acceleration. |
| Parakeet TDT | Fastest model on Apple Silicon (171.6 tok/s) and best English WER (1.69% LibriSpeech), but **no Chinese support**. CC-BY-4.0 license requires attribution. Only useful for English-only mode. |
| Moonshine | No meaningful Chinese advantage over SenseVoice |
| Whisper (keep for compatibility) | Inferior speed and Chinese accuracy vs alternatives |
| FunASR Nano | High VRAM, long-audio issues, not deployment-ready |
| Dolphin | Interesting coverage but unproven accuracy |

---

## Sources

1. [FunAudioLLM/SenseVoice](https://github.com/FunAudioLLM/SenseVoice) — SenseVoice official repo and benchmarks
2. [openai/whisper](https://github.com/openai/whisper) — Whisper official repo and paper
3. [huggingface/distil-whisper](https://github.com/huggingface/distil-whisper) — Distil-Whisper official repo
4. [Distil-Whisper Paper (arXiv:2311.00430)](https://arxiv.org/abs/2311.00430) — Distillation method, all variant benchmarks
5. [Distil-Whisper large-v3.5 Model Card](https://huggingface.co/distil-whisper/distil-large-v3.5) — v3.5 benchmarks, training details
6. [Distil-Whisper ONNX exports for sherpa-onnx](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/whisper/export-onnx.html) — Pre-exported ONNX models
7. [usefulsensors/moonshine](https://github.com/usefulsensors/moonshine) — Moonshine official repo
8. [Moonshine v2 Paper (arXiv:2602.12241v1)](https://arxiv.org/html/2602.12241v1) — Moonshine architecture and benchmarks
9. [Qwen3-ASR Technical Report (arXiv:2601.21337)](https://arxiv.org/html/2601.21337v1) — Qwen3-ASR benchmarks
10. [FireRedTeam/FireRedASR](https://github.com/FireRedTeam/FireRedASR) — FireRedAsr official repo
11. [DataoceanAI/Dolphin](https://github.com/DataoceanAI/Dolphin) — Dolphin official repo
12. [sherpa-onnx Pre-trained Models](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/index.html) — All model listings
13. [VoicePing Offline STT Benchmark (Feb 2026)](https://voiceping.net/en/blog/research-offline-speech-transcription-benchmark/) — Cross-platform speed benchmarks
14. [Northflank STT Model Comparison (Jan 2026)](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks) — Accuracy and latency benchmarks
15. [FunASR-Nano vs Paraformer vs SenseVoice (yunpan.plus)](https://yunpan.plus/t/5087-1-1) — Dialect and speed comparison
16. [NVIDIA Parakeet TDT Paper (arXiv:2509.14128)](https://arxiv.org/html/2509.14128v1) — Parakeet TDT 0.6B v2/v3 and Canary-1B-v2
17. [TDT Architecture Paper (arXiv:2304.06795)](https://arxiv.org/abs/2304.06795) — Token-and-Duration Transducer method
18. [FastConformer Paper (arXiv:2305.05084)](https://arxiv.org/abs/2305.05084) — FastConformer encoder architecture
19. [HF: nvidia/parakeet-tdt-0.6b-v2](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2) — v2 model card and benchmarks
20. [HF: nvidia/parakeet-tdt-0.6b-v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) — v3 model card and multilingual benchmarks
21. [HF: nvidia/parakeet-tdt-1.1b](https://huggingface.co/nvidia/parakeet-tdt-1.1b) — 1.1B model card and benchmarks
