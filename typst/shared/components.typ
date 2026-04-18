// ═══════════════════════════════════════════════════════════════════════════
// invoice-cli — Shared component library
//
// Components handle STRUCTURE, OVERFLOW, PAGINATION, SPACING.
// Templates handle AESTHETIC (palette, fonts, hero composition).
//
// Theme dict (all keys optional — components read with defaults):
//   Colours:        ink, paper, accent, accent-soft, mute, hair, dim
//   Fonts:          display-font, body-font, mono-font
//   Labels:         label-style : "upper" | "smallcaps" | "mono-tag"
//   Tax behaviour:  tax-zero    : "dash" | "percent" | "hide"
//                   hide-zero-tax : bool  (suppress 0% lines in totals)
//   Totals look:    totals-variant : "minimal" | "dark-block" | "soft-fill"
//                                   | "framed-gold" | "signal-bar"
//   Page:           margin, fold-marks, pagination-strip, compact-strip
// ═══════════════════════════════════════════════════════════════════════════

#import "invoice.typ": money

// ─── Spacing scale ─────────────────────────────────────────────────────────
// One source of truth for vertical rhythm. Keeps sections feeling natural.
#let sp = (
  xxs:  2pt,
  xs:   4pt,
  s:    6pt,
  m:    10pt,
  l:    16pt,
  xl:   24pt,
)
// mm scale for page-level separations
#let mm-sp = (
  xs:  3mm,
  s:   5mm,
  m:   8mm,
  l:   12mm,
  xl:  16mm,
)

// ─── Theme helper ──────────────────────────────────────────────────────────
#let th(theme, key, default) = if key in theme { theme.at(key) } else { default }

// ─── Dynamic text-size fitting via measure() ───────────────────────────────
#let fit-size(sizes, max-width, make) = context {
  let chosen = sizes.last()
  for s in sizes {
    if measure(make(s)).width <= max-width {
      chosen = s
      break
    }
  }
  make(chosen)
}

