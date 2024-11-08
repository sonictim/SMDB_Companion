use crate::prelude::*;
use chrono::{Duration, NaiveDateTime};
use std::cmp::Ordering;

pub fn hashset_to_query_string(set: &HashSet<String>) -> String {
    let result: Vec<String> = set
        .iter()
        .map(|s| format!("CAST({s} AS TEXT) AS {s}"))
        .collect();
    let result = format!("rowid, filename, duration, pathname, {}", result.join(", "));
    println!("{result}");
    result
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
    pub fn sort_vec(&self, vec: &mut Vec<FileRecord>) {
        for l in self.list.iter().rev() {
            l.sort(vec);
        }
    }

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

            enum_combo_box(ui, &mut self.operator, "order column");
            match self.operator {
                O::Largest | O::Smallest | O::IsEmpty | O::IsNotEmpty => {}
                _ => {
                    ui.text_edit_singleline(&mut self.input);
                }
            }

            if ui.button("Add Line").clicked {
                match self.operator {
                    O::Largest | O::Smallest | O::IsEmpty | O::IsNotEmpty => {
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
    pub fn get_columns(&self) -> HashSet<String> {
        self.list.iter().map(|logic| logic.column.clone()).collect()
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
            O::Largest => "Largest",
            O::Smallest => "Smallest",
            O::Is => "is",
            O::IsNot => "is NOT",
            O::Contains => "Contains",
            O::DoesNotContain => "Does NOT Contain",
            O::IsEmpty => "Is Empty",
            O::IsNotEmpty => "Is NOT Empty",
        }
    }

    fn variants() -> &'static [OrderOperator] {
        &[
            O::Largest,
            O::Smallest,
            O::Contains,
            O::DoesNotContain,
            O::Is,
            O::IsNot,
            O::IsEmpty,
            O::IsNotEmpty,
        ]
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParsedValue {
    Integer(i64),
    Duration(Duration),
    DateTime(NaiveDateTime),
}

impl PartialOrd for ParsedValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ParsedValue::Integer(a), ParsedValue::Integer(b)) => a.partial_cmp(b),
            (ParsedValue::Duration(a), ParsedValue::Duration(b)) => {
                a.num_milliseconds().partial_cmp(&b.num_milliseconds())
            }
            (ParsedValue::DateTime(a), ParsedValue::DateTime(b)) => a.partial_cmp(b),

            // For different types, we define a custom ordering or return None.
            (ParsedValue::Integer(_), _) => Some(Ordering::Less),
            (ParsedValue::Duration(_), ParsedValue::DateTime(_)) => Some(Ordering::Less),
            (ParsedValue::DateTime(_), ParsedValue::Integer(_)) => Some(Ordering::Greater),
            _ => None,
        }
    }
}

impl Ord for ParsedValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

// Parse a string into an appropriate `ParsedValue`
fn parse_string(value: &str) -> Result<ParsedValue, &'static str> {
    // Try to parse as an integer
    if let Ok(int_value) = value.parse::<i64>() {
        return Ok(ParsedValue::Integer(int_value));
    }

    // Try to parse as a duration in "MM:SS.mmm" or "HH:MM:SS.mmm" format
    if let Some(parsed_duration) = parse_duration(value) {
        return Ok(ParsedValue::Duration(parsed_duration));
    }

    // Try to parse as a datetime in "YYYY-MM-DD HH:MM:SS" format
    if let Ok(date_time) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(ParsedValue::DateTime(date_time));
    }

    Err("Invalid format")
}

