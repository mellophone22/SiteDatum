include!("shared.rs");
fn main() {
    needs_reader(&HistoricalReader::new(Read));
    needs_writer(&ActiveWriter::new(Write));
}
