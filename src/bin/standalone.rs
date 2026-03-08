use nih_plug::prelude::*;

use vst_viseme::VstViseme;

fn main() {
    nih_export_standalone::<VstViseme>();
}
