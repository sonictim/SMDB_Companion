use crate::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct FindPanel {
    #[serde(skip)]
    db_name: String,
    column: String,
    find: String,
    find_buf: String,
    replace: String,
    replace_buf: String,
    search_replace_path: bool,
    path_buf: bool,
    dirty: bool,
    case_sensitive: bool,
    #[serde(skip)]
    find_io: AsyncTunnel<usize>,
    #[serde(skip)]
    handle: Option<tokio::task::JoinHandle<()>>,

    #[serde(skip)]
    replace_safety: bool,
    #[serde(skip)]
    count: usize,
}

impl FindPanel {
    pub fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>, registration: Option<bool>) {
        self.receive_async_data();

        let Some(db) = db else {
            ui.heading(RichText::new("No Open Database").weak());
            welcome_message(ui);
            return;
        };

        if db.name != self.db_name {
            self.replace_safety = false;
            self.count = 0;
            self.db_name = db.name.clone();
        }

        if db.size == 0 {
            ui.heading("No Records in Database");
            return;
        }
        if self.column.is_empty() {
            self.column = String::from("Library");
            self.search_replace_path = true;
        }
        ui.heading(RichText::new("Find and Replace Metadata").strong());

        empty_line(ui);
        self.render_case_sensitive_option(ui);
        empty_line(ui);
        self.render_inputs(ui);
        self.render_column_selector(ui, db);
        empty_line(ui);
        ui.separator();
        empty_line(ui);
        self.render_dirty_records_option(ui);
        self.render_find_replace(ui, db, registration);
    }

    fn render_case_sensitive_option(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut text = RichText::new("Case Sensitive").size(14.0);
            if self.case_sensitive {
                text = text.color(egui::Color32::from_rgb(255, 0, 0)).strong()
            }
            ui.checkbox(&mut self.case_sensitive, text);
        });
    }

    fn render_inputs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Find Text: ");
            ui.text_edit_singleline(&mut self.find);
        });
        ui.horizontal(|ui| {
            ui.label("Replace: ");
            ui.add_space(8.0);
            ui.text_edit_singleline(&mut self.replace);
        });
    }

    fn render_column_selector(&mut self, ui: &mut egui::Ui, db: &Database) {
        ui.horizontal(|ui| {
            ui.label("in Column: ");
            ui.radio_value(&mut self.search_replace_path, true, "FilePath");
            ui.radio_value(&mut self.search_replace_path, false, "Other");

            let filtered_columns: Vec<_> = db
                .columns
                .iter()
                .filter(|&col| !["FilePath", "Pathname", "Filename"].contains(&col.as_str()))
                .cloned()
                .collect();

            egui::ComboBox::from_id_salt("find_column")
                .selected_text(&self.column)
                .show_ui(ui, |ui| {
                    for item in filtered_columns {
                        ui.selectable_value(&mut self.column, item.clone(), item);
                    }
                });
        });
    }
    fn render_dirty_records_option(&mut self, ui: &mut egui::Ui) {
        if !self.search_replace_path {
            ui.checkbox(&mut self.dirty, "Mark Records as Dirty?");
            ui.label("Dirty Records are audio files with metadata that is not embedded");
            empty_line(ui);
            ui.separator();
            empty_line(ui);
        }
    }

    fn render_find_replace(
        &mut self,
        ui: &mut egui::Ui,
        db: &Database,
        registration: Option<bool>,
    ) {
        if self.find.is_empty() {
            return;
        }
        if ui
            .button(RichText::new("Find Records").size(16.0))
            .clicked()
        {
            self.find_records(db.pool());
        }
        empty_line(ui);

        self.update_buffers();

        if !self.replace_safety && self.count > 0 && registration == Some(true) {
            ui.label(format!("{} records replaced", self.count));
            return;
        }

        if self.handle.is_some() {
            ui.spinner();
        } else if self.replace_safety {
            let column = if self.search_replace_path {
                "FilePath"
            } else {
                &self.column
            };
            ui.label(
                RichText::new(format!(
                    "Found {} records matching '{}' in {} of SM database: {}",
                    self.count, self.find, column, db.name
                ))
                .strong(),
            );
        }
        if self.count == 0 {
            return;
        }

        if registration == Some(false) {
            ui.label(
                RichText::new("\nUNREGISTERED!\nPlease Register to Continue with Replacement\nIf you need to purchase a license, there is a link below")
                    .strong(),
            );
            return;
        }

        self.render_replace_warning(ui);
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("Replace Records").size(16.0))
                .clicked()
            {
                self.replace_records(db);
            }
            if ui.button(RichText::new("Cancel").size(16.0)).clicked() {
                self.count = 0;
                self.replace_safety = false;
            }
        });
    }

    fn render_replace_warning(&self, ui: &mut egui::Ui) {
        ui.label(format!("Replace with \"{}\" ?", self.replace));
        ui.horizontal(|ui| {
            ui.label("This is");
            ui.label(RichText::new("NOT").strong());
            ui.label("undoable");
        });
        if self.search_replace_path {
            ui.label("This does not alter your file system.");
        }
    }

    fn receive_async_data(&mut self) {
        if let Ok(count) = self.find_io.rx.lock().unwrap().try_recv() {
            self.count = count;
            self.handle = None;
        }
    }

    fn update_buffers(&mut self) {
        if self.find != self.find_buf
            || self.replace != self.replace_buf
            || self.search_replace_path != self.path_buf
        {
            self.replace_safety = false;
            self.find_buf = self.find.clone();
            self.replace_buf = self.replace.clone();
            self.path_buf = self.search_replace_path;
        }
    }

    fn find_records(&mut self, pool: Option<SqlitePool>) {
        self.replace_safety = true;
        let tx = self.find_io.tx.clone();
        let Some(pool) = pool else { return };
        let find = self.find.clone();
        let column = if self.search_replace_path {
            "FilePath"
        } else {
            &self.column
        };
        let case = if self.case_sensitive { "GLOB" } else { "LIKE" };
        let search_query = format!("SELECT COUNT(rowid) FROM {TABLE} WHERE {column} {case} ?");

        let handle = tokio::spawn(async move {
            println!("Inside Find Async");
            let result: (i64,) = sqlx::query_as(&search_query)
                .bind(format!("%{}%", find))
                .fetch_one(&pool)
                .await
                .unwrap();

            let _ = tx.send(result.0 as usize).await;
        });
        self.handle = Some(handle);
    }

    fn replace_records(&mut self, db: &Database) {
        // let tx = self.find_tx.clone().expect("tx channel exists");
        let Some(pool) = db.pool() else {
            return;
        };
        let find = self.find.clone();
        let replace = self.replace.clone();
        let column = self.column.clone();
        let dirty = self.dirty;
        let is_filepath = self.search_replace_path;
        let case_sensitive = self.case_sensitive;
        tokio::spawn(async move {
            println!("inside replace async");
            let dirty_text = if dirty && !is_filepath {
                ", _Dirty = 1"
            } else {
                ""
            };
            let case_text = if case_sensitive { "GLOB" } else { "LIKE" };

            let queries = if is_filepath {
                vec![
                    format!("UPDATE {TABLE} SET FilePath = REPLACE(Filename, '{}', '{}'){} WHERE Filename {} '%{}%'", find, replace, dirty_text, case_text, find),
                    format!("UPDATE {TABLE} SET Filename = REPLACE(Filename, '{}', '{}'){} WHERE Filename {} '%{}%'", find, replace, dirty_text, case_text, find),
                    format!("UPDATE {TABLE} SET Pathname = REPLACE(Pathname, '{}', '{}'){} WHERE Pathname {} '%{}%'", find, replace, dirty_text, case_text, find),
                    format!("UPDATE justinrdb_Pathname SET Pathname = REPLACE(Pathname, '{}', '{}'){} WHERE Pathname {} '%{}%'", find, replace, dirty_text, case_text, find),
                ]
            } else {
                vec![format!(
                    "UPDATE {TABLE} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
                    column, column, find, replace, dirty_text, column, case_text, find
                )]
            };

            for query in queries {
                println!("{}", query);
                let result = sqlx::query(&query).execute(&pool).await;
                println!("{:?}", result);
            }
        });
        self.replace_safety = false;
    }
}
