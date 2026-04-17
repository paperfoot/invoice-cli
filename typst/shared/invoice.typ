// ═══════════════════════════════════════════════════════════════════════════
// invoice-cli — Shared data + formatting helpers (preview build).
//
// This file is COMMITTED to the public repo and must contain only synthetic,
// clearly-fictional placeholder data. Real issuer/client data lives in each
// user's local SQLite DB at ~/.local/share/invoice/ and is never in the repo.
// ═══════════════════════════════════════════════════════════════════════════

#let sample-data = (
  issuer: (
    name: "Acme Studio",
    legal-name: "Acme Studio Pte. Ltd.",
    tagline: none,
    address: (
      "1 Demo Street",
      "Exampleville 00000",
    ),
    email: "hello@acme.example",
    phone: none,
    tax-id: none,
    company-no: none,
    bank: (
      name: "Example Bank",
      iban: "XX00 XXXX 0000 0000 0000",
      bic: "XXXXXX00",
    ),
  ),
  client: (
    name: "Meridian & Co.",
    attn: "Sophie Lin, Head of Marketing",
    address: (
      "401 Madison Avenue, Suite 700",
      "New York, NY 10017",
      "United States",
    ),
    tax-id: "EIN 00-0000000",
  ),
  invoice: (
    number: "2026-0042",
    issue-date: "17 April 2026",
    due-date: "17 May 2026",
    terms: "Net 30",
    currency: "SGD",
    symbol: "S$",
    tax-label: "GST",
  ),
  items: (
    (
      description: "Design engagement",
      subtitle: "Discovery, prototyping, production handover",
      qty: 1.0, unit: "project", unit-price: 8400.0, tax-rate: 9.0,
    ),
    (
      description: "Interface build — web app",
      subtitle: "Twelve screens, component library, deployment",
      qty: 1.0, unit: "project", unit-price: 12600.0, tax-rate: 9.0,
    ),
    (
      description: "Strategy workshop",
      subtitle: "Two-day intensive, HQ",
      qty: 2.0, unit: "day", unit-price: 1800.0, tax-rate: 9.0,
    ),
    (
      description: "Export services — overseas client",
      subtitle: "Zero-rated under Section 21(3) GST Act",
      qty: 1.0, unit: "engagement", unit-price: 1200.0, tax-rate: 0.0,
    ),
  ),
  notes: "Thank you for the trusted work. Please reference the invoice number on payment. Example placeholder — replace in production with your own issuer details.",
)

// ─── Money formatting with thousands separator ─────────────────────────────
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
    let line = item.qty * item.unit-price
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
  )
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
