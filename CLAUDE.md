# Design Context

When doing any frontend / UI work in this repo, read the design source-of-truth files first:

- **[PRODUCT.md](PRODUCT.md)** — strategic context: register (default `product`; the app is core, the Astro marketing site is `brand`), users, purpose, brand personality, anti-references (not a Discord clone, not crypto/web3, not childish), and accessibility bar (WCAG 2.2 AA + reduced motion).
- **[DESIGN.md](DESIGN.md)** — visual system in Stitch format: the "Lit Room After Dark" North Star, the violet `#7C3AED` palette over near-black tonal layers, Inter + JetBrains Mono, components, and the Do's/Don'ts that enforce the anti-references.
- **[.impeccable/design.json](.impeccable/design.json)** — sidecar with tonal ramps, shadow/motion/breakpoint tokens, and drop-in component snippets (used by `/impeccable live`).
- **[ui-specs/00-design-tokens.md](ui-specs/00-design-tokens.md)** — the per-component token catalog the app specs reference. Live CSS variables are in [frontend/src/app.css](frontend/src/app.css) (note: dark default + `[data-theme='light']` + Tauri vibrancy).

Core rule: violet glow is rationed (the One Glow Rule), depth comes from tonal near-black layers, and every new surface must work in both light and dark themes.
