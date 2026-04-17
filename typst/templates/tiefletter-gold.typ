// ═══════════════════════════════════════════════════════════════════════════
// Template: tiefletter-gold — Editorial luxury (restrained)
// ═══════════════════════════════════════════════════════════════════════════

#import "../shared/invoice.typ": sample-data, compute-totals, star-mark
#import "../shared/components.typ": *

#let d = sample-data
#let totals = compute-totals(d.items)

#let theme = (
  ink: rgb("#1A1512"),
  paper: rgb("#FBF9F4"),
  accent: rgb("#8B6F3A"),
  accent-soft: rgb("#F1EBDD"),
  mute: rgb("#74695E"),
  hair: rgb("#D9CFB8"),
  dim: rgb("#C7BFAE"),
  display-font: ("Playfair Display", "Didot", "Georgia"),
  body-font: ("Adobe Garamond Pro", "Iowan Old Style", "Baskerville", "Georgia"),
  mono-font: ("Menlo", "DejaVu Sans Mono"),
  label-style: "smallcaps",
  tax-zero: "dash",
  totals-variant: "framed-gold",
  hide-zero-tax: true,
  qr-style: "rounded",
  margin: (top: 24mm, bottom: 22mm, left: 24mm, right: 24mm),
)

#show: body => page-shell(theme, d.issuer, d.invoice, body)

#set text(
  font: theme.body-font,
  size: 9.5pt,
  fill: theme.ink,
  lang: "en",
  number-type: "lining",
  number-width: "tabular",
)
#set par(leading: 6.4pt, spacing: 6.8pt, justify: false)

// ─── HERO ────────────────────────────────────────────────────────────────
#grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  column-gutter: 10mm,
  [
    #grid(
      columns: (auto, auto),
      column-gutter: 8pt,
      align: (horizon, horizon),
      star-mark(size: 12pt, color: theme.accent),
      fit-size(
        (14pt, 12pt, 10.5pt),
        90mm,
        s => text(font: theme.body-font, size: s, weight: 600, fill: theme.ink)[#d.issuer.name],
      ),
    )
    #if "legal-name" in d.issuer and d.issuer.legal-name != none [
      #v(2pt)
      #text(size: 8.5pt, style: "italic", fill: theme.mute)[#d.issuer.legal-name]
    ]
  ],
  [
    #align(right, text(font: theme.display-font, size: 26pt, weight: 400, style: "italic", fill: theme.ink)[Invoice])
    #v(-1pt)
    #align(right, fit-size(
      (10pt, 9.5pt, 9pt),
      80mm,
      s => text(font: theme.body-font, size: s, tracking: 0.8pt, fill: theme.accent)[N\u{00BA} #d.invoice.number],
    ))
  ],
)

#v(mm-sp.s)
#line(length: 100%, stroke: 0.4pt + theme.accent)
#v(mm-sp.s)

// ─── PARTIES ─────────────────────────────────────────────────────────────
#grid(
  columns: (1fr, 1fr),
  column-gutter: 14mm,
  party-block(d.client, theme, label-text: "Bill to"),
  party-block(d.issuer, theme, label-text: "Bill from"),
)

#v(mm-sp.s)
#line(length: 100%, stroke: 0.3pt + theme.hair)
#v(mm-sp.xs)

// ─── META ────────────────────────────────────────────────────────────────
#let meta-cell(label, value, emphasize: false) = [
  #lbl(theme, label)
  #v(sp.xs)
  #if emphasize {
    text(size: 9.5pt, weight: 600, fill: theme.accent)[#value]
  } else {
    text(size: 9.5pt)[#value]
  }
]

#grid(
  columns: (auto, auto, 1fr, auto),
  column-gutter: 10mm,
  align: (left + top, left + top, left + top, left + top),
  meta-cell("Invoice date", d.invoice.issue-date),
  meta-cell("Due date",     d.invoice.due-date, emphasize: true),
  meta-cell("Terms",        d.invoice.terms),
  meta-cell("Currency",     d.invoice.currency),
)

#v(mm-sp.m)

// ─── ITEMS + TOTALS ──────────────────────────────────────────────────────
#line-items-table(d.items, theme, currency-symbol: d.invoice.symbol, tax-label: d.invoice.tax-label)
#v(mm-sp.s)
#tax-totals(totals, theme, currency-symbol: d.invoice.symbol, width: 90mm, tax-label: d.invoice.tax-label)

// ─── PAYMENT + NOTES ─────────────────────────────────────────────────────
#v(mm-sp.m)
#block(breakable: false)[
  #if "qr" in d and d.qr != none {
    grid(
      columns: (1fr, 1fr, auto),
      column-gutter: 12mm,
      payment-block(d.issuer.bank, theme, label-text: "Remittance"),
      notes-block(d.notes, theme),
      [
        #qr-render(d.qr.modules, size: 24mm, fg: theme.accent, bg: theme.accent-soft, style: theme.qr-style)
        #v(2pt)
        #align(center, text(font: theme.body-font, size: 7pt, style: "italic", fill: theme.mute, tracking: 0.5pt)[#d.qr.label])
      ],
    )
  } else {
    grid(
      columns: (1fr, 1fr),
      column-gutter: 12mm,
      payment-block(d.issuer.bank, theme, label-text: "Remittance"),
      notes-block(d.notes, theme),
    )
  }
]
