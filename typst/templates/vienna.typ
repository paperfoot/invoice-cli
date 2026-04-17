// ═══════════════════════════════════════════════════════════════════════════
// Template: vienna-1910 — Statement (Bauhaus-Secession)
// Slab title, terracotta accent band, dark-block totals. Fold marks optional.
// English labels by default (Singapore, UK, general use). Renamed from the
// earlier "RECHNUNG" label to "INVOICE" for non-German markets.
// ═══════════════════════════════════════════════════════════════════════════

#import "../shared/invoice.typ": sample-data, compute-totals, resolve-totals, star-mark, money
#import "../shared/components.typ": *

#let d = sample-data
#let totals = resolve-totals(d)

#let theme = (
  ink: rgb("#1B1B1B"),
  paper: rgb("#F5F0E6"),
  accent: rgb("#C74B39"),
  accent-soft: rgb("#EDE6D6"),
  mute: rgb("#6E685D"),
  hair: rgb("#C7BFAE"),
  dim: rgb("#C7BFAE"),
  display-font: ("Inter Display", "Inter", "Helvetica Neue"),
  body-font: ("Inter", "Helvetica Neue"),
  mono-font: ("Menlo", "DejaVu Sans Mono"),
  label-style: "upper",
  tax-zero: "dash",
  totals-variant: "ledger",
  hide-zero-tax: true,
  qr-style: "square",
  margin: (top: 20mm, bottom: 22mm, left: 20mm, right: 18mm),
  fold-marks: false,
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
#set par(leading: 5.6pt, spacing: 5.6pt)

// ─── HERO ──
#grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  column-gutter: 10mm,
  [
    #if "logo" in d.issuer and d.issuer.logo != none [
      #image(d.issuer.logo, height: 10mm)
      #v(2mm)
    ]
    #grid(
      columns: (auto, auto),
      column-gutter: 8pt,
      align: (horizon, horizon),
      star-mark(size: 13pt, color: theme.accent),
      fit-size(
        (13pt, 11.5pt, 10pt),
        90mm,
        s => text(font: theme.display-font, size: s, weight: 600, tracking: 1.4pt)[#upper(d.issuer.name)],
      ),
    )
    #if "legal-name" in d.issuer and d.issuer.legal-name != none [
      #v(2pt)
      #text(size: 8.5pt, fill: theme.mute, tracking: 0.2pt)[#d.issuer.legal-name]
    ]
  ],
  [
    #fit-size(
      (34pt, 30pt, 26pt),
      90mm,
      s => text(font: theme.display-font, size: s, weight: 800, tracking: -1.4pt, fill: theme.ink)[#upper(d.invoice.title)],
    )
  ],
)

#v(-2mm)
#align(right)[
  #fit-size(
    (9pt, 8.5pt, 8pt),
    110mm,
    s => text(size: s, tracking: 2pt, fill: theme.accent, weight: 500)[№ #d.invoice.number],
  )
  #if d.invoice.kind == "credit-note" and d.invoice.credits-number != none [
    #v(-1mm)
    #text(size: 8.5pt, fill: theme.mute)[re: Invoice № #d.invoice.credits-number]
  ]
]

#v(mm-sp.s)
#rect(width: 100%, height: 3pt, fill: theme.accent, stroke: none)
#v(mm-sp.m)

// ─── PARTIES (Bill to · Bill from) ──
#grid(
  columns: (1fr, 1fr),
  column-gutter: 14mm,
  party-block(d.client, theme, label-text: "Bill to"),
  party-block(d.issuer, theme, label-text: "Bill from"),
)

#v(mm-sp.s)
#line(length: 100%, stroke: 0.3pt + theme.hair)
#v(mm-sp.xs)

// ─── META strip — one body-size row, proportional columns so Terms gets flex ──
// All values render at body size (9.5pt) for consistency. "Due date" is
// marked by accent colour rather than size — hierarchy via tone, not scale.
#let meta-cell(label, value, emphasize: false) = [
  #lbl(theme, label)
  #v(sp.xs)
  #if emphasize {
    text(size: 9.5pt, weight: 600, fill: theme.accent)[#value]
  } else {
    text(size: 9.5pt)[#value]
  }
]

// columns: Date / Due / Terms (flex, absorbs long terms strings) / Currency
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

// ─── ITEMS + TOTALS ──
#line-items-table(d.items, theme, currency-symbol: d.invoice.symbol, tax-label: d.invoice.tax-label)

// Line-level discount summary — rendered below the main items table as a
// minimal muted strip (the shared component doesn't carry discount awareness).
#let discounted-items = d.items.filter(it => "discount" in it and it.discount != none)
#if discounted-items.len() > 0 [
  #v(sp.xs)
  #align(right)[
    #box(width: 96mm)[
      #for it in discounted-items [
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.l,
          align: (left, right),
          text(size: 8pt, fill: theme.mute, style: "italic")[
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

#v(mm-sp.m)
#tax-totals(totals, theme, currency-symbol: d.invoice.symbol, width: 96mm, tax-label: d.invoice.tax-label)

// Invoice-level discount row (when present on totals dict). Rendered as a
// compact line under the ledger totals block.
#if "discount" in totals and totals.discount != none [
  #v(sp.s)
  #align(right)[
    #box(width: 96mm)[
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

// Reverse-charge callout — rendered as a legal-style block before notes.
#if d.invoice.reverse-charge [
  #v(mm-sp.m)
  #block(width: 100%, inset: 8pt, stroke: 0.5pt + theme.hair, [
    #text(weight: "medium", size: 9pt, fill: theme.ink)[Reverse charge]\
    #text(size: 8pt, fill: theme.mute)[VAT to be accounted for by the recipient under the reverse-charge mechanism.]
  ])
]

// ─── PAYMENT + NOTES (Bill from is already up top, no repeat) ──
#v(mm-sp.l)
#line(length: 100%, stroke: 0.4pt + theme.ink)
#v(mm-sp.s)

#if "qr" in d and d.qr != none {
  grid(
    columns: (1fr, 1.3fr, auto),
    column-gutter: 10mm,
    payment-block(d.issuer.bank, theme),
    notes-block(d.notes, theme),
    [
      #qr-render(d.qr.modules, size: 24mm, fg: theme.accent, bg: theme.paper, style: theme.qr-style)
      #v(2pt)
      #align(center, text(size: 6.5pt, fill: theme.mute, tracking: 1pt)[#upper(d.qr.label)])
    ],
  )
} else {
  grid(
    columns: (1fr, 1.3fr),
    column-gutter: 10mm,
    payment-block(d.issuer.bank, theme),
    notes-block(d.notes, theme),
  )
}
