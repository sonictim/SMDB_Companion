use crate::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Basic {
    config: NodeConfig,
    match_criteria: SelectableList,
    match_null: bool,
    preservation_order: OrderPanel,
}

impl Basic {
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }
    pub fn render(&mut self, ui: &mut egui::Ui, db: &Database) {
        ui.checkbox(&mut self.config.enabled, "Basic Duplicate Search");
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label("Duplicate Match Criteria: ");
        });
        if self.match_criteria.get().is_empty() {
            self.config.enabled = false;
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(light_red_text("Add Match Criteria to Enable Search").size(14.0));
            });
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                button(ui, "Restore Defaults", || {
                    self.match_criteria.set(vec![
                        "Filename".to_owned(),
                        "Duration".to_owned(),
                        "Channels".to_owned(),
                    ])
                });
            });
            empty_line(ui);
        } else {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                self.match_criteria.render(ui, 3, "Match Criteria", true);
            });
        }
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(RichText::new("Add:"));
            self.match_criteria.add_combo_box(ui, &db.columns);

            button(ui, "Remove Selected", || {
                self.match_criteria.remove_selected();
            });
        });

        if ui.input(|i| i.modifiers.alt) {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label("Unmatched Records: ");
                ui.radio_value(&mut self.match_null, false, "Ignore");
                ui.radio_value(&mut self.match_null, true, "Process Together");
            });
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct OrderPanel {
    pub list: Vec<PreservationLogic>,
    #[serde(skip)]
    pub sel_line: Option<usize>,
    pub column: String,
    pub operator: OrderOperator,
    #[serde(skip)]
    pub input: String,
}

impl OrderPanel {
    pub fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>) {
        ui.heading(RichText::new("Duplicate Filename Preservation Priority").strong());
        ui.label("or... How to decide which file to keep when duplicates are found");
        ui.label("Entries at the top of list take precedence to those below");
        empty_line(ui);
        // ui.separator();
        if let Some(db) = db {
            self.top_toolbar(ui, &db.columns);
        } else {
            ui.label(light_red_text("Open DB to enable ADD NEW"));
        }
        empty_line(ui);
        ui.separator();

        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                // Determine the width available for the scrollable area
                let width = ui.available_width();
                let height = ui.available_height();

                // Create a vertical scrollable area with a specified width and height
                egui::ScrollArea::vertical()
                    .max_height(height - 55.0) // Set the maximum height of the scroll area
                    .show(ui, |ui| {
                        // Create a container with the desired width and height
                        ui.horizontal(|ui| {
                            ui.set_min_width(width);

                            // Create a grid layout within the scrollable area
                            egui::Grid::new("Order Grid")
                                .spacing([20.0, 8.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for (index, line) in self.list.iter_mut().enumerate() {
                                        let checked = self.sel_line == Some(index);
                                        let text: &str = if ui.input(|i| i.modifiers.alt) {
                                            &line.get_sql()
                                        } else {
                                            &line.get_friendly()
                                        };
                                        if ui
                                            .selectable_label(
                                                checked,
                                                RichText::new(format!(
                                                    "{:02} : {}",
                                                    index + 1,
                                                    text
                                                ))
                                                .size(14.0),
                                            )
                                            .clicked()
                                        {
                                            self.sel_line =
                                                if checked { None } else { Some(index) };
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                    });
                ui.separator();
                empty_line(ui);

                self.bottom_toolbar(ui);
            },
        );
    }

    pub fn top_toolbar(&mut self, ui: &mut egui::Ui, db_columns: &[String]) {
        ui.horizontal(|ui| {
            combo_box(ui, "order_column", &mut self.column, db_columns);

            enum_combo_box(ui, &mut self.operator);
            match self.operator {
                OrderOperator::Largest
                | OrderOperator::Smallest
                | OrderOperator::IsEmpty
                | OrderOperator::IsNotEmpty => {}
                _ => {
                    ui.text_edit_singleline(&mut self.input);
                }
            }

            if ui.button("Add Line").clicked {
                match self.operator {
                    OrderOperator::Largest
                    | OrderOperator::Smallest
                    | OrderOperator::IsEmpty
                    | OrderOperator::IsNotEmpty => {
                        self.list.insert(
                            0,
                            PreservationLogic {
                                column: self.column.clone(),
                                operator: self.operator,
                                variable: self.input.clone(),
                            },
                        );
                        self.input.clear();
                    }
                    _ => {
                        if !self.input.is_empty() {
                            self.list.insert(
                                0,
                                PreservationLogic {
                                    column: self.column.clone(),
                                    operator: self.operator,
                                    variable: self.input.clone(),
                                },
                            );
                            self.input.clear();
                        }
                    }
                }
            }
        });
    }

    pub fn bottom_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Move Up").clicked() {
                if let Some(index) = self.sel_line {
                    if index > 0 {
                        self.sel_line = Some(index - 1);

                        self.list.swap(index, index - 1);
                    }
                }
            }
            if ui.button("Move Down").clicked() {
                if let Some(index) = self.sel_line {
                    if index < self.list.len() - 1 {
                        self.sel_line = Some(index + 1);

                        self.list.swap(index, index + 1);
                    }
                }
            }
            if ui.button("Remove").clicked() {
                if let Some(index) = self.sel_line {
                    self.list.remove(index);
                    self.sel_line = None;
                }
            }
        });
    }

    pub fn extract_sql(&self) -> Vec<String> {
        self.list.iter().map(|logic| logic.get_sql()).collect()
    }
}
