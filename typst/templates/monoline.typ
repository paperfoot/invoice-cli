// ═══════════════════════════════════════════════════════════════════════════
// Template: monoline — Technical receipt
// ═══════════════════════════════════════════════════════════════════════════

#import "../shared/invoice.typ": sample-data, compute-totals, resolve-totals, money
#import "../shared/components.typ": *

#let d = sample-data
#let totals = resolve-totals(d)

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
    #if "logo" in d.issuer and d.issuer.logo != none [
      #image(d.issuer.logo, height: 10mm)
      #v(2mm)
    ]
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
    #align(right, text(fill: theme.accent, size: 9pt, tracking: 1pt)[\[ #upper(d.invoice.title) \]])
    #v(-3pt)
    #align(right, fit-size(
      (20pt, 17pt, 14pt, 12pt),
      90mm,
      s => text(size: s, weight: 500, tracking: 0.4pt)[\##d.invoice.number],
    ))
    #if d.invoice.kind == "credit-note" and d.invoice.credits-number != none [
      #align(right, text(size: 8.5pt, fill: theme.mute)[\# re: \##d.invoice.credits-number])
    ]
  ],
)

#v(mm-sp.xs)
#line(length: 100%, stroke: 0.5pt + theme.dim)
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

// Line-level discount summary — monospace technical notation.
#let discounted-items = d.items.filter(it => "discount" in it and it.discount != none)
#if discounted-items.len() > 0 [
  #v(sp.xs)
  #align(right)[
    #box(width: 82mm)[
      #for it in discounted-items [
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.m,
          align: (left, right),
          text(font: theme.mono-font, size: 8pt, fill: theme.mute)[
            #it.description :
            #if it.discount-label != none and it.discount-label.starts-with("rate:") {
              "-" + it.discount-label.slice(5) + "%"
            } else { "less" }
          ],
          text(font: theme.mono-font, size: 8pt, fill: theme.mute)[-#money(it.discount, symbol: d.invoice.symbol)],
        )
      ]
    ]
  ]
]

#v(mm-sp.s)
#tax-totals(totals, theme, currency-symbol: d.invoice.symbol, width: 82mm, tax-label: d.invoice.tax-label)

// Invoice-level discount row.
#if "discount" in totals and totals.discount != none [
  #v(sp.s)
  #align(right)[
    #box(width: 82mm)[
      #grid(
        columns: (1fr, auto),
        column-gutter: sp.m,
        align: (left, right),
        text(font: theme.mono-font, size: 9.5pt, fill: theme.mute)[#if totals.discount-label != none { totals.discount-label } else { "Discount" }],
        text(font: theme.mono-font, size: 9.5pt, fill: theme.accent)[-#money(totals.discount, symbol: d.invoice.symbol)],
      )
    ]
  ]
]

// Reverse-charge callout — technical hairline box.
#if d.invoice.reverse-charge [
  #v(mm-sp.s)
  #block(width: 100%, inset: 8pt, stroke: 0.3pt + theme.dim, [
    #text(font: theme.mono-font, weight: "medium", size: 9pt, fill: theme.ink)[\[ REVERSE CHARGE \]]\
    #text(font: theme.mono-font, size: 8pt, fill: theme.mute)[VAT to be accounted for by the recipient under the reverse-charge mechanism.]
  ])
]

// ─── PAYMENT + NOTES ─────────────────────────────────────────────────────
#v(mm-sp.m)
#line(length: 100%, stroke: 0.5pt + theme.dim)
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