// ─── Label rendering respects theme.label-style ────────────────────────────
#let lbl(theme, txt, size: 7.5pt, tracking: 1.2pt, fill: none, weight: 500) = {
  let style = th(theme, "label-style", "upper")
  let f = if fill == none { th(theme, "mute", rgb("#666")) } else { fill }
  if style == "smallcaps" {
    text(
      font: th(theme, "display-font", ("Inter",)),
      size: size + 0.3pt, tracking: tracking + 0.2pt, fill: f, weight: weight,
    )[#upper(txt)]
  } else if style == "mono-tag" {
    text(font: th(theme, "mono-font", ("Menlo",)), size: size - 0.5pt, tracking: tracking - 0.3pt, fill: f, weight: weight)[#upper(txt)]
  } else {
    text(size: size, tracking: tracking, fill: f, weight: weight)[#upper(txt)]
  }
}

// ─── Party block — handles optional fields cleanly ─────────────────────────
// Strict type scale across ALL templates — 3 sizes only:
//   11pt  — party heading (bold, display font)
//   9.5pt — body (everything else: addresses, amounts, dates, bank details,
//                 attn, tax-id, legal name, notes, item descriptions)
//   7.5pt — labels (upper tracked)
//
// Hierarchy comes from WEIGHT (bold vs regular) and COLOUR (ink vs mute),
// never from size. Size only varies for headings and labels, nowhere else.
#let party-block(party, theme, label-text: "To") = {
  let mute = th(theme, "mute", rgb("#666"))
  let display = th(theme, "display-font", ("Inter",))
  let has(k) = k in party and party.at(k) != none
  lbl(theme, label-text)
  v(sp.s)
  text(font: display, size: 11pt, weight: 600)[#party.name]
  linebreak()
  if has("legal-name") and party.legal-name != party.name {
    text(size: 9.5pt, fill: mute, style: "italic")[#party.legal-name]
    linebreak()
  }
  if has("attn") {
    text(size: 9.5pt, fill: mute)[#party.attn]
    linebreak()
  }
  for line in party.address [#text(size: 9.5pt)[#line]\ ]
  let id-bits = ()
  if has("tax-id")    { id-bits.push(party.tax-id) }
  if has("company-no") { id-bits.push("Co. " + party.company-no) }
  if id-bits.len() > 0 {
    text(size: 9.5pt, fill: mute)[#id-bits.join(" · ")]
  }
}

// ─── Meta block — stacked or inline-grid ──────────────────────────────────
#let meta-block(pairs, theme, layout: "stacked", emphasize: ()) = {
  let mute = th(theme, "mute", rgb("#666"))
  let accent = th(theme, "accent", rgb("#333"))
  if layout == "stacked" {
    for (lab, val) in pairs [
      #lbl(theme, lab)
      #v(sp.xs)
      #if lab in emphasize {
        text(size: 10.5pt, weight: 600, fill: accent)[#val]
      } else {
        text(size: 10pt)[#val]
      }
      #v(sp.m)
    ]
  } else {
    grid(
      columns: (auto, 1fr),
      row-gutter: sp.s,
      column-gutter: sp.m,
      ..pairs.map(((lab, val)) => (
        lbl(theme, lab),
        if lab in emphasize {
          text(size: 10pt, weight: 600, fill: accent)[#val]
        } else {
          text(size: 10pt)[#val]
        },
      )).flatten()
    )
  }
}

// ─── Line items table ──────────────────────────────────────────────────────
#let line-items-table(items, theme, currency-symbol: "S$", tax-label: "Tax") = {
  let mute = th(theme, "mute", rgb("#666"))
  let hair = th(theme, "hair", rgb("#e0e0e0"))
  let accent = th(theme, "accent", rgb("#333"))
  let display = th(theme, "display-font", ("Inter",))
  let zero-style = th(theme, "tax-zero", "dash")

  let fmt-qty(q) = if calc.rem(q, 1) == 0 { str(int(q)) } else { str(q) }
  let fmt-tax(r) = {
    if r == 0.0 {
      if zero-style == "dash" { "—" }
      else if zero-style == "hide" { "" }
      else { "0%" }
    } else {
      str(int(r)) + "%"
    }
  }

  table(
    // Description flexes; numeric columns sized to content. Column-gutter
    // provides inter-column breathing room without per-column inset hacks —
    // ensures description text flushes left AND amount text flushes right.
    columns: (1fr, auto, auto, auto, auto),
    column-gutter: 16pt,
    stroke: none,
    inset: (x: 0pt, y: 9pt),
    align: (col, row) => if col == 0 { left + horizon } else { right + horizon },
    table.header(
      lbl(theme, "Description"),
      lbl(theme, "Qty"),
      lbl(theme, "Rate"),
      lbl(theme, tax-label),
      lbl(theme, "Amount"),
    ),
    table.hline(stroke: 0.5pt + accent, y: 1),
    ..items.map(item => (
      block(breakable: false)[
        #text(font: display, size: 9.5pt, weight: 600)[#item.description]\
        #text(size: 9.5pt, fill: mute, style: "italic")[#item.subtitle]
      ],
      text(size: 9.5pt)[#fmt-qty(item.qty)],
      text(size: 9.5pt)[#money(item.unit-price, symbol: currency-symbol)],
      text(size: 9.5pt, fill: mute)[#fmt-tax(item.tax-rate)],
      text(size: 9.5pt, weight: 600, font: display)[#money(item.qty * item.unit-price, symbol: currency-symbol)],
    )).flatten(),
    table.hline(stroke: 0.3pt + hair),
  )
}

// ─── Totals internals ─────────────────────────────────────────────────────
#let _totals-rows(totals, theme, currency-symbol, label-fn, tax-label) = {
  let rows = (
    label-fn("Subtotal"),
    text(size: 9.5pt)[#money(totals.subtotal, symbol: currency-symbol)],
  )
  for tl in totals.tax-lines {
    let hide-zero = th(theme, "hide-zero-tax", false)
    if tl.amount == 0.0 and hide-zero { continue }
    let rate-txt = if tl.rate == 0 { "zero-rated" } else { "@ " + str(int(tl.rate)) + "%" }
    rows.push(label-fn(tax-label + " " + rate-txt))
    rows.push(text(size: 9.5pt)[#money(tl.amount, symbol: currency-symbol)])
  }
  rows
}

#let _fit-total(total-str, font, weight, max-width, accent: none, sizes: (18pt, 16pt, 15pt, 13pt, 12pt)) = {
  fit-size(
    sizes,
    max-width,
    s => {
      let opts = (size: s, weight: weight, font: font)
      if accent != none { opts.insert("fill", accent) }
      text(..opts, tracking: -0.2pt)[#total-str]
    },
  )
}

#let tax-totals(totals, theme, currency-symbol: "S$", width: 84mm, tax-label: "Tax") = {
  let variant = th(theme, "totals-variant", "minimal")
  let mute    = th(theme, "mute", rgb("#666"))
  let accent  = th(theme, "accent", rgb("#333"))
  let ink     = th(theme, "ink", black)
  let paper   = th(theme, "paper", white)
  let accent-soft = th(theme, "accent-soft", rgb("#f2f2f2"))
  let dim     = th(theme, "dim", rgb("#ccc"))
  let display = th(theme, "display-font", ("Inter",))

  // Tax-row labels (Subtotal / GST @ 9% / …) match body size (9.5pt) so the
  // rhythm across the doc reads as one voice; colour alone distinguishes
  // label from value.
  let label-mut(t) = text(size: 9.5pt, fill: mute)[#t]

  if variant == "minimal" {
    align(right)[
      #box(width: width)[
        #grid(
          columns: (1fr, auto),
          row-gutter: sp.s,
          column-gutter: sp.l,
          align: (left, right),
          .._totals-rows(totals, theme, currency-symbol, label-mut, tax-label),
        )
        #v(sp.m)
        #line(length: 100%, stroke: 0.5pt + accent)
        #v(sp.m)
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.l,
          align: (left + horizon, right + horizon),
          text(size: 11pt, weight: 600)[Total due],
          _fit-total(money(totals.total, symbol: currency-symbol), display, 500, width - 40mm),
        )
      ]
    ]
  } else if variant == "framed-gold" {
    align(right)[
      #box(width: width)[
        #line(length: 100%, stroke: 0.5pt + accent)
        #v(sp.m)
        #grid(
          columns: (1fr, auto),
          row-gutter: sp.s,
          column-gutter: sp.xl,
          align: (left, right),
          .._totals-rows(totals, theme, currency-symbol, t => text(size: 9.5pt, fill: mute, style: "italic")[#t], tax-label),
        )
        #v(sp.m)
        #line(length: 100%, stroke: 0.3pt + accent)
        #v(sp.m)
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.xl,
          align: (left + horizon, right + horizon),
          text(font: display, size: 12pt, style: "italic")[Total payable],
          _fit-total(money(totals.total, symbol: currency-symbol), display, 500, width - 42mm),
        )
        #v(sp.m)
        #line(length: 100%, stroke: 0.5pt + accent)
      ]
    ]
  } else if variant == "signal-bar" {
    align(right)[
      #box(width: width)[
        #grid(
          columns: (1fr, auto),
          row-gutter: sp.xs,
          column-gutter: sp.m,
          align: (left, right),
          .._totals-rows(totals, theme, currency-symbol, label-mut, tax-label),
        )
        #v(sp.s)
        #line(length: 100%, stroke: 0.8pt + accent)
        #v(sp.s)
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.m,
          align: (left + horizon, right + horizon),
          lbl(theme, "Total due →", fill: ink, size: 9pt),
          _fit-total(money(totals.total, symbol: currency-symbol), th(theme, "mono-font", ("Menlo",)), 600, width - 36mm, accent: accent),
        )
      ]
    ]
  } else if variant == "dark-block" {
    // Asymmetric pad: content has left gutter, right flush so amounts align
    // with the line-items-table AMOUNT column (which itself is flush-right).
    align(right)[
      #block(width: width, fill: ink)[
        #pad(left: sp.l, top: sp.l, bottom: sp.l, right: 0pt)[
          #set text(fill: paper)
          #grid(
            columns: (1fr, auto),
            row-gutter: sp.s,
            column-gutter: sp.l,
            align: (left, right),
            .._totals-rows(totals, theme, currency-symbol, t => text(size: 9pt, fill: dim, tracking: 1pt)[#upper(t)], tax-label),
          )
          #v(sp.m)
          #line(length: 100%, stroke: 0.5pt + dim)
          #v(sp.m)
          #grid(
            columns: (1fr, auto),
            column-gutter: sp.m,
            align: (left + horizon, right + horizon),
            text(size: 10pt, fill: paper, tracking: 1.4pt, weight: 500)[#upper("Total due")],
            _fit-total(money(totals.total, symbol: currency-symbol), display, 700, width - 38mm, accent: accent, sizes: (18pt, 16pt, 15pt, 13pt, 12pt)),
          )
        ]
      ]
    ]
  } else if variant == "ledger" {
    // No fill. Hairlines only. Total figure picks up accent colour.
    align(right)[
      #box(width: width)[
        #grid(
          columns: (1fr, auto),
          row-gutter: sp.s,
          column-gutter: sp.l,
          align: (left, right),
          .._totals-rows(totals, theme, currency-symbol, label-mut, tax-label),
        )
        #v(sp.m)
        #line(length: 100%, stroke: 0.8pt + accent)
        #v(sp.m)
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.l,
          align: (left + horizon, right + horizon),
          text(size: 10pt, tracking: 1.4pt, weight: 600)[#upper("Total due")],
          _fit-total(money(totals.total, symbol: currency-symbol), display, 600, width - 40mm, accent: accent, sizes: (18pt, 16pt, 15pt, 13pt, 12pt)),
        )
      ]
    ]
  } else if variant == "stamp-outline" {
    // Outlined accent box around only the total. Tax lines sit quietly above.
    align(right)[
      #box(width: width)[
        #grid(
          columns: (1fr, auto),
          row-gutter: sp.s,
          column-gutter: sp.l,
          align: (left, right),
          .._totals-rows(totals, theme, currency-symbol, label-mut, tax-label),
        )
        #v(sp.m)
        #block(
          width: 100%,
          stroke: 1.2pt + accent,
          inset: sp.m,
        )[
          #grid(
            columns: (1fr, auto),
            column-gutter: sp.l,
            align: (left + horizon, right + horizon),
            text(size: 10pt, tracking: 1.4pt, weight: 600, fill: accent)[#upper("Total due")],
            _fit-total(money(totals.total, symbol: currency-symbol), display, 700, width - 40mm, accent: accent, sizes: (17pt, 15pt, 14pt, 13pt, 12pt)),
          )
        ]
      ]
    ]
  } else if variant == "soft-fill" {
    // Symmetric inset — the soft-green card is a cohesive object and
    // deserves internal breathing room on all sides.
    align(right)[
      #block(width: width, fill: accent-soft, inset: sp.l, radius: 2pt)[
        #grid(
          columns: (1fr, auto),
          row-gutter: sp.s,
          column-gutter: sp.l,
          align: (left, right),
          .._totals-rows(totals, theme, currency-symbol, label-mut, tax-label),
        )
        #v(sp.m)
        #line(length: 100%, stroke: 0.5pt + accent)
        #v(sp.m)
        #grid(
          columns: (1fr, auto),
          column-gutter: sp.m,
          align: (left + horizon, right + horizon),
          text(font: display, size: 12pt, style: "italic", fill: accent)[Total due],
          _fit-total(money(totals.total, symbol: currency-symbol), display, 600, width - 36mm, accent: accent),
        )
      ]
    ]
  }
}

