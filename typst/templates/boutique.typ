// ═══════════════════════════════════════════════════════════════════════════
// Template: boutique — Brand-led chromatic (restrained)
// ═══════════════════════════════════════════════════════════════════════════

#import "../shared/invoice.typ": sample-data, compute-totals, resolve-totals, star-mark, money
#import "../shared/components.typ": *

#let d = sample-data
#let totals = resolve-totals(d)

#let accent = rgb("#3D5D4A")
#let theme = (
  ink: rgb("#1E2420"),
  paper: rgb("#F9F7F2"),
  accent: accent,
  accent-soft: rgb("#E8EFEA"),
  mute: rgb("#706D65"),
  hair: rgb("#E0DDD4"),
  dim: rgb("#C9C5BC"),
  display-font: ("Inter Display", "Inter", "Helvetica Neue"),
  body-font: ("Inter", "Helvetica Neue"),
  mono-font: ("Menlo", "DejaVu Sans Mono"),
  label-style: "upper",
  tax-zero: "dash",
  totals-variant: "soft-fill",
  hide-zero-tax: true,
  qr-style: "dots",
  margin: (top: 0pt, bottom: 22mm, left: 0pt, right: 0pt),
  page-inset-x: 22mm,
)

#let serif = ("Adobe Garamond Pro", "Iowan Old Style", "Baskerville", "Georgia")
#let paper-on-accent = rgb("#F6F3EC")

#show: body => page-shell(theme, d.issuer, d.invoice, body)

#set text(
  font: theme.body-font,
  size: 9.5pt,
  fill: theme.ink,
  lang: "en",
  number-type: "lining",
  number-width: "tabular",
)
#set par(leading: 6pt, spacing: 6pt)

// ─── ACCENT BAND ─────────────────────────────────────────────────────────
#block(width: 100%, height: 38mm, fill: theme.accent, inset: (left: 22mm, right: 22mm, y: 12mm))[
  #set text(fill: paper-on-accent)
  #grid(
    columns: (1.1fr, 1fr),
    align: (left + horizon, right + horizon),
    [
      #if "logo" in d.issuer and d.issuer.logo != none [
        #image(d.issuer.logo, height: 14mm)
        #v(2mm)
      ]
      #grid(
        columns: (auto, auto),
        column-gutter: 10pt,
        align: (horizon, horizon),
        star-mark(size: 16pt, color: paper-on-accent),
        [
          #fit-size(
            (15pt, 13pt, 11.5pt),
            90mm,
            s => text(font: theme.display-font, size: s, weight: 500, tracking: 1pt)[#upper(d.issuer.name)],
          )
          #if "legal-name" in d.issuer and d.issuer.legal-name != none [
            \
            #v(-1pt)
            #text(size: 8.5pt, tracking: 0.2pt)[#d.issuer.legal-name]
          ]
        ],
      )
    ],
    [
      #align(right, text(size: 8.5pt, tracking: 2pt, weight: 500)[#upper(d.invoice.title)])
      #v(-2pt)
      #align(right, fit-size(
        (22pt, 19pt, 16pt, 13pt),
        80mm,
        s => text(font: serif, size: s, style: "italic", weight: 500, tracking: -0.2pt)[№ #d.invoice.number],
      ))
      #if d.invoice.kind == "credit-note" and d.invoice.credits-number != none [
        #align(right, text(size: 8.5pt, style: "italic", tracking: 0.3pt)[re: Invoice № #d.invoice.credits-number])
      ]
    ],
  )
]

// ─── BODY ────────────────────────────────────────────────────────────────
#pad(x: 22mm, top: mm-sp.l, bottom: 0mm)[

#grid(
  columns: (1fr, 1fr),
  column-gutter: 14mm,
  party-block(d.client, theme, label-text: "Bill to"),
  party-block(d.issuer, theme, label-text: "Bill from"),
)

#v(mm-sp.s)
#line(length: 100%, stroke: 0.3pt + theme.hair)
#v(mm-sp.xs)

#let meta-cell(label, value, emphasize: false) = [
  #lbl(theme, label, fill: theme.accent)
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

#line-items-table(d.items, theme, currency-symbol: d.invoice.symbol, tax-label: d.invoice.tax-label)

// Line-level discount summary — soft sage, aligned with totals column.
#let discounted-items = d.items.filter(it => "discount" in it and it.discount != none)
#if discounted-items.len() > 0 [
  #v(sp.xs)
  #align(right)[
    #box(width: 90mm)[
      #for it in discounted-items [
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.l,
          align: (left, right),
          text(size: 8pt, fill: theme.mute)[
            #it.description ·
            #if it.discount-label != none and it.discount-label.starts-with("rate:") {
              "less " + it.discount-label.slice(5) + "%"
            } else { "discount" }
          ],
          text(size: 8pt, fill: theme.mute)[−#money(it.discount, symbol: d.invoice.symbol)],
        )
      ]
    ]
  ]
]

#v(mm-sp.s)
#grid(
  columns: (1fr, auto),
  column-gutter: 0mm,
  [
    #v(sp.m)
    #text(font: serif, size: 11pt, style: "italic", fill: theme.accent)[With thanks — #d.issuer.name.]
  ],
  tax-totals(totals, theme, currency-symbol: d.invoice.symbol, width: 90mm, tax-label: d.invoice.tax-label),
)

// Invoice-level discount row — sits under the soft-fill totals card.
#if "discount" in totals and totals.discount != none [
  #v(sp.s)
  #align(right)[
    #box(width: 90mm)[
      #grid(
        columns: (1fr, auto),
        column-gutter: sp.m,
        align: (left, right),
        text(size: 9.5pt, fill: theme.mute)[#if totals.discount-label != none { totals.discount-label } else { "Discount" }],
        text(size: 9.5pt, fill: theme.accent)[−#money(totals.discount, symbol: d.invoice.symbol)],
      )
    ]
  ]
]

// Reverse-charge callout — hairline sage box matching boutique palette.
#if d.invoice.reverse-charge [
  #v(mm-sp.s)
  #block(width: 100%, inset: 8pt, stroke: 0.3pt + theme.hair, [
    #text(weight: "medium", size: 9pt, fill: theme.accent)[Reverse charge]\
    #text(size: 8pt, fill: theme.mute)[VAT to be accounted for by the recipient under the reverse-charge mechanism.]
  ])
]

#v(mm-sp.m)
#line(length: 100%, stroke: 0.3pt + theme.hair)
#v(mm-sp.s)

#block(breakable: false)[
  #if "qr" in d and d.qr != none {
    grid(
      columns: (1fr, 1fr, auto),
      column-gutter: 8mm,
      payment-block(d.issuer.bank, theme, label-text: "Pay to"),
      notes-block(d.notes, theme, label-text: "Notes"),
      [
        #qr-render(d.qr.modules, size: 24mm, fg: theme.accent, bg: theme.accent-soft, style: theme.qr-style)
        #v(2pt)
        #align(center, text(size: 6pt, fill: theme.mute, tracking: 1pt)[#upper(d.qr.label)])
      ],
    )
  } else {
    grid(
      columns: (1fr, 1fr),
      column-gutter: 10mm,
      payment-block(d.issuer.bank, theme, label-text: "Pay to"),
      notes-block(d.notes, theme, label-text: "Notes"),
    )
  }
]

]
