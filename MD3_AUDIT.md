# Material 3 audit status

**Final audit not performed.** A local-audio prototype exists; the full client does not. The attached Material 3
document is the proposed design reference, not evidence of compliance.

Once the full UI exists, audit color tokens, typography, shape, elevation, components,
layout, navigation, motion, accessibility, and theming. Record evidence and a
0–10 score in each category; fix every score below 8 and repeat the audit. Verify
light/dark and three contrast levels, actual text contrast, keyboard/focus
behavior, 48dp targets, reduced motion, rail/compact navigation, and readable
content width. No score is assigned before inspection.

Preliminary prototype inspection: colors, shapes, typography, and motion use
semantic tokens; Roboto Flex is bundled locally with OFL attribution. Tonal
surfaces, 48px button/range targets, visible focus styles, reduced motion, and a
content-width constraint are present. Actual WebKitGTK automation checked
compact/wide sizes without horizontal overflow; see `reports/desktop-spike.png`.
Only transform animates, and document-hidden state pauses polling.

Missing before a final audit: light theme, three contrast levels, album-art dynamic
color/caching/off-thread computation, full navigation and player views, actual
contrast measurements, keyboard and screen-reader checks, and minimized-window
behavior across OSs. No final category scores or compliance pass are assigned.


Account-screen iteration: inspected the actual WebKitGTK screenshot in
`reports/account-desktop.png` and the seven-check UI report. New text fields use
color/shape/type tokens, labels, a password field, 56px input height, and visible
focus. Buttons retain 48px minimum height and wrap on compact widths. API metadata
uses textContent; secret values are not rendered or stored in localStorage.
Compact/wide overflow checks passed. This is still a diagnostic; light/dynamic
schemes, contrast modes, complete navigation, and full keyboard/screen-reader
validation are outstanding. No final audit pass or full-client score is claimed.


Calculated sRGB contrast ratios for the account screen's declared opaque token
pairs: on-surface/surface **14.35:1**, on-surface-variant/surface-container
**9.56:1**, on-primary/primary **7.71:1**, and primary/surface-container **9.56:1**.
These normal text pairs exceed 4.5:1; this calculation does not substitute for a
complete screen-reader, focus, theming, or dynamic-content audit.
