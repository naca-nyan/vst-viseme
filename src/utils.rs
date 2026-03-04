const NOTES: [&'static str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub fn note_friendly_name(note: &u8) -> String {
    let octave = (note / 12) as i8 - 2;
    let note_num = note % 12;
    let note = NOTES[note_num as usize];
    format!("{note}{octave}")
}

#[test]
fn test_note_friendly_name() {
    assert_eq!(note_friendly_name(&0), "C-2");
    assert_eq!(note_friendly_name(&12), "C-1");
    assert_eq!(note_friendly_name(&60), "C3");
    assert_eq!(note_friendly_name(&68), "A3");
    assert_eq!(note_friendly_name(&127), "G8");
}
