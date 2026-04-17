// Helper: encode a URL to a QR matrix and print as a Typst 2-D array.
// Usage:  cargo run --example qr_matrix -- "https://example.com/pay/abc"

use invoice_cli::render::encode_qr;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "https://buy.stripe.com/test_ACME-STUDIO-S$28014".to_string());

    let qr = encode_qr(&data).expect("encoding failed");
    println!("#let qr-data = (");
    println!("  size: {},", qr.size);
    println!("  label: \"{}\",", qr.label);
    println!("  modules: (");
    for row in &qr.modules {
        let cells: Vec<&str> = row.iter().map(|b| if *b { "true" } else { "false" }).collect();
        println!("    ({}),", cells.join(", "));
    }
    println!("  ),");
    println!(")");
}
