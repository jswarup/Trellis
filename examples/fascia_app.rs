// examples/fascia_app.rs
//
// Standalone runnable example demonstrating the complete Fascia desktop application shell.
//
// Run with:
//   cargo run -- -e Fascia
// or:
//   cargo run --example fascia_app
fn main() -> iced::Result {
    segue::fascia::run_app()
}
