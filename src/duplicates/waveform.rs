use crate::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Waveforms {
    pub enabled: bool,
    #[serde(skip)]
    pub config: Node,
}

impl NodeCommon for Waveforms {
    fn config(&mut self) -> &mut Node {
        &mut self.config
    }
    fn render(&mut self, ui: &mut egui::Ui, _: &Database) {
        ui.checkbox(&mut self.enabled, "Search Audio Waveforms for duplicates")
            .on_hover_text_at_pointer("Will Analyze the Audio Content to search for duplicates");
    }
    fn process(&mut self, _: &Database) {
        println!("Waveform not implemented yet");
    }
}
