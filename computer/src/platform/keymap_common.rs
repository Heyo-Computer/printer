pub fn split_chord(combo: &str) -> Vec<&str> {
    combo.split('+').map(str::trim).filter(|s| !s.is_empty()).collect()
}
