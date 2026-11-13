use nih_plug::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Enum)]
enum Named {
    #[name = "Soft réglage"]
    #[id = "soft"]
    Soft,
    #[name = "Hard setting"]
    #[id = "hard"]
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Enum)]
enum Plain {
    First,
    Second,
}

#[test]
fn named_enum_preserves_names_ids_and_order() {
    assert_eq!(Named::variants(), &["Soft réglage", "Hard setting"]);
    assert_eq!(Named::ids(), Some(["soft", "hard"].as_slice()));
    assert_eq!(Named::Soft.to_index(), 0);
    assert_eq!(Named::Hard.to_index(), 1);
    assert_eq!(Named::from_index(1), Named::Hard);
    assert_eq!(Named::from_index(usize::MAX), Named::Soft);
}

#[test]
fn plain_enum_preserves_default_names_and_optional_ids() {
    assert_eq!(Plain::variants(), &["First", "Second"]);
    assert_eq!(Plain::ids(), None);
    assert_eq!(Plain::from_index(1), Plain::Second);
}
