use nih_plug::prelude::*;

#[derive(Enum, PartialEq, Clone)]
pub enum Address {
    Viseme1,
    Viseme2,
    Viseme3,
    Viseme4,
    Viseme5,
}

impl Address {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Viseme1 => "/avatar/parameters/Viseme1",
            Self::Viseme2 => "/avatar/parameters/Viseme2",
            Self::Viseme3 => "/avatar/parameters/Viseme3",
            Self::Viseme4 => "/avatar/parameters/Viseme4",
            Self::Viseme5 => "/avatar/parameters/Viseme5",
        }
    }
}
