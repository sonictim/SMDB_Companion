use crate::prelude::*;

// A reusable button component that takes a function (callback) to run when clicked
pub fn button<F>(ui: &mut Ui, label: &str, action: F)
where
    F: FnOnce(),
{
    if ui.button(label).clicked() {
        action(); // Call the passed function when the button is clicked
    }
}
pub fn rt_button<F>(ui: &mut Ui, label: egui::RichText, action: F)
where
    F: FnOnce(),
{
    if ui.button(label).clicked() {
        action(); // Call the passed function when the button is clicked
    }
}

pub fn large_button<F>(ui: &mut Ui, label: &str, action: F)
where
    F: FnOnce(),
{
    if ui
        .add_sized(
            [200.0, 50.0],
            egui::Button::new(RichText::new(label).size(24.0).strong()),
        )
        .clicked()
    {
        action();
    }
}
pub fn large_button2<F>(ui: &mut Ui, label: &str, action: F)
where
    F: FnOnce(),
{
    if ui
        .add_sized(
            [200.0, 50.0],
            egui::Button::new(
                RichText::new(label)
                    .size(24.0)
                    .strong()
                    .color(egui::Color32::from_rgb(255, 100, 100)),
            ), // .fill(egui::Color32::from_rgb(35, 35, 35)),
        )
        .clicked()
    {
        action();
    }
}
// pub fn large_rt_button<F>(ui: &mut Ui, label: egui::RichText, action: F)
// where
//     F: FnOnce(),
// {
//     if ui
//         .add_sized([200.0, 50.0], egui::Button::new(label.size(24.0).strong()))
//         .clicked()
//     {
//         action();
//     }
// }

pub fn red_text(text: &str) -> RichText {
    RichText::new(text)
        .color(egui::Color32::from_rgb(255, 0, 0))
        .strong()
}
pub fn light_red_text(text: &str) -> RichText {
    RichText::new(text)
        .color(egui::Color32::from_rgb(255, 100, 100))
        .strong()
}

pub fn enabled_text(text: &str, enabled: &bool) -> RichText {
    if *enabled {
        RichText::new(text)
    } else {
        RichText::new(text).weak()
    }
}

// pub fn spawn_db<F>(tx: Sender<Database>, action: F)
// where
//     F: FnOnce(),
// {
//     tokio::spawn(async move {
//         let db = open_db().await.unwrap();
//         if let Err(_) = tx.send(db).await {
//             // eprintln!("Failed to send db");
//         }
//     });
// }

pub fn empty_line(ui: &mut Ui) {
    ui.horizontal(|_| {});
}

// pub struct ComboBox {
//     selected: String,
//     // list: Vec<String>,
// }

// impl ComboBox {
//     fn render(&mut self, ui: &mut egui::Ui, label: &str, list: &[String]) {
//         egui::ComboBox::from_id_salt(label)
//             .selected_text(&self.selected)
//             .show_ui(ui, |ui| {
//                 for item in list {
//                     ui.selectable_value(&mut self.selected, item.clone(), item);
//                 }
//             });
//     }
// }

pub fn combo_box(ui: &mut Ui, label: &str, selected: &mut String, list: &[String]) {
    egui::ComboBox::from_id_salt(label)
        .selected_text(selected.clone())
        .show_ui(ui, |ui| {
            for item in list {
                ui.selectable_value(selected, item.clone(), item);
            }
        });
}

pub trait EnumComboBox {
    fn as_str(&self) -> &'static str;
    fn variants() -> &'static [Self]
    where
        Self: Sized;
}

pub fn enum_combo_box<T>(ui: &mut egui::Ui, selected_variant: &mut T, label: &str)
where
    T: EnumComboBox + PartialEq + Copy + 'static, // Ensure T implements EnumComboBox, PartialEq, Copy, and is 'static
{
    egui::ComboBox::from_id_salt(label)
        .selected_text(selected_variant.as_str())
        .show_ui(ui, |ui| {
            for variant in T::variants() {
                ui.selectable_value(selected_variant, *variant, variant.as_str());
            }
        });
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct SelectableList {
    #[serde(skip)]
    pub add: String,
    #[serde(skip)]
    selected: Vec<usize>,
    list: Vec<String>, // Use &str for the list
}

impl SelectableList {
    pub fn set(&mut self, list: Vec<String>) {
        self.list = list;
    }

    pub fn get(&self) -> &[String] {
        &self.list
    }

    pub fn render(&mut self, ui: &mut egui::Ui, columns: usize, label: &str, border: bool) {
        if border {
            self.render_with_border(ui, columns, label);
        } else {
            self.render_grid(ui, columns, label);
        }
    }

    fn render_with_border(&mut self, ui: &mut egui::Ui, columns: usize, label: &str) {
        egui::Frame::none()
            .inner_margin(egui::vec2(8.0, 8.0))
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(2.0);
                        self.render_grid(ui, columns, label);
                        ui.add_space(2.0);
                    });
                });
            });
    }

    fn render_grid(&mut self, ui: &mut egui::Ui, columns: usize, label: &str) {
        egui::Grid::new(label)
            .num_columns(columns)
            .spacing([20.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                for (index, tag) in self.list.iter_mut().enumerate() {
                    let is_selected = self.selected.contains(&index);

                    if ui
                        .selectable_label(is_selected, RichText::new(tag.clone()).size(14.0))
                        .clicked()
                    {
                        if is_selected {
                            self.selected.retain(|&i| i != index);
                        } else {
                            self.selected.push(index);
                        }
                    }

                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }

                if self.list.len() % columns != 0 {
                    ui.end_row();
                }
            });
    }

    pub fn add_combo_box(&mut self, ui: &mut egui::Ui, box_list: &[String]) {
        let filtered_list: Vec<String> = box_list
            .iter()
            .filter(|item| !&self.list.contains(*item))
            .cloned()
            .collect();
        // .retain();
        combo_box(ui, "match criteria", &mut self.add, &filtered_list);
        self.add();
    }

    pub fn add_text_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Add Tag:").clicked() {
                self.add();
            }
            ui.text_edit_singleline(&mut self.add);
        });
    }

    pub fn add(&mut self) {
        if self.add.is_empty() {
            return;
        }
        self.list.push(self.add.clone());
        self.add.clear();
        self.list.sort_by_key(|s| s.to_lowercase());
    }

    pub fn remove_selected(&mut self) {
        let mut sorted_indices: Vec<usize> = self.selected.clone();
        sorted_indices.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

        for index in sorted_indices {
            if index < self.list.len() {
                self.list.remove(index);
            }
        }
        self.selected.clear();
    }
}
