use crate::prelude::*;
use order::*;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Basic {
    pub enabled: bool,
    #[serde(skip)]
    pub config: Node,
    pub match_criteria: SelectableList,
    match_null: bool,
    pub preservation_order: OrderPanel,
}

impl Basic {
    pub fn get_required_metadata_columns(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        // set.insert("rowid".to_string());
        // set.insert("Filename".to_string());
        // set.insert("Pathname".to_string());
        for item in self.match_criteria.get() {
            set.insert(item.clone());
        }
        set.extend(self.preservation_order.get_columns());
        set
    }
}

impl Default for Basic {
    fn default() -> Self {
        let mut default = Basic {
            enabled: true,
            config: Node::default(),
            match_criteria: SelectableList::default(),
            match_null: false,
            preservation_order: OrderPanel::default(),
        };
        default.match_criteria.set(vec![
            "Channels".to_owned(),
            "Duration".to_owned(),
            "Filename".to_owned(),
        ]);
        default.preservation_order.list = default_order();
        default
    }
}

impl NodeCommon for Basic {
    fn config(&mut self) -> &mut Node {
        &mut self.config
    }
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn render(&mut self, ui: &mut egui::Ui, db: &Database) {
        ui.checkbox(&mut self.enabled, "Basic Duplicate Search");
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label("Duplicate Match Criteria: ");
        });
        if self.match_criteria.get().is_empty() {
            self.enabled = false;
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

    fn process(&mut self, db: &Database, _: &HashSet<String>) {
        let progress_sender = self.config.progress.tx.clone();
        let status_sender = self.config.status.tx.clone();
        let pool = db.pool().unwrap();
        let order = self.preservation_order.extract_sql().clone();
        let match_groups = self.match_criteria.get().to_vec();
        let match_null = self.match_null;
        self.config.wrap_async(move || {
            Self::async_gather(
                pool,
                progress_sender,
                status_sender,
                order,
                match_groups,
                match_null,
            )
        })
    }
}

impl Basic {
    pub fn tjf_default(&mut self) {
        *self = Self::default();

        self.match_criteria.set(vec!["Filename".to_owned()]);
        self.preservation_order.list = tjf_order();
    }

    pub async fn async_gather(
        pool: SqlitePool,
        progress_sender: mpsc::Sender<Progress>,
        status_sender: mpsc::Sender<Arc<str>>,
        order: Vec<String>,
        match_groups: Vec<String>,
        match_null: bool,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let mut file_records = HashSet::new();
        let _ = status_sender
            .send("Gathering Duplicate Records".into())
            .await;
        println!("basic search begin");
        // Construct the ORDER BY clause dynamically
        let order_clause = order.join(", ");
        let partition_by_clause = match_groups.join(", ");
        let where_clause = if match_null || match_groups.is_empty() {
            String::new()
        } else {
            let non_null_conditions: Vec<String> = match_groups
                .iter()
                .map(|group| format!("{group} IS NOT NULL AND {group} !=''"))
                .collect();
            format!("WHERE {}", non_null_conditions.join(" AND "))
        };

        let sql = format!(
            "
        WITH ranked AS (
            SELECT
                rowid AS id,
                filename,
                duration,
                filepath,
                ROW_NUMBER() OVER (
                    PARTITION BY {}
                    ORDER BY {}
                ) as rn
            FROM {}
            {}
        )
        SELECT id, filename, duration, filepath FROM ranked WHERE rn > 1
        ",
            partition_by_clause, order_clause, TABLE, where_clause
        );
        println!("fetching rows: {}", &sql);
        let rows = sqlx::query(&sql).fetch_all(&pool).await?;
        println!("received rows");
        let _ = status_sender.send("Organizing Records".into()).await;

        let total = rows.len();
        let mut counter = 0;

        for row in rows {
            file_records.insert(FileRecord::new(&row));
            counter += 1;

            if counter % 100 == 0 {
                let _ = progress_sender.send(Progress { counter, total }).await;
            }
        }

        Ok(file_records)
    }
}
