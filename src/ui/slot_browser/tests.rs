use super::*;

#[test]
fn elapsed_reads_as_a_duration() {
    assert_eq!(format_elapsed(0.0), "0:00");
    assert_eq!(format_elapsed(65.0), "1:05");
    assert_eq!(format_elapsed(3600.0), "1:00:00");
    assert_eq!(format_elapsed(3725.0), "1:02:05");
}

/// A negative or absurd elapsed time must not panic the menu — it comes out
/// of a save file, which is the one input this project does not author.
#[test]
fn a_nonsense_elapsed_time_still_renders() {
    assert_eq!(format_elapsed(-10.0), "0:00");
    assert!(!format_elapsed(f32::MAX).is_empty());
}

#[test]
fn mission_ids_render_as_words() {
    assert_eq!(humanize("the_iron_siege"), "The Iron Siege");
    assert_eq!(humanize("skirmish"), "Skirmish");
    assert_eq!(humanize(""), "");
}
