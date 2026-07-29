use std::os::linux::raw;
use std::str::FromStr;

use chrono::Date;
use chrono::Datelike;
use chrono::Month;
use chrono::NaiveDate;
use chrono::offset;
use eframe::egui;
use egui::LayerId;
use egui::Order;
use egui::Popup;
use egui::PopupAnchor;
use egui::vec2;
use rand::RngExt;
use serde::Deserialize;
use serde::Serialize;

use crate::database::Database;
use crate::database::create_backend_with_string;
use crate::databasespec::Event;
use crate::databasespec::EventTime;
use crate::databasespec::EventsDatabase;
use crate::databasespec::Settings;
use crate::databasespec::SettingsDatabase;
use crate::databasespec::SimpleTimes;

use directories::ProjectDirs;

mod databasespec;

mod database {
    include!("jsondatabase.rs");
}

use database::JsonBackend;

#[derive(PartialEq, Clone, Debug)]
struct DayTime {
    pub(crate) year: u64,
    pub(crate) month: Month,
    pub(crate) day: u64,
}

#[derive(Clone)]
struct MonthTime {
    year: u64,
    month: Month,
}

#[derive(Clone, Debug)]
struct WorkingEvent {
    time: DayTime,
    working_hour: String,
    working_minute: String,
    working_name: String,
    working_desc: String,
    working_notification_times: String,
    working_repeat_times: String,
    working_mirror_times: String,
    working_update_repeat_for_current_day: bool,
}
impl Default for WorkingEvent {
    fn default() -> Self {
        let date = chrono::offset::Local::now();
        let year = date.year() as u64;
        let month = Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();
        Self {
            time: DayTime {
                year,
                month,
                day: 1,
            },
            working_hour: Default::default(),
            working_minute: Default::default(),
            working_name: Default::default(),
            working_desc: Default::default(),
            working_notification_times: Default::default(),
            working_repeat_times: Default::default(),
            working_update_repeat_for_current_day: Default::default(),
            working_mirror_times: Default::default(),
        }
    }
}

fn main() -> eframe::Result {
    env_logger::init();
    let icon_image = image::load_from_memory(include_bytes!("../assets/calendar-rs-v1.png"))
        .unwrap()
        .to_rgba8();
    let (icon_width, icon_height) = icon_image.dimensions();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            title: Some("calendar-rs-egui".to_string()),
            app_id: Some("calendar-rs-egui".to_string()),
            inner_size: Some(vec2(320.0, 240.0)),
            icon: Some(std::sync::Arc::new(egui::IconData {
                rgba: icon_image.to_vec(),
                width: icon_width,
                height: icon_height,
            })),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(MyApp::new(cc)))
        }),
    )
}
fn first_connection() -> Result<JsonBackend, String> {
    Ok(JsonBackend::new(None))
}