// ─── QR code renderer ─────────────────────────────────────────────────────
// Accepts a boolean matrix (the real QR module grid encoded in Rust using
// the `qrcode` crate) and renders it with themed colours and styled modules.
//
//   modules: array of arrays of bool — outer = rows, inner = columns
//   size:    overall rendered size (square)
//   fg:      module colour
//   bg:      background / quiet-zone colour
//   style:   "square" (default) | "dots" | "rounded"
//
// For placeholder previews (no data) you can still call epc-qr(..) which
// generates a fake-looking pattern — useful when just previewing templates.
#let qr-render(modules, size: 22mm, fg: black, bg: white, style: "square", quiet-zone: 2) = {
  if modules == none or modules.len() == 0 { return }
  let n = modules.len() + quiet-zone * 2
  let cell = size / n
  let finder-size = 7  // modules in each finder pattern (QR standard)

  // Finder pattern positions (top-left corner in module coords, excluding quiet zone)
  let finders = (
    (quiet-zone, quiet-zone),                                    // top-left
    (quiet-zone, quiet-zone + modules.len() - finder-size),      // top-right  (visually top-right of the matrix)
    (quiet-zone + modules.len() - finder-size, quiet-zone),      // bottom-left (visually bottom-left)
  )

  // Is this cell inside a finder pattern?
  let in-finder(r, c) = {
    for (fr, fc) in finders {
      if r >= fr and r < fr + finder-size and c >= fc and c < fc + finder-size {
        return true
      }
    }
    false
  }

  box(width: size, height: size, fill: bg)[
    // Draw data modules
    #for row in range(modules.len()) {
      for col in range(modules.at(row).len()) {
        if modules.at(row).at(col) and not in-finder(row + quiet-zone, col + quiet-zone) {
          let x = (col + quiet-zone) * cell
          let y = (row + quiet-zone) * cell
          if style == "dots" {
            place(top + left, dx: x, dy: y,
              circle(radius: cell * 0.42, fill: fg))
          } else if style == "rounded" {
            place(top + left, dx: x + cell * 0.08, dy: y + cell * 0.08,
              rect(width: cell * 0.84, height: cell * 0.84, radius: cell * 0.25, fill: fg))
          } else {
            place(top + left, dx: x, dy: y,
              rect(width: cell, height: cell, fill: fg))
          }
        }
      }
    }

    // Draw styled finder patterns (3 corners)
    #for (fr, fc) in finders {
      let x = fc * cell
      let y = fr * cell
      let outer = cell * finder-size
      let ring-w = cell
      let inner-outer = outer - ring-w * 2
      let inner = inner-outer * 0.5
      let radius-outer = if style == "dots" or style == "rounded" { cell * 1.6 } else { 0pt }
      let radius-inner = if style == "dots" or style == "rounded" { cell * 0.6 } else { 0pt }
      // Outer filled square (with ring punch-out)
      place(top + left, dx: x, dy: y,
        rect(width: outer, height: outer, radius: radius-outer, fill: fg))
      // Inner background ring
      place(top + left, dx: x + ring-w, dy: y + ring-w,
        rect(width: outer - ring-w * 2, height: outer - ring-w * 2,
             radius: radius-outer * 0.6, fill: bg))
      // Inner solid square
      place(top + left, dx: x + ring-w * 2, dy: y + ring-w * 2,
        rect(width: outer - ring-w * 4, height: outer - ring-w * 4,
             radius: radius-inner, fill: fg))
    }
  ]
}

