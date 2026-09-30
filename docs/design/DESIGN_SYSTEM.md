# Design System

Character: quiet, precise, professional, fast. Desktop productivity software, not SaaS marketing.

Hierarchy should come from layout/alignment, typography, spacing, separators/surfaces, then restrained color. Use a compact legible type scale; no display-sized dashboard headings. Use a consistent 4px-based spacing scale. Modest radii, few elevation levels, subtle separators instead of excessive shadows. Neutral base colors; accent for focus/action; semantic colors for state. Never rely on color alone. No decorative gradients. Use one professional icon family; no emoji icons.

Tables/lists are first-class. Forms keep visible labels and specific validation. Buttons have primary/secondary/quiet/destructive hierarchy. Motion is short and functional.

## SiteDatum visual refresh

The brand palette is navy `#102440`, action teal `#087f89`, and a restrained orange `#ed8a40`. The canvas is a cool drafting-paper neutral. Orange accents the current-project marker and immediate-work hierarchy; textual reasons and counts remain authoritative. Teal replaces the former generic blue action/selection treatment. The existing logo's teal transition is preserved; decorative surface gradients are not introduced.

Dense registers retain their tables and filters. Home uses one dark, compact, clickable workload strip and open sections with thin rules instead of repeated cards. Typography, tabular numbers, consistent squared corners, and visible focus carry the hierarchy. Shared styles are in `src/design.css`, loaded after structural component styles.

View entrances are 220ms, dialog entrances 200ms, and feedback entrances 180ms. No data-row stagger, ornamental looping motion, or animation dependency is used. Reduced-motion preferences remove all animation and transitions. Dialogs size to content and remain viewport-bounded.

About is a software title sheet using the shared full Brand lockup, the statement "The project record you control", "Created by Francisco Cabrera", and the running executable's version. The page preserves current-project context and is reachable from system navigation and the command palette.

## Appearance modes

SiteDatum supports Light, Dark, and Windows-default appearance. The header carries a compact, accessible Light/Dark switch for frequent changes. Settings provides the authoritative three-choice control with labeled previews and a live description of the effective theme. Choosing the header switch while following Windows deliberately creates an explicit Light or Dark preference; choosing Windows default again resumes live system tracking.

The preference is device-local under `appearance.mode`, defaults safely to `system`, and is applied before React renders to avoid a bright startup flash. Dark mode retains the navy sidebar and brand accents while using deep blue-gray working surfaces, teal action/focus states, orange urgency cues, and readable semantic success/warning/error colors. Both modes preserve the same density, hierarchy, layout, and control behavior.