struct MyApp {
    database: database::Database,
    working_database_file: Option<String>,
    show_events: bool,
    show_settings: bool,
    current_month: Option<MonthTime>,
    current_displayed_event: Option<Event>,
    working_settings: Settings,
    selected_date: Option<WorkingEvent>,
    rt: tokio::runtime::Runtime,
    client: reqwest::Client,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            database: Database::new(None),
            working_database_file: None,
            show_events: false,
            show_settings: false,
            current_month: None,
            current_displayed_event: None,
            working_settings: Settings::default(),
            selected_date: None,
            rt: tokio::runtime::Runtime::new().unwrap(),
            client: reqwest::Client::new(),
        }
    }
}
impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let database = 'database: {
            if let Some(proj_dirs) = ProjectDirs::from("", "", "calendar-rs-egui") {
                // let config_dir = proj_dirs.config_dir();
                let data_dir = proj_dirs.data_dir().to_string_lossy();
                if let Ok(conn) = create_backend_with_string(format!("{data_dir}/database.json")) {
                    break 'database database::Database::new(Some(conn));
                }
            }
            if let Ok(conn) = first_connection() {
                database::Database::new(Some(conn))
            } else {
                database::Database::new(None)
            }
        };
        // if let (Ok(settings), Ok(events)) = (self.database.get_settings(), self.database.get_events()) {
        //     get_all_events_from_webhook(&self.rt, self.client, settings, events)
        // }
        let client = reqwest::Client::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Ok(settings) = database.get_settings() {
            if settings.enabled_webhooks {
                get_all_events_from_webhook(&rt, client.clone(), database.clone());
            }
        }

        let _ = database.ensure_database_conn();

        let date = chrono::offset::Local::now();
        let year = date.year() as u64;
        let month = Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();

        Self {
            database: database.clone(),
            working_database_file: None,
            show_events: false,
            show_settings: false,
            current_month: Some(MonthTime {
                year: year.into(),
                month,
            }),
            current_displayed_event: None,
            working_settings: database.get_settings().unwrap(),
            selected_date: None,
            rt,
            client,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let all_days = vec![
            "Monday", "Tuesday", "Wensday", "Thursday", "Friday", "Saturday", "Sunday",
        ];

        let date = chrono::offset::Local::now();
        let date_month = Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();
        let date_year = date.year() as u64;
        let year = self.current_month.clone().unwrap().year;
        let month = self.current_month.clone().unwrap().month;
    
        let current_width = ui.available_width();
        let current_height = ui.available_width();

        //let middle_position = egui::pos2(current_width / 3.3333, current_height / 3.3333);

        Popup::new(
            "events".into(),
            ui.ctx().clone(),
            PopupAnchor::Position(egui::pos2(current_width / 3.3333, current_height / 3.3333)),
            LayerId::new(Order::Foreground, "first-layer".into()),
        )
        .open(self.current_displayed_event.is_some())
        .show(|ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(50, 50, 200))
                .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_min_width(550.0);
                    ui.label(self.current_displayed_event.clone().unwrap().name);
                    ui.label(self.current_displayed_event.clone().unwrap().description);
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.current_displayed_event = None;
                        }
                    });
                });
        });

        Popup::new(
            "events".into(),
            ui.ctx().clone(),
            PopupAnchor::Position(egui::pos2(current_width / 2.5, current_height / 6.3333)),
            LayerId::new(Order::Foreground, "first-layer".into()),
        )
        .open(self.show_events)
        .show(|ui| {
            ui.set_height(300.0);
            ui.set_width(300.0);
            let selected_date = self.selected_date.as_mut().unwrap();
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(50, 50, 200))
                .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if let Ok(events) = self.database.get_events_on_day(selected_date.time.clone())
                    {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                        for event in events {
                            egui::Frame::new()
                                .fill(egui::Color32::from_rgb(0, 0, 0))
                                .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.label(event.clone().name);
                                    ui.label(event.clone().description);
                                    ui.label(format!(
                                        "{}:{}",
                                        event.clone().time.get_hour(),
                                        event.clone().time.get_minute()
                                    ));
                                    if event.repeat_until_current_day {
                                        if ui.button("Freeze event at time").clicked() {
                                            let hour_to_freeze = selected_date.working_hour.parse::<u64>().unwrap_or(event.time.get_hour());
                                            let minuite_to_freeze = selected_date.working_minute.parse::<u64>().unwrap_or(event.time.get_minute());
                                            let _ = self.database.remove_event_by_name(event.clone().name);
                                            let mut new_event = event.clone();
                                            new_event.freeze_at_time = Some(
                                                EventTime::new(selected_date.time.year, selected_date.time.month, selected_date.time.day, hour_to_freeze, minuite_to_freeze)
                                            );
                                            let _ = self.database.add_event(new_event);
                                        }
                                    }
                                    if ui.button("Remove event").clicked() {
                                        let cloned_event = event.clone();
                                        if let Ok(settings) = self.database.get_settings() {
                                            if settings.enabled_webhooks {
                                                forward_remove_event(&self.rt, self.client.clone(), settings, cloned_event);
                                            }
                                        }
                                        let _ = self.database.remove_event_by_time(event.time);
                                    }
                                });
                        }
                    });
                    }
                });

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(50, 50, 200))
                .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut selected_date.working_name)
                            .hint_text("Type name here..."),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut selected_date.working_desc)
                            .hint_text("Type desc here..."),
                    );
                    ui.add_space(16.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut selected_date.working_notification_times)
                            .hint_text("Type the amount of time (in units, e.g 1h, 1m), before the event, or 0, to get a notification..."),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut selected_date.working_repeat_times)
                            .hint_text("Type how often this event should repeat, valid units are xd (days)..."),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut selected_date.working_mirror_times)
                            .hint_text("Type how often this event should mirror, valid units are xd (days)..."),
                    );
                    ui.checkbox(
                                &mut selected_date.working_update_repeat_for_current_day,
                                "Repeat until it reaches the current day (updates)",
                    );
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        let half = ui.available_width() / 2.0;

                        ui.allocate_ui(egui::vec2(half, ui.available_height()), |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut selected_date.working_hour)
                                    .hint_text("Type hour here...")
                                    .desired_width(current_width / 2.0),
                            );
                        });
                        ui.allocate_ui(egui::vec2(half, ui.available_height()), |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut selected_date.working_minute)
                                    .hint_text("Type minute here...")
                                    .desired_width(current_width / 2.0),
                            );
                        });
                    });
                    ui.add_space(16.0);
                    if ui.button("Add event").clicked() {
                        if let (Ok(working_hour), Ok(working_minute)) = (
                            &mut selected_date.working_hour.parse::<u64>(),
                            &mut selected_date.working_minute.parse::<u64>(),
                        ) {
                            let event = Event {
                                name: selected_date.working_name.clone(),
                                description: selected_date.working_desc.clone(),
                                time: EventTime::new(
                                    selected_date.time.year,
                                    selected_date.time.month,
                                    selected_date.time.day,
                                    *working_hour,
                                    *working_minute,
                                ),
                                notify_times: parse_simple_times(&selected_date.working_notification_times),
                                repeat_times: parse_simple_times(&selected_date.working_repeat_times),
                                mirror_times: parse_simple_times(&selected_date.working_mirror_times),
                                repeat_until_current_day: selected_date.working_update_repeat_for_current_day,
                                freeze_at_time: None
                            };
                            let add_event_result = self.database.add_event(event.clone());
                            if add_event_result.is_ok() {
                                if let Ok(settings) = self.database.get_settings() {
                                    forward_event(&self.rt, self.client.clone(), settings, event);
                                }
                            }
                        }
                    }
                    ui.horizontal(|ui| {
                            if ui.button("Close").clicked() {
                                self.show_events = false;
                            }
                        });
                });
        });
        if !self.show_settings {
            if let Ok(settings) = self.database.get_settings() {
                self.working_settings = settings;
            }
        }
        Popup::new(
            "settings".into(),
            ui.ctx().clone(),
            PopupAnchor::Position(egui::pos2(current_width / 2.5, current_height / 4.3333)),
            LayerId::new(Order::Foreground, "first-layer".into()),
        )
        .open(self.show_settings)
        .show(|ui| {
            ui.set_height(300.0);
            ui.set_width(300.0);
            ui.checkbox(
                &mut self.working_settings.enabled_webhooks,
                "Enable webhook forwarding",
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.working_settings.webhook_url_push)
                    .hint_text("Type webhook url for individual events to be pushed to..."),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.working_settings.webhook_url_remove)
                    .hint_text("Type webhook url for individual events to be removed to..."),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.working_settings.webhook_url_all)
                    .hint_text("Type webhook url to merge/override events with..."),
            );
            ui.checkbox(
                &mut self.working_settings.override_local_webhook,
                "Override local events with fetched events",
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.working_settings.webhook_header)
                    .hint_text("Type webhook authorization header..."),
            );
            ui.add_space(16.0);

            let mut text = self.working_database_file.clone().unwrap_or_default();
            let response = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .hint_text("Type the database file (if you want to change it)"),
            );
            if response.changed() {
                self.working_database_file = if text.is_empty() { None } else { Some(text) };
            }
            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if let Some(database_file) = &self.working_database_file {
                        if let Ok(conn) = create_backend_with_string(database_file.to_string()) {
                            self.database = Database::new(Some(conn));
                        }
                    };
                    let _ = self.database.set_settings(self.working_settings.clone());
                }
                if ui.button("Close").clicked() {
                    self.working_settings = Settings::default();
                    self.working_database_file = None;
                    self.show_settings = false;
                }
            });
        });
        egui::TopBottomPanel::top("menu_bar").show_inside(ui, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Configure settings").clicked() {
                    self.show_settings = true;
                }
            })
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Egui calendar");
                let mut offset = month_start_offset(
                    self.current_month.clone().unwrap().year,
                    get_month_index(self.current_month.clone().unwrap().month).unwrap() as u64,
                );
                let days: u64 = get_days_in_month_chrono("", month);
                ui.horizontal(|ui| {
                    if ui.button("<").clicked() {
                        self.current_month.as_mut().unwrap().month =
                            shift_months(self.current_month.clone().unwrap().month, -1)
                    }
                    ui.label(month.name());
                    if ui.button(">").clicked() {
                        self.current_month.as_mut().unwrap().month =
                            shift_months(self.current_month.clone().unwrap().month, 1)
                    }
                    ui.add_space(16.0);
                    if ui.button("<").clicked() {
                        self.current_month.as_mut().unwrap().year =
                            self.current_month.clone().unwrap().year - 1;
                    }
                    ui.label(year.to_string());
                    if ui.button(">").clicked() {
                        self.current_month.as_mut().unwrap().year =
                            self.current_month.clone().unwrap().year + 1;
                    }
                });
                egui::Grid::new("date_grid").show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            for day in all_days {
                                ui.label(day);
                                ui.add_space(175.0);
                            }
                        });

                        let total_cells = offset + days;
                        let total_rows = (total_cells + 6) / 7;

                        for row in 0..total_rows {
                            ui.allocate_ui(egui::vec2(current_width, current_height), |ui| {
                                ui.horizontal(|ui| {
                                    for col in 0..7 {
                                        let cell = row * 7 + col;
                                        if cell < offset || cell >= total_cells {
                                            egui::Frame::new()
                                                .fill(egui::Color32::from_rgb(0, 0, 0))
                                                .stroke(egui::Stroke::new(
                                                    2.0,
                                                    egui::Color32::WHITE,
                                                ))
                                                .inner_margin(12.0)
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(175.0, 175.0));
                                                });
                                        } else {
                                            let day_num = cell - offset + 1;
                                            let is_current_period =
                                                self.current_month.clone().unwrap().month
                                                    == date_month
                                                    && self.current_month.clone().unwrap().year
                                                        == date_year;
                                            let is_current_day =
                                                is_current_period && day_num == date.day() as u64;
                                            egui::Frame::new()
                                                .fill(if is_current_day {
                                                    egui::Color32::from_rgb(255, 255, 0)
                                                } else {
                                                    egui::Color32::from_rgb(0, 0, 0)
                                                })
                                                .stroke(egui::Stroke::new(
                                                    2.0,
                                                    egui::Color32::WHITE,
                                                ))
                                                .inner_margin(12.0)
                                                .show(ui, |ui| {
                                                    date_cell(self, ui, day_num);
                                                });
                                        }
                                    }
                                });
                            });
                        }
                    });
                });
                if let Ok(settings) = self.database.get_settings() {
                    if settings.enabled_webhooks {
                        if ui.button("Forward all events over webhook").clicked() {
                            if let Ok(events) = self.database.get_events() {
                                for event in events {
                                    forward_event(
                                        &self.rt,
                                        self.client.clone(),
                                        settings.clone(),
                                        event,
                                    );
                                }
                            }
                        };
                    }
                }
            });
        });
    }
}
fn parse_simple_times(times: &str) -> Vec<SimpleTimes> {
    if times.len() > 0 {
        let mut total_times = Vec::new();
        for time in times.split(",") {
            let mut letters = time.chars();
            let last_letter = letters.next_back().unwrap();
            let amount = letters.as_str().parse::<u64>().unwrap();
            total_times.push(match last_letter {
                'y' => SimpleTimes::Year(amount),
                'm' => SimpleTimes::Month(amount),
                'w' => SimpleTimes::Week(amount),
                'd' => SimpleTimes::Day(amount),
                'h' => SimpleTimes::Hour(amount),
                'M' => SimpleTimes::Minuite(amount),
                _ => SimpleTimes::None,
            });
        }
        total_times
    } else {
        vec![]
    }
}
fn shift_months(original_month: Month, raw_shift: i64) -> Month {
    let mut shift = {
        if raw_shift > 12 {
            raw_shift % 12
        } else {
            raw_shift
        }
    };
    let month_list = vec![
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let month_name = original_month.name();
    let mut current_month = Month::January;
    let current_month_index = month_list
        .iter()
        .position(|current_month| *current_month == month_name)
        .unwrap();
    if current_month_index == 0 && shift < 0 {
        current_month = Month::from_str(month_list[month_list.len() - 1]).unwrap()
    } else if current_month_index + 1 == month_list.len() && shift > 0 {
        current_month = Month::from_str(month_list[0]).unwrap()
    } else {
        if current_month_index as i64 + shift > 12 {
            shift = (current_month_index as i64 + shift) % 12;
        } else {
            shift = current_month_index as i64 + shift;
        }
        current_month = Month::from_str(month_list[shift as usize]).unwrap()
    }
    current_month
}
fn get_month_index(month: Month) -> Option<usize> {
    let month_list = vec![
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    month_list
        .iter()
        .position(|current_month| *current_month == month.name())
}
fn get_month_by_index(index: usize) -> Result<Month, chrono::ParseMonthError> {
    let month_list = vec![
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    Month::from_str(month_list[index - 1])
}
fn intersects_by_day_mirror(
    original_time: DayTime,
    times: Vec<SimpleTimes>,
    comparison_time: DayTime,
    freeze_at_time: &Option<EventTime>,
) -> bool {
    let date = {
        let current_date = chrono::offset::Local::now();
        if let Some(freeze_date) = freeze_at_time {
            DayTime {
                year: freeze_date.get_year(),
                month: freeze_date.get_month(),
                day: freeze_date.get_day(),
            }
        } else {
            DayTime {
                year: current_date.year() as u64,
                month: get_month_by_index(current_date.month().try_into().unwrap()).unwrap(),
                day: current_date.day() as u64,
            }
        }
    };
    let mut intersects = false;
    for time in times.clone() {
        let mut compare_day = false;
        let mut compare_month = false;
        let mut compare_year = false;
        let final_time = match time {
            SimpleTimes::Year(y) => {
                compare_year = true;
                DayTime {
                    year: original_time.year + y,
                    month: original_time.month,
                    day: original_time.day,
                }
            }
            SimpleTimes::Month(m) => {
                compare_month = true;
                DayTime {
                    year: original_time.year,
                    month: shift_months(original_time.month, m.try_into().unwrap()),
                    day: original_time.day,
                }
            }
            SimpleTimes::Week(w) => {
                compare_day = true;
                DayTime {
                    year: original_time.year,
                    month: original_time.month,
                    day: original_time.day + (w * 7),
                }
            }
            SimpleTimes::Day(d) => {
                compare_day = true;
                DayTime {
                    year: original_time.year,
                    month: original_time.month,
                    day: original_time.day + d,
                }
            }
            _ => DayTime {
                year: original_time.year,
                month: original_time.month,
                day: original_time.day,
            },
        };
        
        if compare_day {
            if convert_day_num_to_day(&comparison_time.day) == convert_day_num_to_day(&original_time.day) {
                intersects = true;
            }
        } else if compare_month {
            intersects = false;
        } else if compare_year {
            intersects = false;
        }
    }
    intersects
}
#[derive(PartialEq)]
enum Days {
    Monday, 
    Tuesday,
    Wensday, 
    Thursday, 
    Friday,
    Saturday,
    Sunday
}
fn convert_day_num_to_day(day: &u64) -> Days {
    let final_day = day % 7; 
    match final_day {
        0 => Days::Monday,
        1 => Days::Tuesday,
        2 => Days::Wensday,
        3 => Days::Thursday,
        4 => Days::Friday,
        5 => Days::Saturday,
        6 => Days::Sunday,
        _ => {
            Days::Monday
        }
    }
}
fn intersects_by_day_repeat(
    original_time: DayTime,
    times: Vec<SimpleTimes>,
    comparison_time: DayTime,
    repeat_until_current_day: bool,
    freeze_at_time: &Option<EventTime>,
) -> bool {
    let date = {
        let current_date = chrono::offset::Local::now();
        if let Some(freeze_date) = freeze_at_time {
            DayTime {
                year: freeze_date.get_year(),
                month: freeze_date.get_month(),
                day: freeze_date.get_day(),
            }
        } else {
            DayTime {
                year: current_date.year() as u64,
                month: get_month_by_index(current_date.month().try_into().unwrap()).unwrap(),
                day: current_date.day() as u64,
            }
        }
    };
    let mut intersects = false;
    for time in times.clone() {
        let mut compare_day = false;
        let mut compare_month = false;
        let mut compare_year = false;
        let final_time = match time {
            SimpleTimes::Year(y) => {
                compare_year = true;
                DayTime {
                    year: original_time.year + y,
                    month: original_time.month,
                    day: original_time.day,
                }
            }
            SimpleTimes::Month(m) => {
                compare_month = true;
                DayTime {
                    year: original_time.year,
                    month: shift_months(original_time.month, m.try_into().unwrap()),
                    day: original_time.day,
                }
            }
            SimpleTimes::Week(w) => {
                compare_day = true;
                DayTime {
                    year: original_time.year,
                    month: original_time.month,
                    day: original_time.day + (w * 7),
                }
            }
            SimpleTimes::Day(d) => {
                compare_day = true;
                DayTime {
                    year: original_time.year,
                    month: original_time.month,
                    day: original_time.day + d,
                }
            }
            _ => DayTime {
                year: original_time.year,
                month: original_time.month,
                day: original_time.day,
            },
        };

        if compare_day {
            if comparison_time.day > original_time.day
                && comparison_time.day < final_time.day
                && original_time.month == comparison_time.month
                && original_time.year == comparison_time.year
                && ((!repeat_until_current_day && freeze_at_time.is_none())
                    || (comparison_time.day <= date.day
                        && comparison_time.month == date.month
                        && comparison_time.year == date.year)
                    || (comparison_time.month < date.month && comparison_time.year <= date.year))
            {
                intersects = true;
            }
        } else if compare_year {
            if (original_time.year <= comparison_time.year
                && final_time.year >= comparison_time.year)
            {
                if final_time.year == comparison_time.year
                    || original_time.year == comparison_time.year
                {
                    if final_time.year == comparison_time.year {
                        intersects = !intersects_by_day_repeat(
                            DayTime {
                                year: final_time.year,
                                month: final_time.month,
                                day: final_time.day - 1,
                            },
                            vec![SimpleTimes::Month(
                                get_month_index(final_time.month)
                                    .unwrap()
                                    .try_into()
                                    .unwrap(),
                            )],
                            comparison_time.clone(),
                            repeat_until_current_day,
                            freeze_at_time,
                        )
                    } else {
                        intersects = intersects_by_day_repeat(
                            original_time.clone(),
                            vec![SimpleTimes::Month(
                                get_month_index(final_time.month)
                                    .unwrap()
                                    .try_into()
                                    .unwrap(),
                            )],
                            comparison_time.clone(),
                            repeat_until_current_day,
                            freeze_at_time,
                        )
                    }
                } else {
                    intersects = true;
                }
            }
        } else if compare_month {
            if (((final_time.day < comparison_time.day
                && original_time.month == comparison_time.month)
                || (final_time.day > comparison_time.day
                    && final_time.month == comparison_time.month))
                || (final_time.month != comparison_time.month)
                    && get_month_index(original_time.month).unwrap()
                        < get_month_index(comparison_time.month).unwrap())
                && original_time.year == comparison_time.year
            {
                if !repeat_until_current_day && freeze_at_time.is_none() {
                    intersects = true;
                } else {
                    if (get_month_index(comparison_time.month).unwrap()
                        <= (get_month_index(date.month).unwrap())
                        && comparison_time.year <= date.year
                        && comparison_time.month >= original_time.month
                        && comparison_time.year >= original_time.year)
                    {
                        if get_month_index(comparison_time.month).unwrap()
                            == get_month_index(date.month).unwrap()
                            && comparison_time.year == date.year
                        {
                            if (date.day) >= comparison_time.day {
                                intersects = true;
                            }
                        } else {
                            intersects = true;
                        }
                    }
                }
            }
        }
    }
    if times.len() == 0 {
        if repeat_until_current_day || freeze_at_time.is_some() {
            if get_month_index(comparison_time.month).unwrap()
                <= get_month_index(date.month).unwrap()
                && comparison_time.year <= date.year
                && comparison_time.month >= original_time.month
                && comparison_time.year >= original_time.year
            {
                if get_month_index(comparison_time.month).unwrap()
                    == get_month_index(date.month).unwrap()
                    && comparison_time.year == date.year
                {
                    if date.day >= comparison_time.day && comparison_time.day >= original_time.day {
                        intersects = true;
                    }
                } else if comparison_time.month == original_time.month {
                    if comparison_time.day >= original_time.day {
                        intersects = true;
                    }
                } else {
                    intersects = true;
                }
            }
        }
    }
    intersects
}

fn success_frame(ui: &mut egui::Ui, text: String) -> egui::InnerResponse<egui::Response> {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(0, 0, 0))
        .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
        .inner_margin(12.0)
        .show(ui, |ui| ui.label(text))
}
fn failure_frame(ui: &mut egui::Ui, text: String) -> egui::InnerResponse<egui::Response> {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(0, 0, 0))
        .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
        .inner_margin(12.0)
        .show(ui, |ui| ui.label(text))
}

fn date_cell(state: &mut MyApp, ui: &mut egui::Ui, day: u64) -> egui::InnerResponse<()> {
    ui.set_min_size(egui::vec2(175.0, 175.0));
    ui.vertical(|ui: &mut egui::Ui| {
        ui.label(day.to_string());
        if let Some(month_time) = &state.current_month {
            let day_time = DayTime {
                year: month_time.year,
                month: month_time.month,
                day,
            };
            let n = rand::rng().random_range(1000..=9999);
            if let Ok(events) = state.database.get_events_on_day(day_time.clone()) {
                egui::ScrollArea::vertical()
                    .id_salt(format!("{}_{}", day, n))
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for event in events {
                            egui::Frame::new()
                                .fill(egui::Color32::from_rgb(0, 0, 0))
                                .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    let display_text = {
                                        if event.clone().name.len() > 20 {
                                            format!("{}..", event.clone().name.get(0..20).unwrap())
                                        } else {
                                            event.clone().name
                                        }
                                    };
                                    if ui.button(display_text).clicked() {
                                        state.current_displayed_event = Some(event);
                                    }
                                });
                        }
                    });
            }
            if ui.button("Events").clicked() {
                state.selected_date = Some(WorkingEvent {
                    time: day_time,
                    working_hour: String::new(),
                    working_minute: String::new(),
                    working_desc: String::new(),
                    working_name: String::new(),
                    working_notification_times: String::new(),
                    working_repeat_times: String::new(),
                    working_update_repeat_for_current_day: false,
                    working_mirror_times: String::new(),
                });
                state.show_events = true;
            }
        }
    })
}
fn days_in_first_week(year: u64, month: u64) -> u64 {
    let first_day = NaiveDate::from_ymd_opt(year as i32, month as u32, 1).unwrap();
    let weekday = first_day.weekday().num_days_from_monday();
    7 - weekday as u64
}
fn month_start_offset(year: u64, month: u64) -> u64 {
    println!("month {}", month);
    NaiveDate::from_ymd_opt(year as i32, month as u32 + 1, 1)
        .unwrap()
        .weekday()
        .num_days_from_monday() as u64
}
fn get_days_in_month_chrono(year: &str, month: Month) -> u64 {
    match month {
        Month::January => 31,
        Month::February => 28,
        Month::March => 31,
        Month::April => 30,
        Month::May => 31,
        Month::June => 30,
        Month::July => 31,
        Month::August => 31,
        Month::September => 30,
        Month::October => 31,
        Month::November => 30,
        Month::December => 31,
    }
}