// ─── EPC-QR placeholder (for template previews without real data) ─────────
#let epc-qr(size: 22mm, fg: black, bg: white) = {
  box(width: size, height: size, fill: bg, stroke: 0.5pt + fg)[
    #place(center + horizon, block(width: size - 4mm, height: size - 4mm, {
      let inner = size - 4mm
      let cells = 10
      let cell = inner / cells
      for row in range(cells) {
        for col in range(cells) {
          let seed = calc.rem(row * 7 + col * 3 + row * col, 5)
          if seed < 2 {
            place(top + left, dx: col * cell, dy: row * cell, rect(width: cell, height: cell, fill: fg))
          }
        }
      }
      for (dx, dy) in ((0mm, 0mm), (inner - 5.5mm, 0mm), (0mm, inner - 5.5mm)) {
        place(top + left, dx: dx, dy: dy, rect(width: 5.5mm, height: 5.5mm, fill: fg))
        place(top + left, dx: dx + 1.1mm, dy: dy + 1.1mm, rect(width: 3.3mm, height: 3.3mm, fill: bg))
        place(top + left, dx: dx + 1.9mm, dy: dy + 1.9mm, rect(width: 1.7mm, height: 1.7mm, fill: fg))
      }
    }))
  ]
}

// ─── Payment block ────────────────────────────────────────────────────────
// Renders bank / payment details as a two-column list. Input: `bank` dict
// with a `lines` array of `{label, value}` rows (from finance-core's
// BankLine::parse_all). Handles every country — SG bank code, UK sort code,
// US ABA routing, EU IBAN, AU BSB — because the caller decides the labels.
// If `bank` is none (issuer has no bank_details set) the block renders
// nothing, letting the template lay out without a crash.
//
// Typography: labels and values share the same 9.5pt size for visual
// consistency. Distinction comes from colour (mute for labels) and font
// family (monospace for values — clean for account numbers, codes, BIC).
// The only smaller text is the block heading ("Pay to") which inherits
// its styling from the `lbl` helper.
#let payment-block(bank, theme, label-text: "Pay to") = {
  if bank == none { return }
  let mute = th(theme, "mute", rgb("#666"))
  let mono = th(theme, "mono-font", ("Menlo",))
  lbl(theme, label-text)
  v(sp.s)
  let cells = ()
  for line in bank.lines {
    if line.label != "" {
      cells.push(text(size: 8.5pt, fill: mute)[#line.label])
      cells.push(text(size: 8.5pt, font: mono)[#line.value])
    } else {
      // Continuation line without a label — span both columns visually.
      cells.push([])
      cells.push(text(size: 8.5pt, font: mono, fill: mute)[#line.value])
    }
  }
  grid(
    columns: (auto, 1fr),
    column-gutter: sp.m,
    row-gutter: sp.xs,
    ..cells,
  )
}

#let notes-block(text-body, theme, label-text: "Notes") = {
  let mute = th(theme, "mute", rgb("#666"))
  lbl(theme, label-text)
  v(sp.s)
  text(size: 8.5pt, fill: mute)[#text-body]
}

// ─── DIN 5008 fold marks ─────────────────────────────────────────────────
#let fold-marks-placement(theme) = {
  let ink = th(theme, "ink", black)
  place(top + left, dx: 6mm, dy: 87mm,   line(length: 3mm, stroke: 0.3pt + ink))
  place(top + left, dx: 6mm, dy: 148.5mm, line(length: 2mm, stroke: 0.2pt + ink))
  place(top + left, dx: 6mm, dy: 192mm,  line(length: 3mm, stroke: 0.3pt + ink))
}

// ─── Compact page-2+ header strip (invoice-no shrink-to-fit) ─────────────
#let compact-strip(theme, issuer, invoice) = {
  let mute = th(theme, "mute", rgb("#666"))
  let hair = th(theme, "hair", rgb("#e0e0e0"))
  let inset-x = th(theme, "page-inset-x", 0mm)
  pad(top: mm-sp.s, bottom: 0mm, x: inset-x)[
    #grid(
      columns: (1fr, auto),
      align: (left + horizon, right + horizon),
      context fit-size(
        (8pt, 7.5pt, 7pt),
        140mm,
        s => text(size: s, fill: mute, tracking: 0.3pt)[#upper(issuer.name) · Invoice #invoice.number],
      ),
      context text(size: 7.5pt, fill: mute, tracking: 0.3pt)[
        Page #here().page() / #counter(page).final().first()
      ],
    )
    #v(sp.xs)
    #line(length: 100%, stroke: 0.3pt + hair)
  ]
}

// ─── Pagination footer — minimal, one line ────────────────────────────────
#let pagination-footer(theme, issuer) = {
  let mute = th(theme, "mute", rgb("#666"))
  let hair = th(theme, "hair", rgb("#e0e0e0"))
  let inset-x = th(theme, "page-inset-x", 0mm)
  let bits = ()
  if "email" in issuer and issuer.email != none { bits.push(issuer.email) }
  if "phone" in issuer and issuer.phone != none { bits.push(issuer.phone) }
  pad(top: 0mm, bottom: mm-sp.s, x: inset-x)[
    #line(length: 100%, stroke: 0.3pt + hair)
    #v(sp.xs)
    #grid(
      columns: (1fr, auto),
      align: (left + horizon, right + horizon),
      text(size: 7pt, fill: mute, tracking: 0.4pt)[#bits.join(" · ")],
      context text(size: 7pt, fill: mute, tracking: 0.4pt)[
        #here().page() / #counter(page).final().first()
      ],
    )
  ]
}

// ─── Page shell ───────────────────────────────────────────────────────────
#let page-shell(theme, issuer, invoice, body) = {
  let use-folds = th(theme, "fold-marks", false)
  let use-compact = th(theme, "compact-strip", auto) != none
  let use-footer = th(theme, "pagination-strip", auto) != none

  set page(
    paper: "a4",
    margin: th(theme, "margin", (top: 22mm, bottom: 22mm, left: 22mm, right: 22mm)),
    fill: th(theme, "paper", white),
    header: if use-compact { context if here().page() > 1 {
      compact-strip(theme, issuer, invoice)
    } },
    footer: if use-footer { pagination-footer(theme, issuer) },
    background: if use-folds { context if here().page() == 1 {
      fold-marks-placement(theme)
    } },
  )
  body
}
