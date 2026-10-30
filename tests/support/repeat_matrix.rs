/// Two partitions with 128 distinct patterns each, but many repeats within
/// small subtrees. Fixed tips keep every character variable and all states
/// observed, including at the edge of each enumeration.
pub fn matrix() -> String {
    let mut text = String::from(
        "#NEXUS BEGIN DATA; DIMENSIONS NTAX=12 NCHAR=256; FORMAT SYMBOLS=\"012\"; MATRIX\n",
    );
    for tip in 0..12 {
        text.push_str(&format!("T{tip} "));
        for states in [2u32, 3] {
            for pattern in 0..128u32 {
                let state = if tip >= 9 {
                    (tip - 9) % states
                } else {
                    (pattern / states.pow(tip)) % states
                };
                text.push(char::from_digit(state, 10).unwrap());
            }
        }
        text.push('\n');
    }
    text.push_str("; END;");
    text
}
