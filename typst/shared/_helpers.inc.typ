// ═══════════════════════════════════════════════════════════════════════════
// invoice-cli — static helpers shared by all templates.
// This file is bundled into shared/invoice.typ at render time alongside the
// generated sample-data block. Do not move or rename without updating the
// Rust renderer.
// ═══════════════════════════════════════════════════════════════════════════

#let money(amount, symbol: "S$", decimals: 2) = {
  let negative = amount < 0
  let a = if negative { -amount } else { amount }
  let whole = int(a)
  let frac = int(calc.round((a - whole) * calc.pow(10, decimals)))
  let s = str(whole)
  let chars = s.codepoints()
  let n = chars.len()
  let out = ""
  for i in range(n) {
    out += chars.at(i)
    let r = n - i - 1
    if r > 0 and calc.rem(r, 3) == 0 { out += "," }
  }
  let frac-str = str(frac)
  while frac-str.len() < decimals { frac-str = "0" + frac-str }
  let result = symbol + out + "." + frac-str
  if negative { "−" + result } else { result }
}

#let compute-totals(items) = {
  let subtotal = 0.0
  let by-rate = (:)
  for item in items {
    // Prefer the pre-computed `amount` (post-discount, from Rust) when
    // present; else fall back to qty × unit-price for legacy data.
    let line = if "amount" in item and item.amount != none { item.amount } else { item.qty * item.unit-price }
    subtotal += line
    let k = str(item.tax-rate)
    if k in by-rate {
      by-rate.insert(k, by-rate.at(k) + line)
    } else {
      by-rate.insert(k, line)
    }
  }
  let tax-lines = ()
  let tax-total = 0.0
  for (rate, base) in by-rate {
    let r = float(rate)
    let amt = base * r / 100.0
    tax-total += amt
    tax-lines.push((rate: r, base: base, amount: amt))
  }
  (
    subtotal: subtotal,
    tax-lines: tax-lines,
    tax-total: tax-total,
    total: subtotal + tax-total,
    discount: none,
    discount-label: none,
  )
}

// Use Rust-precomputed totals (precision + discount-aware) when available;
// otherwise compute from items. Templates call this rather than compute-totals
// directly so they always see consistent numbers.
#let resolve-totals(d) = {
  if "totals-override" in d and d.totals-override != none {
    d.totals-override
  } else {
    compute-totals(d.items)
  }
}

#let star-mark(size: 14pt, color: black) = {
  let s = size
  box(width: s, height: s, {
    place(top + left, polygon(
      fill: color,
      (s * 0.5, s * 0.0),
      (s * 0.58, s * 0.42),
      (s * 1.0, s * 0.5),
      (s * 0.58, s * 0.58),
      (s * 0.5, s * 1.0),
      (s * 0.42, s * 0.58),
      (s * 0.0, s * 0.5),
      (s * 0.42, s * 0.42),
    ))
  })
}

#let label(txt, size: 7.5pt, tracking: 0.8pt, fill: black, weight: "medium") = {
  text(size: size, weight: weight, fill: fill, tracking: tracking)[#upper(txt)]
}

#let hairline(color: rgb("#1a1a1a"), weight: 0.4pt) = line(length: 100%, stroke: weight + color)

#let tax-profiles = (
  uk:     (label: "VAT",        default-rate: 20.0, currency: "GBP", symbol: "£"),
  us:     (label: "Sales tax",  default-rate: 0.0,  currency: "USD", symbol: "$"),
  sg:     (label: "GST",        default-rate: 9.0,  currency: "SGD", symbol: "S$"),
  eu-de:  (label: "USt",        default-rate: 19.0, currency: "EUR", symbol: "€"),
  custom: (label: "Tax",        default-rate: 0.0,  currency: "",    symbol: ""),
)