fn get_days_in_month_string(year: &str, month: &str) -> Option<u64> {
    match month {
        "January" => Some(31),
        "February" => Some(28),
        "March" => Some(31),
        "April" => Some(30),
        "May" => Some(31),
        "June" => Some(30),
        "July" => Some(31),
        "August" => Some(31),
        "September" => Some(30),
        "October" => Some(31),
        "November" => Some(30),
        "December" => Some(31),
        _ => None,
    }
}

#[derive(Deserialize, Serialize)]
struct EventsRequest {
    events: Vec<Event>,
}
#[derive(Deserialize, Serialize)]
struct RemoveEventRequest {
    event_name: String,
}

fn get_all_events_from_webhook(
    rt: &tokio::runtime::Runtime,
    client: reqwest::Client,
    database: database::Database,
    // settings: Settings,
    // events: Vec<Event>,
) {
    if let (Ok(settings), Ok(mut events)) = (database.get_settings(), database.get_events()) {
        rt.spawn(async move {
            let response = client
                .get(&settings.webhook_url_all)
                .header("authorization", &settings.webhook_header)
                .send()
                .await
                .unwrap();
            if response.status().is_success() {
                match response.json::<EventsRequest>().await {
                    Ok(body) => {
                        if !&settings.override_local_webhook {
                            events.extend(body.events);
                            let _ = database.set_events(events);
                        } else {
                            let _ = database.set_events(body.events);
                        }
                    }
                    Err(err) => {}
                }
            } else {
                eprintln!("request failed: {}", response.status());
            }
        });
    }
}
fn forward_remove_event(
    rt: &tokio::runtime::Runtime,
    client: reqwest::Client,
    settings: Settings,
    event: Event,
) {
    rt.spawn(async move {
        let _ = client
            .post(&settings.webhook_url_remove)
            .header("authorization", &settings.webhook_header)
            .json(&RemoveEventRequest {
                event_name: event.name,
            })
            .send()
            .await;
    });
}

fn forward_event(
    rt: &tokio::runtime::Runtime,
    client: reqwest::Client,
    settings: Settings,
    event: Event,
) {
    rt.spawn(async move {
        let _ = client
            .post(&settings.webhook_url_all)
            .header("authorization", &settings.webhook_header)
            .json(&event)
            .send()
            .await;
    });
}