fn parse_duration(value: &str) -> Option<Duration> {
    let parts: Vec<&str> = value.split(&[':', '.'][..]).collect();
    match parts.len() {
        3 => {
            // "MM:SS.mmm"
            let minutes: i64 = parts[0].parse().ok()?;
            let seconds: i64 = parts[1].parse().ok()?;
            let millis: i64 = parts[2].parse().ok()?;
            Some(
                Duration::minutes(minutes)
                    + Duration::seconds(seconds)
                    + Duration::milliseconds(millis),
            )
        }
        4 => {
            // "HH:MM:SS.mmm"
            let hours: i64 = parts[0].parse().ok()?;
            let minutes: i64 = parts[1].parse().ok()?;
            let seconds: i64 = parts[2].parse().ok()?;
            let millis: i64 = parts[3].parse().ok()?;
            Some(
                Duration::hours(hours)
                    + Duration::minutes(minutes)
                    + Duration::seconds(seconds)
                    + Duration::milliseconds(millis),
            )
        }
        _ => None,
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct PreservationLogic {
    pub column: String,
    pub operator: OrderOperator,
    pub variable: String,
}

impl PreservationLogic {
    fn sort(&self, vec: &mut Vec<FileRecord>) {
        match self.operator {
            O::Largest => {
                vec.sort_by(|a, b| {
                    // Get values from each FileRecord
                    let a_value = a.data.get(&self.column).map_or("", |v| v);
                    let b_value = b.data.get(&self.column).map_or("", |v| v);

                    // Parse values
                    let a_num = parse_string(a_value).unwrap_or(ParsedValue::Integer(0));
                    let b_num = parse_string(b_value).unwrap_or(ParsedValue::Integer(0));

                    // Compare b to a for descending order
                    b_num.cmp(&a_num)
                });
            }
            O::Smallest => {}
            O::Is => {
                vec.sort_by(|a, b| {
                    let a_matches = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == *self.variable);
                    let b_matches = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == self.variable);

                    b_matches.cmp(&a_matches)
                });
            }
            O::IsNot => {
                vec.sort_by(|a, b| {
                    let a_matches = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == self.variable);
                    let b_matches = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == self.variable);

                    a_matches.cmp(&b_matches)
                });
            }
            O::IsEmpty => {
                vec.sort_by(|a, b| {
                    let a_empty = a.data.get(&self.column).map_or(false, |v| v.is_empty());
                    let b_empty = b.data.get(&self.column).map_or(false, |v| v.is_empty());
                    b_empty.cmp(&a_empty)
                });
            }
            O::IsNotEmpty => {
                vec.sort_by(|a, b| {
                    let a_empty = a.data.get(&self.column).map_or(false, |v| v.is_empty());
                    let b_empty = b.data.get(&self.column).map_or(false, |v| v.is_empty());
                    a_empty.cmp(&b_empty)
                });
            }
            O::Contains => {
                vec.sort_by(|a, b| {
                    let a_contains = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(&self.variable));
                    let b_contains = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(&self.variable));
                    b_contains.cmp(&a_contains)
                });
            }
            O::DoesNotContain => {
                vec.sort_by(|a, b| {
                    let a_contains = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(&self.variable));
                    let b_contains = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(&self.variable));
                    a_contains.cmp(&b_contains)
                });
            }
        }
    }

    fn get_sql(&self) -> String {
        match self.operator {
            O::Largest => format! {"{} DESC", self.column.to_lowercase()},
            O::Smallest => format!("{} ASC", self.column.to_lowercase()),
            O::Is => format!(
                "CASE WHEN {} IS '%{}%' THEN 0 ELSE 1 END ASC",
                self.column, self.variable,
            ),
            O::IsNot => format!(
                "CASE WHEN {} IS '%{}%' THEN 1 ELSE 0 END ASC",
                self.column, self.variable
            ),
            O::Contains => format!(
                "CASE WHEN {} LIKE '%{}%' THEN 0 ELSE 1 END ASC",
                self.column, self.variable
            ),
            O::DoesNotContain => format!(
                "CASE WHEN {} LIKE '%{}%' THEN 1 ELSE 0 END ASC",
                self.column, self.variable
            ),
            O::IsEmpty => format!(
                "CASE WHEN {} IS NOT NULL AND {} != '' THEN 1 ELSE 0 END ASC",
                self.column, self.column
            ),
            O::IsNotEmpty => format!(
                "CASE WHEN {} IS NOT NULL AND {} != '' THEN 0 ELSE 1 END ASC",
                self.column, self.column
            ),
        }
    }
    fn get_friendly(&self) -> String {
        match self.operator {
            O::Largest => format! {"Largest {}", self.column},
            O::Smallest => format!("Smallest {} ", self.column),
            O::Is => format!("{} is '{}'", self.column, self.variable),
            O::IsNot => format!("{} is NOT '{}'", self.column, self.variable),
            O::Contains => format!("{} contains '{}'", self.column, self.variable),
            O::DoesNotContain => {
                format!("{} does NOT contain '{}'", self.column, self.variable)
            }
            O::IsEmpty => format!("{} is empty", self.column,),
            O::IsNotEmpty => format!("{} is NOT empty", self.column,),
        }
    }
}

pub fn default_order() -> Vec<PreservationLogic> {
    vec![
        PreservationLogic {
            column: String::from("Description"),
            operator: O::IsNotEmpty,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::DoesNotContain,
            variable: String::from("Audio Files"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("LIBRARIES"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("/LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("LIBRARY/"),
        },
        PreservationLogic {
            column: String::from("Duration"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Channels"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("SampleRate"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BitDepth"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BWDate"),
            operator: O::Smallest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("ScannedDate"),
            operator: O::Smallest,
            variable: String::new(),
        },
    ]
}

pub fn tjf_order() -> Vec<PreservationLogic> {
    vec![
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("TJF RECORDINGS"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("LIBRARIES"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::DoesNotContain,
            variable: String::from("SHOWS/Tim Farrell"),
        },
        PreservationLogic {
            column: String::from("Description"),
            operator: O::IsNotEmpty,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::DoesNotContain,
            variable: String::from("Audio Files"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("RECORD"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("CREATED SFX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("CREATED FX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("/LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("LIBRARY/"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("SIGNATURE"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::Contains,
            variable: String::from("PULLS"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::DoesNotContain,
            variable: String::from("EDIT"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::DoesNotContain,
            variable: String::from("MIX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: O::DoesNotContain,
            variable: String::from("SESSION"),
        },
        PreservationLogic {
            column: String::from("Duration"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Channels"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("SampleRate"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BitDepth"),
            operator: O::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BWDate"),
            operator: O::Smallest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("ScannedDate"),
            operator: O::Smallest,
            variable: String::new(),
        },
    ]
}
