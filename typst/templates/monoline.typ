// ═══════════════════════════════════════════════════════════════════════════
// Template: monoline — Technical receipt
// ═══════════════════════════════════════════════════════════════════════════

#import "../shared/invoice.typ": sample-data, compute-totals
#import "../shared/components.typ": *

#let d = sample-data
#let totals = compute-totals(d.items)

#let theme = (
  ink: rgb("#0F0F0F"),
  paper: rgb("#FAFAFA"),
  accent: rgb("#E8664D"),
  mute: rgb("#6B6B6B"),
  hair: rgb("#D0D0D0"),
  dim: rgb("#B8B8B8"),
  display-font: ("Menlo", "DejaVu Sans Mono"),
  body-font: ("Menlo", "DejaVu Sans Mono"),
  mono-font: ("Menlo", "DejaVu Sans Mono"),
  label-style: "mono-tag",
  tax-zero: "percent",
  totals-variant: "signal-bar",
  hide-zero-tax: true,
  qr-style: "square",
  margin: (top: 20mm, bottom: 20mm, left: 22mm, right: 22mm),
)

#show: body => page-shell(theme, d.issuer, d.invoice, body)

#set text(
  font: theme.body-font,
  size: 9.5pt,
  fill: theme.ink,
  lang: "en",
  number-width: "tabular",
)
#set par(leading: 5.6pt, spacing: 5.6pt)

// ─── HERO ────────────────────────────────────────────────────────────────
#grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  column-gutter: 8mm,
  [
    #fit-size(
      (12pt, 10.5pt, 9pt),
      100mm,
      s => text(size: s, weight: 500, tracking: 1.4pt)[#upper(d.issuer.name)],
    )
    #if "legal-name" in d.issuer and d.issuer.legal-name != none [
      #v(-1pt)
      #text(size: 8.5pt, fill: theme.mute)[#d.issuer.legal-name]
    ]
  ],
  [
    #align(right, text(fill: theme.accent, size: 9pt, tracking: 1pt)[\[ INVOICE \]])
    #v(-3pt)
    #align(right, fit-size(
      (20pt, 17pt, 14pt, 12pt),
      90mm,
      s => text(size: s, weight: 500, tracking: 0.4pt)[\##d.invoice.number],
    ))
  ],
)

#v(mm-sp.xs)
#line(length: 100%, stroke: 0.6pt + theme.dim)
#v(mm-sp.s)

// ─── PARTIES ─────────────────────────────────────────────────────────────
#grid(
  columns: (1fr, 1fr),
  column-gutter: 10mm,
  party-block(d.client, theme, label-text: "Bill to"),
  party-block(d.issuer, theme, label-text: "Bill from"),
)

#v(mm-sp.s)
#line(length: 100%, stroke: 0.3pt + theme.dim)
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

#v(mm-sp.s)
#line(length: 100%, stroke: 0.3pt + theme.dim)
#v(mm-sp.xs)

// ─── ITEMS + TOTALS ──────────────────────────────────────────────────────
#line-items-table(d.items, theme, currency-symbol: d.invoice.symbol, tax-label: d.invoice.tax-label)
#v(mm-sp.s)
#tax-totals(totals, theme, currency-symbol: d.invoice.symbol, width: 82mm, tax-label: d.invoice.tax-label)

// ─── PAYMENT + NOTES ─────────────────────────────────────────────────────
#v(mm-sp.m)
#line(length: 100%, stroke: 0.6pt + theme.dim)
#v(mm-sp.xs)
#block(breakable: false)[
  #if "qr" in d and d.qr != none {
    grid(
      columns: (1.2fr, 1fr, auto),
      column-gutter: 10mm,
      payment-block(d.issuer.bank, theme, label-text: "Pay to"),
      notes-block(d.notes, theme, label-text: "Notes"),
      [
        #qr-render(d.qr.modules, size: 24mm, fg: theme.accent, bg: theme.paper, style: theme.qr-style)
        #v(2pt)
        #align(center, text(font: theme.mono-font, size: 6.5pt, fill: theme.mute, tracking: 0.8pt)[#upper(d.qr.label)])
      ],
    )
  } else {
    grid(
      columns: (1.2fr, 1fr),
      column-gutter: 10mm,
      payment-block(d.issuer.bank, theme, label-text: "Pay to"),
      notes-block(d.notes, theme, label-text: "Notes"),
    )
  }
]
