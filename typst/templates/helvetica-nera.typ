// ═══════════════════════════════════════════════════════════════════════════
// Template: helvetica-nera — Swiss minimal
// ═══════════════════════════════════════════════════════════════════════════

#import "../shared/invoice.typ": sample-data, compute-totals, resolve-totals, star-mark, hairline, money
#import "../shared/components.typ": *

#let d = sample-data
#let totals = resolve-totals(d)

#let theme = (
  ink: rgb("#0A0A0A"),
  paper: rgb("#FAFAF7"),
  accent: rgb("#1A1A1A"),
  mute: rgb("#6E6E6A"),
  hair: rgb("#E4E4E0"),
  dim: rgb("#C4C4C0"),
  display-font: ("Helvetica Neue", "Helvetica", "Arial", "New Computer Modern"),
  body-font: ("Helvetica Neue", "Helvetica", "Arial", "New Computer Modern"),
  mono-font: ("Menlo", "DejaVu Sans Mono"),
  label-style: "upper",
  tax-zero: "dash",
  totals-variant: "minimal",
  hide-zero-tax: true,
  qr-style: "square",
  margin: (top: 22mm, bottom: 22mm, left: 22mm, right: 22mm),
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
#set par(leading: 6.2pt, spacing: 6pt)

// ─── HERO ────────────────────────────────────────────────────────────────
#grid(
  columns: (1fr, 1fr),
  column-gutter: 12mm,
  align(top + left)[
    #if "logo" in d.issuer and d.issuer.logo != none [
      #image(d.issuer.logo, height: 14mm)
      #v(2mm)
    ]
    #grid(
      columns: (auto, auto),
      column-gutter: 7pt,
      align: (horizon, horizon),
      star-mark(size: 14pt, color: theme.ink),
      text(font: theme.display-font, size: 11pt, weight: 500, tracking: 0.2pt)[#d.issuer.name],
    )
    #if "legal-name" in d.issuer and d.issuer.legal-name != none and d.issuer.legal-name != d.issuer.name [
      #v(3pt)
      #text(size: 8.5pt, fill: theme.mute)[#d.issuer.legal-name]
    ]
  ],
  align(top + right)[
    #fit-size(
      (34pt, 30pt, 26pt),
      80mm,
      s => text(font: theme.display-font, size: s, weight: 300, tracking: -1.2pt, fill: theme.ink)[#d.invoice.title],
    )
    #v(-4pt)
    #fit-size(
      (10pt, 9.5pt, 9pt),
      80mm,
      s => text(font: theme.body-font, size: s, weight: 500, tracking: 0.2pt, fill: theme.ink)[№ #d.invoice.number],
    )
    #if d.invoice.kind == "credit-note" and d.invoice.credits-number != none [
      #v(-1mm)
      #text(size: 8.5pt, fill: theme.mute)[re: Invoice № #d.invoice.credits-number]
    ]
  ],
)

#v(mm-sp.l)
#hairline(color: theme.accent, weight: 0.4pt)
#v(mm-sp.s)

// ─── PARTIES (Bill to | Bill from) ───────────────────────────────────────
#grid(
  columns: (1fr, 1fr),
  column-gutter: 14mm,
  party-block(d.client, theme, label-text: "Bill to"),
  party-block(d.issuer, theme, label-text: "Bill from", show-name: false),
)

#v(mm-sp.s)
#line(length: 100%, stroke: 0.3pt + theme.hair)
#v(mm-sp.xs)

// ─── META STRIP ──────────────────────────────────────────────────────────
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

// Line-level discount summary — Swiss-minimal hairline strip.
#let discounted-items = d.items.filter(it => "discount" in it and it.discount != none)
#if discounted-items.len() > 0 [
  #v(sp.xs)
  #align(right)[
    #box(width: 82mm)[
      #for it in discounted-items [
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.l,
          align: (left, right),
          text(size: 8pt, fill: theme.mute)[
            #it.description —
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
#tax-totals(totals, theme, currency-symbol: d.invoice.symbol, width: 82mm, tax-label: d.invoice.tax-label, total-label: if d.invoice.kind == "credit-note" { "Total credit" } else { none })

// Invoice-level discount row.
#if "discount" in totals and totals.discount != none [
  #v(sp.s)
  #align(right)[
    #box(width: 82mm)[
      #grid(
        columns: (1fr, auto),
        column-gutter: sp.l,
        align: (left, right),
        text(size: 9.5pt, fill: theme.mute)[#if totals.discount-label != none { totals.discount-label } else { "Discount" }],
        text(size: 9.5pt, fill: theme.accent)[−#money(totals.discount, symbol: d.invoice.symbol)],
      )
    ]
  ]
]

// Reverse-charge callout — minimal hairline box matching Swiss aesthetic.
#if d.invoice.reverse-charge [
  #v(mm-sp.m)
  #block(width: 100%, inset: 8pt, stroke: 0.3pt + theme.hair, [
    #text(weight: "medium", size: 9pt, fill: theme.ink)[Reverse charge]\
    #text(size: 8pt, fill: theme.mute)[VAT to be accounted for by the recipient under the reverse-charge mechanism.]
  ])
]

// ─── PAYMENT + NOTES ─────────────────────────────────────────────────────
#v(mm-sp.l)
#hairline(color: theme.accent, weight: 0.4pt)
#v(mm-sp.s)
#let payment-label = if d.invoice.kind == "credit-note" { "Payment details" } else { "Pay to" }

#block(breakable: false)[
  #if "qr" in d and d.qr != none {
    grid(
      columns: (1fr, 1fr, auto),
      column-gutter: 10mm,
      payment-block(d.issuer.bank, theme, label-text: payment-label),
      notes-block(d.notes, theme),
      [
        #qr-render(d.qr.modules, size: 24mm, fg: theme.ink, bg: theme.paper, style: theme.qr-style)
        #v(2pt)
        #align(center, text(size: 6.5pt, fill: theme.mute, tracking: 0.8pt)[#upper(d.qr.label)])
      ],
    )
  } else {
    grid(
      columns: (1fr, 1fr),
      column-gutter: 10mm,
      payment-block(d.issuer.bank, theme, label-text: payment-label),
      notes-block(d.notes, theme),
    )
  }
]
