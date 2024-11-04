use crate::prelude::*;

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
    fn render_progress_bar(&mut self, ui: &mut egui::Ui) {
        self.config.render(ui);
    }
    fn receive(&mut self) -> Option<HashSet<FileRecord>> {
        self.config.receive()
    }
    fn abort(&mut self) {
        self.config.abort();
    }

    fn clear(&mut self) {
        self.config.clear();
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

    fn process(&mut self, db: &Database) {
        if self.enabled {
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
        let mut count = 0;

        for row in rows {
            file_records.insert(FileRecord::new(&row));
            count += 1;

            if count % 100 == 0 {
                let _ = progress_sender.send(Progress { count, total }).await;
            }
        }

        Ok(file_records)
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

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum OrderOperator {
    Largest,
    Smallest,
    #[default]
    Contains,
    DoesNotContain,
    Is,
    IsNot,
    IsEmpty,
    IsNotEmpty,
}

impl EnumComboBox for OrderOperator {
    fn as_str(&self) -> &'static str {
        match self {
            OrderOperator::Largest => "Largest",
            OrderOperator::Smallest => "Smallest",
            OrderOperator::Is => "is",
            OrderOperator::IsNot => "is NOT",
            OrderOperator::Contains => "Contains",
            OrderOperator::DoesNotContain => "Does NOT Contain",
            OrderOperator::IsEmpty => "Is Empty",
            OrderOperator::IsNotEmpty => "Is NOT Empty",
        }
    }

    fn variants() -> &'static [OrderOperator] {
        &[
            OrderOperator::Largest,
            OrderOperator::Smallest,
            OrderOperator::Contains,
            OrderOperator::DoesNotContain,
            OrderOperator::Is,
            OrderOperator::IsNot,
            OrderOperator::IsEmpty,
            OrderOperator::IsNotEmpty,
        ]
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct PreservationLogic {
    pub column: String,
    pub operator: OrderOperator,
    pub variable: String,
}

impl PreservationLogic {
    fn get_sql(&self) -> String {
        match self.operator {
            OrderOperator::Largest => format! {"{} DESC", self.column.to_lowercase()},
            OrderOperator::Smallest => format!("{} ASC", self.column.to_lowercase()),
            OrderOperator::Is => format!(
                "CASE WHEN {} IS '%{}%' THEN 0 ELSE 1 END ASC",
                self.column, self.variable,
            ),
            OrderOperator::IsNot => format!(
                "CASE WHEN {} IS '%{}%' THEN 1 ELSE 0 END ASC",
                self.column, self.variable
            ),
            OrderOperator::Contains => format!(
                "CASE WHEN {} LIKE '%{}%' THEN 0 ELSE 1 END ASC",
                self.column, self.variable
            ),
            OrderOperator::DoesNotContain => format!(
                "CASE WHEN {} LIKE '%{}%' THEN 1 ELSE 0 END ASC",
                self.column, self.variable
            ),
            OrderOperator::IsEmpty => format!(
                "CASE WHEN {} IS NOT NULL AND {} != '' THEN 1 ELSE 0 END ASC",
                self.column, self.column
            ),
            OrderOperator::IsNotEmpty => format!(
                "CASE WHEN {} IS NOT NULL AND {} != '' THEN 0 ELSE 1 END ASC",
                self.column, self.column
            ),
        }
    }
    fn get_friendly(&self) -> String {
        match self.operator {
            OrderOperator::Largest => format! {"Largest {}", self.column},
            OrderOperator::Smallest => format!("Smallest {} ", self.column),
            OrderOperator::Is => format!("{} is '{}'", self.column, self.variable),
            OrderOperator::IsNot => format!("{} is NOT '{}'", self.column, self.variable),
            OrderOperator::Contains => format!("{} contains '{}'", self.column, self.variable),
            OrderOperator::DoesNotContain => {
                format!("{} does NOT contain '{}'", self.column, self.variable)
            }
            OrderOperator::IsEmpty => format!("{} is empty", self.column,),
            OrderOperator::IsNotEmpty => format!("{} is NOT empty", self.column,),
        }
    }
}

fn default_order() -> Vec<PreservationLogic> {
    vec![
        PreservationLogic {
            column: String::from("Description"),
            operator: OrderOperator::IsNotEmpty,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("Audio Files"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARIES"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("/LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY/"),
        },
        PreservationLogic {
            column: String::from("Duration"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Channels"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("SampleRate"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BitDepth"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BWDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("ScannedDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
    ]
}

fn tjf_order() -> Vec<PreservationLogic> {
    vec![
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("TJF RECORDINGS"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARIES"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("SHOWS/Tim Farrell"),
        },
        PreservationLogic {
            column: String::from("Description"),
            operator: OrderOperator::IsNotEmpty,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("Audio Files"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("RECORD"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("CREATED SFX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("CREATED FX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("/LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY/"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("SIGNATURE"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("PULLS"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("EDIT"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("MIX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("SESSION"),
        },
        PreservationLogic {
            column: String::from("Duration"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Channels"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("SampleRate"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BitDepth"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BWDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("ScannedDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
    ]
}
