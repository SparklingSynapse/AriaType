# AriaType Desktop Design Language

AriaType is a quiet, capable voice-to-text utility. Its design should feel
native, immediate, and trustworthy: a tool that stays out of the user's way
while making recording, transcription, correction, and setup feel clear.

This guide defines the product's emotional tone, visual style, interaction
character, and cross-surface design rules. It is intended to be reusable across
systems. It focuses on style, mood, and experience rather than engineering
guidance.

## Design North Star

AriaType should feel like a polished desktop utility with a small moment of
magic. The everyday interface is calm and efficient. The recording experience is
more vivid, tactile, and instantly recognizable.

The design is guided by four qualities:

- Calm confidence: the product feels reliable, focused, and composed.
- Native immediacy: controls respond quickly and feel at home on desktop.
- Soft precision: surfaces are rounded and gentle, but layout remains exact.
- Trust through restraint: privacy, local models, and system status are
  communicated plainly, without fear-based copy or visual drama.

## Emotional Tone

AriaType should reduce the anxiety around dictation. Speaking to a computer can
feel exposed; the interface should make the user feel in control.

The emotional tone is:

- Calm, not sleepy.
- Capable, not technical.
- Friendly, not playful for its own sake.
- Private, not secretive.
- Fast, not frantic.
- Premium, not decorative.

Avoid the emotional language of AI spectacle: glowing sci-fi panels, exaggerated
gradients, robotic metaphors, or claims that feel overconfident. The product
should earn trust by being clear, quiet, and accurate.

## Visual Personality

The primary visual personality is a soft native workspace:

- Neutral backgrounds and panels.
- Rounded geometry.
- Low-contrast structure.
- Sparse semantic color.
- Compact, readable controls.
- Short and purposeful motion.

The interface should not look like a marketing page, analytics dashboard, chat
bot, game, or generic SaaS web app. It should feel like installed software: a
focused utility that can be used many times a day without visual fatigue.

## Core Visual System

### App Workspace

The main workspace is calm and neutral. It should prioritize hierarchy,
scannability, and predictable layout. Pages should have enough spacing to breathe,
but remain dense enough for repeated operational use.

Use the workspace for:

- Settings.
- History.
- Dictionary management.
- Model and runtime configuration.
- Usage summaries.

The workspace should avoid oversized hero sections, decorative backgrounds, and
large promotional copy. The user's task should always be the visual center.

### Floating Recording Surface

The floating recording surface is the most distinctive part of the product. It
should feel compact, high-contrast, and tactile, like a small piece of desktop
hardware.

It may be visually stronger than the rest of the app:

- Dark or deep-toned surface.
- Pill-like silhouette.
- Clear recording and processing states.
- Small, legible controls.
- Subtle animated emphasis during active work.

This surface needs to remain readable above arbitrary desktop content. Contrast
and stability matter more than matching the light workspace.

### Setup and Onboarding

Setup can be warmer than the daily interface, but it should still be practical.
The purpose is to help the user reach a working dictation setup quickly.

Use onboarding to communicate:

- Permissions.
- Recording readiness.
- Model availability.
- First successful transcription.

Illustration may be used here, but it should explain the product state rather
than act as decoration.

### Lists and Data

History, dictionary, and usage surfaces should be list-first and scannable. They
should feel like records the user can trust, not charts competing for attention.

Use data visualization sparingly:

- Show rhythm and trend, not spectacle.
- Prefer muted chart color.
- Keep labels direct.
- Keep status and metadata easy to compare.

## Color Direction

AriaType is neutral-first. Color is used to clarify meaning, not to decorate the
screen.

The default palette should emphasize:

- Warm or balanced neutrals for backgrounds.
- White or near-white panels in light mode.
- Soft charcoal surfaces in dark mode.
- Low-contrast borders and dividers.
- Muted text for descriptions and metadata.
- Strong text only for labels, values, and current state.

Accent color should be local and semantic:

- Green for ready, success, enabled, or complete.
- Amber for caution, missing setup, or pending attention.
- Red for errors and destructive actions.
- Blue, green, or purple for restrained data visualization.

Do not let the product become a one-color theme. Avoid large saturated areas,
decorative gradients, neon effects, and color used only for excitement.

## Typography Direction

Typography should feel system-native and efficient. Use a familiar desktop UI
font direction rather than a branded display typeface for operational surfaces.

The hierarchy should be clear but not loud:

- Page titles are confident and compact.
- Section titles are semibold and easy to scan.
- Body copy is short and plain.
- Metadata is muted and visually secondary.
- Numeric values should align cleanly and remain easy to compare.

