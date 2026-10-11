include!("shared.rs");
fn main() {
    needs_reader(&ActiveWriter::new(Write));
}
