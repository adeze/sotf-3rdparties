use crate::Property;

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Edit {
    pub mode: EditMode,
    pub binding: EditBinding,
    pub value: Property,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub enum EditBinding {
    Strong,
    #[default]
    Weak,
    Same,
}

parse_enum! {
    EditBinding,
    (Strong, "strong"),
    (Weak, "weak"),
    (Same, "same"),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub enum EditMode {
    #[default]
    Assign,
    AssignReplace,
    Prepend,
    PrependFirst,
    Append,
    AppendLast,
    Delete,
    DeleteAll,
}

parse_enum! {
    EditMode,
    (Assign, "assign"),
    (AssignReplace, "assign_replace"),
    (Prepend, "prepend"),
    (PrependFirst, "prepend_first"),
    (Append, "append"),
    (AppendLast, "append_last"),
    (Delete, "delete"),
    (DeleteAll, "delete_all"),
}