Avoid oversized marketing-style typography inside the app. Large expressive type
belongs to promotional material, not to settings, history, or model management.

## Shape and Spacing

AriaType uses generous rounding to make utilitarian surfaces feel softer. The
geometry should be friendly without becoming childish.

Use:

- Rounded panels for grouped information.
- Pill-shaped primary actions and floating controls.
- Soft rectangles for fields and compact settings.
- Circular icon treatments for small status marks.

Spacing should be calm and deliberate:

- Enough padding for touch and readability.
- Clear separation between unrelated groups.
- Compact rows for repeated settings and history.
- Stable dimensions for controls that update dynamically.

Avoid nested card-heavy layouts, cramped grids, sharp visual cuts, and large
empty spaces that make the product feel unfinished.

## Motion and Interaction

Motion should make state changes easier to understand. It should not feel
cinematic.

The motion character is:

- Fast.
- Short.
- Smooth.
- Functional.
- Slightly tactile on the floating recording surface.

Use subtle motion for:

- Opening and closing overlays.
- Active navigation changes.
- Hover and focus feedback.
- Recording, processing, and completion states.
- Small changes in the floating recording surface.

Avoid long transitions, bouncing effects, excessive page movement, or motion
that delays the user's next action.

## Iconography and Illustration

Icons should be thin, calm, and literal. They support recognition; they should
not carry the full meaning of a control without nearby context unless the action
is universally understood.

Use icons for:

- Navigation recognition.
- Small actions.
- Permission and status states.
- Recording controls.

Illustrations should be reserved for setup, empty states, and light explanatory
moments. They should be simple, warm, and product-specific. Avoid mascots,
generic office scenes, abstract blobs, and purely decorative imagery.

## Feedback and Status

The product should make state visible without making the user feel monitored or
interrupted.

Feedback should be:

- Immediate when the user starts or stops recording.
- Clear when a model is downloading, unavailable, or ready.
- Calm when transcription is processing.
- Direct when an error needs action.
- Brief when a task succeeds.

Status copy should describe what happened or what the user can do next. Avoid
long explanations, internal technical language, and alarmist warnings.

## Privacy and Trust

Privacy is a central part of the product's emotional design. Local model
behavior should feel reassuring and concrete, not hidden behind vague claims.

Communicate privacy with:

- Plain language.
- Visible local/remote distinctions where relevant.
- Calm permission explanations.
- Honest model and network status.

Avoid fear-based framing. The design should say, "You are in control," not
"Everything is dangerous."

## Content Voice

AriaType's product language should be concise, factual, and polished.

Use:

- Short labels.
- Specific action verbs.
- Direct descriptions.
- Calm success and error messages.
- Human-readable model and privacy language.

Avoid:

- Hype.
- Buzzwords.
- Long instructional paragraphs.
- Overly clever empty states.
- Internal architecture terms.
- Copy that sounds like an engineer explaining the system.

The best AriaType copy feels like a capable assistant: precise, quiet, and
respectful of the user's time.

## Light and Dark Modes

Light and dark modes should share the same structure, rhythm, and hierarchy.
Dark mode is not a separate personality; it is the same product under lower
ambient light.

In dark mode:

- Preserve the same layout density.
- Keep borders visible but soft.
- Avoid pure black as the only surface color.
- Keep status colors legible without becoming neon.
- Make the floating recording surface distinct without losing contrast.

Do not introduce dark-mode-only decorations, gradients, or layout changes.

## Design Rules

Do:

- Keep the interface calm, efficient, and native-feeling.
- Use color only when it clarifies meaning.
- Let the floating recording surface be the strongest visual moment.
- Make settings and lists predictable.
- Keep text short and action-oriented.
- Use rounded geometry consistently.
- Make privacy and model state understandable without drama.
- Preserve readability above all decorative choices.

Do not:

- Turn the desktop app into a marketing page.
- Use decorative gradients, visual noise, or AI spectacle.
- Make every screen colorful.
- Over-explain features in the interface.
- Hide important state behind vague labels.
- Use charts or illustrations as decoration.
- Let motion slow down common workflows.
- Make the recording surface look like an ordinary settings panel.

## Cross-System Adaptation

When applying this design language in another system, preserve the emotional and
visual intent rather than copying any one platform's exact controls.

The transferable rules are:

- Calm neutral workspace.
- High-contrast compact recording moment.
- Rounded, soft, precise geometry.
- Sparse semantic color.
- Short, purposeful motion.
- Direct privacy and model-state communication.
- Concise product language.

Adapt spacing, control behavior, and platform conventions to the host system,
but keep the product feeling quiet, capable, private, and fast.
