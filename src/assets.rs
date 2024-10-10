use eframe::egui::{self, RichText, Ui};
// use sqlx::SqlitePool;
// use sqlx::sqlite::SqlitePool;
// use tokio;
// use tokio::sync::mpsc::Sender;
use crate::config::*;

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

// pub fn enabled_text(text: &str, enabled: &bool) -> RichText {
//     if *enabled {
//         RichText::new(text)
//     } else {
//         RichText::new(text).weak()
//     }
// }

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

pub fn combo_box(ui: &mut Ui, label: &str, selected: &mut String, list: &Vec<String>) {
    egui::ComboBox::from_id_salt(label)
        .selected_text(selected.clone())
        .show_ui(ui, |ui| {
            for item in list {
                ui.selectable_value(selected, item.clone(), item);
            }
        });
}

// pub fn node_status_line(ui: &mut Ui, text: &str, working: bool) {
//     ui.horizontal(|ui| {
//         if working {
//             ui.spinner();
//         } else {
//             ui.add_space(24.0);
//         }
//         ui.label(RichText::new(text).strong());
//     });
// }

pub fn node_progress_bar(ui: &mut Ui, node: &NodeConfig) {
    ui.horizontal(|ui| {
        if node.working {
            ui.spinner();
        } else {
            ui.add_space(24.0)
        }
        ui.label(RichText::new(&node.status).strong());
        if node.working {
            ui.label(format!(
                "Progress: {} / {}",
                node.progress.0, node.progress.1
            ));
        }
    });

    if node.working {
        ui.add(
            egui::ProgressBar::new(node.progress.0 / node.progress.1)
                // .text("progress")
                .desired_height(4.0),
        );
    } else {
        // ui.separator();
    }
    empty_line(ui);
}

pub fn selectable_grid(
    ui: &mut Ui,
    label: &str,
    columns: usize,
    selected: &mut Vec<usize>,
    list: &mut [String],
) {
    egui::Grid::new(label)
        .num_columns(columns)
        .spacing([20.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            for (index, tag) in list.iter_mut().enumerate() {
                // Check if current index is in `sel_tags`
                let is_selected = selected.contains(&index);

                if ui
                    .selectable_label(is_selected, RichText::new(tag.clone()).size(14.0))
                    .clicked()
                {
                    if is_selected {
                        // Deselect
                        selected.retain(|&i| i != index);
                    } else {
                        // Select
                        selected.push(index);
                    }
                }

                if (index + 1) % columns == 0 {
                    ui.end_row(); // Move to the next row after 4 columns
                }
            }

            // End the last row if not fully filled
            if list.len() % columns != 0 {
                ui.end_row();
            }
        });
}

pub fn records_window(
    ctx: &egui::Context,
    records: &str,
    open: &mut bool,
    scroll_to_top: &mut bool,
) {
    let available_size = ctx.available_rect(); // Get the full available width and height
    let width = available_size.width() - 20.0;
    let height = available_size.height();
    egui::Window::new("Records Marked as Duplicates")
        .open(open) // Control whether the window is open
        .resizable(false) // Make window non-resizable if you want it fixed
        .min_width(width)
        .min_height(height)
        .max_width(width)
        .max_height(height)
        .show(ctx, |ui| {
            // ui.label("To Be Implemented\n Testing line break");

            if *scroll_to_top {
                egui::ScrollArea::vertical()
                    .max_height(height)
                    .max_width(width)
                    .scroll_offset(egui::vec2(0.0, 0.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(records).size(14.0));
                    });
                *scroll_to_top = false;
            } else {
                egui::ScrollArea::vertical()
                    .max_height(height)
                    .max_width(width)
                    .show(ui, |ui| {
                        ui.label(RichText::new(records).size(14.0));
                    });
            }
        });
}

// pub fn order_help(ui: &mut Ui) {
//     ui.heading("Column in order of Priority and whether it should be DESCending or ASCending.");
//     ui.label(
//         "These are SQL arguments and Google/ChatGPT can help you figure out how to compose them",
//     );
//     ui.horizontal(|_| {});
//     ui.heading("Examples:");
//     ui.heading("CASE WHEN pathname LIKE '%Audio Files%' THEN 1 ELSE 0 END ASC");
//     ui.label("Records with 'Audio Files' in the path will be removed over something that does not have it");
//     ui.horizontal(|_| {});
//     ui.heading("CASE WHEN pathname LIKE '%LIBRARY%' THEN 0 ELSE 1 END ASC");
//     ui.label(
//         "Records with 'LIBRARY' (not case sensitive) in the path will be kept over records without",
//     );
//     ui.horizontal(|_| {});
//     ui.heading("Rules at the top of the list are prioritized over those below");
//     ui.separator();
// }

//SMALL TAG EDITOR

// ui.horizontal(|ui| {
//     ui.add_space(24.0);
//     if ui.button("Add Tag:").clicked {
//         app.tags.sort_by_key(|s| s.to_lowercase());
//         if app.new_tag.len() > 0 {
//             app.tags.push(app.new_tag.clone());
//             app.new_tag = "".to_string();
//     }}
//     ui.text_edit_singleline(&mut app.new_tag);
// });
//     ui.horizontal(|ui| {
//         ui.add_space(24.0);
//         if let Some(tag_ref) = &mut app.tags.option {
//             if ui.button("Remove Tag").clicked {
//                 app.tags.retain(|s| s != tag_ref);
//                 tag_ref.clear();
//             }
//             egui::ComboBox::from_label("")
//             .selected_text(format!("{}", tag_ref))
//             .show_ui(ui, |ui| {
//                 for tag in &app.tags {
//                     ui.selectable_value(tag_ref, tag.to_string(), format!("{tag}"));
//                 }
//             });
//         }
//     });
