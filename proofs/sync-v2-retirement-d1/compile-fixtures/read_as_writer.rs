include!("shared.rs");
fn main() {
    needs_writer(&HistoricalReader::new(Read));
}
