# Home Dashboard Redesign

## Version

- Feature: `home-dashboard`
- Version: `0.1.0`

## Problem

The current desktop home screen shows isolated totals, a single daily count chart, and an engine pie chart. It does not answer the questions a voice-typing user actually cares about after repeated use:

1. Am I using AriaType regularly?
2. How long do I usually speak each time?
3. How much usable text do I get from each capture?
4. Is my workflow leaning toward local privacy or cloud power?
5. Are my habits improving over time or staying flat?

As a result, the home screen feels like a technical status page instead of a personal productivity dashboard.

## Goal

Redesign the desktop home dashboard into a flat, elegant, rounded, black-and-white experience that helps users understand their usage rhythm, capture depth, and engine preference at a glance.

## Design Context

- Product: desktop voice-to-text application
- Primary audience: frequent knowledge workers who dictate notes, prompts, messages, and drafts
- Core job: turn spoken thought into usable text with low friction
- Brand direction: calm, monochrome, rounded, friendly, flat, refined

## First-Principles Model

The home dashboard must answer four user questions in this order:

1. **Momentum**: am I actively using the product?
2. **Depth**: how substantial is each dictation session?
3. **Efficiency**: how much output do I generate relative to my effort?
4. **Preference**: which engines and modes do I trust most?

Anything that does not help answer one of those questions should not occupy prime home-screen real estate.

## Information Architecture

The redesigned dashboard is composed of four layers.

### 1. Hero Summary

Purpose: communicate the user’s current relationship with the product in one glance.

Content:
- greeting
- weekly summary sentence
- total lifetime captures
- current streak
- total dictated time

### 2. Core Habit Metrics

Purpose: show the four strongest indicators of practical usage quality.

Metrics:
- total captures
- average speaking duration per capture
- average output volume per capture
- polish adoption rate

Notes:
- “average output volume” must work across languages
- use a unified word-or-character approximation rather than whitespace-only word counting

### 3. Rhythm Trend Chart

Purpose: reveal whether usage is becoming more frequent, deeper, or lighter.

Chart: 30-day multi-series line chart

Curves:
1. `captures` — number of dictation sessions per day
2. `avg speaking duration` — average speaking time per capture on that day
3. `avg output volume` — average output units per capture on that day

Meaning:
- `captures` shows frequency
- `avg speaking duration` shows session depth
- `avg output volume` shows session output density

Visual rules:
- monochrome palette only
- solid black for primary series
- medium gray for second series
- dashed or lighter gray for third series
- tooltip must explain all three series clearly

### 4. Preference / Behavior Panels

Purpose: help the user understand how they use the product.

Panels:
- activity profile
  - active days
  - longest streak
  - local/cloud split
- engine preference
  - ranked engine list by usage share
  - each row shows session count and average STT processing time

## Data Contract

### Dashboard Summary

Required fields:
- `total_count`
- `today_count`
- `total_chars`
- `total_audio_ms`
- `total_output_units`
- `avg_stt_ms`
- `avg_audio_ms`
- `avg_output_units`
- `local_count`
- `cloud_count`
- `polish_count`
- `active_days`
- `current_streak_days`
- `longest_streak_days`
- `last_7_days_count`
- `last_7_days_audio_ms`
- `last_7_days_output_units`

### Daily Trend Point

Required fields:
- `date`
- `count`
- `audio_ms`
- `output_units`

### Engine Usage Row

Required fields:
- `engine`
- `count`
- `avg_stt_ms`

## Empty State

If the user has no history yet:
- show refined demo data so the layout still teaches the product
- clearly mark the state as sample data
- avoid dead blank containers

## Visual Direction

- Black/white/neutrals only
- Flat surfaces instead of glossy effects
- Large rounded corners
- Sparse, editorial spacing
- Minimal icon usage
- No pie chart
- No rainbow accents
- No dense “BI dashboard” look

## Acceptance Criteria

1. Home screen uses a new dashboard hierarchy aligned with the information architecture above.
2. The trend chart displays three series: captures, average speaking duration, and average output volume.
3. Backend exposes real aggregated values for average speaking duration and average output volume per capture.
4. Backend exposes streak and active-day metrics.
5. Engine usage data includes average STT processing time per engine.
6. Empty state remains informative through sample data and explicit labeling.
7. All new user-facing copy is internationalized.
8. Frontend and backend tests cover the added aggregation logic and the new dashboard rendering contract.

## BDD Scenarios

### Scenario: populated history shows habit dashboard

- Given a user has transcription history across multiple days and engines
- When the dashboard loads
- Then the home screen shows real totals, streak metrics, habit metrics, a 30-day three-series trend chart, and ranked engine usage

### Scenario: multilingual output still produces meaningful average output metric

- Given transcription history contains both whitespace-delimited and CJK text
- When the dashboard summary is computed
- Then the average output volume metric is non-zero and derived from a cross-language output-unit approximation

### Scenario: empty history shows guided sample dashboard

- Given a user has no transcription history
- When the dashboard loads
- Then the screen renders sample dashboard data and an explicit sample-data notice

## Verification

- Rust unit tests for dashboard aggregation and cross-day streak logic
- Frontend component test for the redesigned dashboard sections
- Desktop frontend build
- Desktop Rust tests
- i18n validation
