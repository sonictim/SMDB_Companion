#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Tags {
    list: SelectableList,
    config: NodeConfig,
}

impl Tags {
    fn render_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Tag Editor").strong());
        ui.label("Protools Audiosuite Tags use the following format:  -example_");
        ui.label("You can enter any string of text and if it is a match, the file will be marked for removal");
        empty_line(ui);
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.list.render(ui, 6, "tags editor", false);
            if !self.list().is_empty() {
                ui.separator();
            }
            empty_line(ui);
            self.list.add_text_input(ui);

            if !self.list.get().is_empty() && ui.button("Remove Selected Tags").clicked() {
                self.list.remove_selected();
            }
        });
    }
    pub fn list(&self) -> &[String] {
        self.list.get()
    }

    fn render_node(&mut self, ui: &mut egui::Ui) {
        let enabled = !self.list().is_empty();
        let text = enabled_text(
            "Search for Records with AudioSuite Tags in Filename",
            &enabled,
        );
        ui.checkbox(&mut self.config.enabled, text)
            .on_hover_text_at_pointer(
                "Filenames with Common Protools AudioSuite Tags will be marked for removal",
            );

        if enabled {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                if ui.button("Edit Tags List").clicked() {
                    self.my_panel = Panel::Tags
                }
            });
        } else {
            self.config.enabled = false;
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                if ui.button("Add Tags to Enable").clicked() {
                    self.my_panel = Panel::Tags
                }
            });
        }
    }
}
