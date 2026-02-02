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
            Address::Viseme1 => "/avatar/parameters/Viseme1",
            Address::Viseme2 => "/avatar/parameters/Viseme2",
            Address::Viseme3 => "/avatar/parameters/Viseme3",
            Address::Viseme4 => "/avatar/parameters/Viseme4",
            Address::Viseme5 => "/avatar/parameters/Viseme5",
        }
    }
}
