# Website Homepage Redesign

## Version

- Feature: `website-homepage`
- Version: `0.1.0`

## Problem

The current website homepage is readable but too narrow in both layout and message. It behaves like a stacked marketing page:

1. the visual hierarchy is flat after the hero
2. the content repeats product claims instead of clarifying product behavior
3. the page does not feel visibly connected to the desktop product design system
4. the homepage does not show a calm, trustworthy interaction model for a privacy-first desktop tool

As a result, the page communicates "marketing site" more strongly than "carefully designed desktop product".

## Goal

Redesign the homepage into a calm, clear, system-driven entry point that:

- presents AriaType as a desktop voice input tool first
- follows the desktop product's neutral tokens, rounded surfaces, and restrained interaction language
- minimizes decorative iconography and SVG usage
- explains the product through structure, copy, and state presentation rather than visual noise

## Design Context

- Product: localized static-export website for a desktop voice typing app
- Audience: people evaluating whether AriaType is trustworthy, local-first, and practical for daily work
- Constraints:
  - must remain compatible with Next.js static export
  - must keep English and Chinese content in sync
  - should align with desktop tokens already defined in `apps/desktop/src/index.css`
- Tone: plain, calm, editorial, desktop-native, privacy-first

## First-Principles Model

The homepage must answer four user questions in this order:

1. what is this product for?
2. how does it fit into my existing desktop workflow?
3. why does it feel safer and calmer than noisy cloud-first alternatives?
4. what should I do next if I want to try it?

Anything that does not support one of those questions should not occupy prime homepage space.

## Information Architecture

The redesigned homepage is composed of five layers.

### 1. Hero With Product Preview

Purpose: define the product clearly and show a quiet interaction model.

Content:
- concise eyebrow
- single strong headline
- short supporting paragraph
- primary download CTA
- secondary source/GitHub CTA
- a preview card that shows recording-state progression and core desktop settings

### 2. Principle Cards

Purpose: explain the product in three compact ideas instead of a long feature dump.

Content:
- cursor-first workflow
- privacy-first defaults
- desktop-native design language

### 3. Workflow Steps

Purpose: show the real operating model in a simple sequence.

Content:
- trigger
- speak
- insert at cursor

### 4. Control Surface Summary

Purpose: connect homepage messaging to real desktop controls.

Content:
- voice engine selection
- optional text polish
- hotkey and permissions
- concise product facts / availability notes

### 5. Closing Download Block

Purpose: end the page with a clear, low-friction next step.

Content:
- short value summary
- download CTA
- minimal requirement note

## Visual Direction

- inherit the desktop neutral palette and radius scale
- rely on typography, spacing, borders, and grouped surfaces instead of illustration-heavy marketing
- keep icon and SVG usage minimal and functional
- use restrained motion only for reveal timing and state emphasis
- preserve a light, open reading experience with calm contrast

## Acceptance Criteria

1. Homepage uses a new layout with a wider editorial grid and grouped card surfaces.
2. Homepage visually aligns with desktop tokens for color, corner radius, borders, and button style.
3. Homepage uses minimal decorative iconography; content hierarchy is carried mainly by text, spacing, and surfaces.
4. Hero includes a product preview card that communicates calm state transitions.
5. Homepage copy is rewritten for clarity and added to both `en` and `zh` locale files.
6. Primary CTA remains a download path for the desktop app.
7. Website build succeeds.
8. Built output can be served locally and opened in a browser context for visual verification.

## BDD Scenarios

### Scenario: first-time visitor understands product quickly

- Given a visitor lands on the homepage
- When the page renders
- Then the visitor sees what AriaType is, how it works, why it is local-first, and how to download it without reading a long feature wall

### Scenario: homepage feels aligned with desktop product

- Given a user has already seen the desktop app
- When they view the homepage
- Then the site uses the same neutral palette, rounded surfaces, restrained controls, and quiet visual language

### Scenario: localized homepage remains complete

- Given the site is rendered in English or Chinese
- When the homepage loads
- Then each redesigned section has complete translated copy with no missing keys

## Verification

- Website build
- i18n key integrity for the new homepage copy
- Local static or dev-server browser check
- Visual inspection of the rendered homepage in browser context
