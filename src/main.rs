use std::str::FromStr;

use chrono::Datelike;
use chrono::Month;
use eframe::egui;
use egui::LayerId;
use egui::Order;
use egui::Popup;
use egui::PopupAnchor;

use crate::database::Database;
use crate::databasespec::Event;
use crate::databasespec::EventTime;
use crate::databasespec::EventsDatabase;
use crate::databasespec::Settings;
use crate::databasespec::SettingsDatabase;

mod databasespec;

mod database {
    include!("jsondatabase.rs");
}

use database::JsonBackend;

#[derive(PartialEq, Clone)]
struct DayTime {
    year: u64,
    month: Month,
    day: u64,
}

#[derive(Clone)]
struct MonthTime {
    year: u64,
    month: Month,
}

#[derive(Clone)]
struct WorkingEvent {
    time: DayTime,
    working_hour: String,
    working_minute: String,
    working_name: String,
    working_desc: String,
}
impl Default for WorkingEvent {
    fn default() -> Self {
        let date = chrono::offset::Local::now();
        let year = date.year() as u64;
        let month = Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();
        Self { 
            time: DayTime { year, month, day: 1 }, 
            working_hour: Default::default(), 
            working_minute: Default::default(), 
            working_name: Default::default(), 
            working_desc: Default::default() 
        }
    }
}

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
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
    show_events: bool,
    show_settings: bool,
    current_month: Option<MonthTime>,
    working_settings: Settings,
    selected_date: Option<WorkingEvent>,
    rt: tokio::runtime::Runtime, 
    client: reqwest::Client
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            database: Database::new(None),
            show_events: false,
            show_settings: false,
            current_month: None,
            working_settings: Settings::default(),
            selected_date: None,
            rt: tokio::runtime::Runtime::new().unwrap(),
            client: reqwest::Client::new()
        }
    }
}
impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // let conn = first_connection()?;
        // let database = database::Database::new(Some(conn));
        let database = {
            if let Ok(conn) = first_connection() {
                database::Database::new(Some(conn))
            } else {
                database::Database::new(None)
            }
        };
        let _ = database.ensure_database_conn();

        let date = chrono::offset::Local::now();
        let year = date.year() as u64;
        let month = Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();

        Self {
            database: database.clone(),
            show_events: false,
            show_settings: false,
            current_month: Some(MonthTime {
                year: year.into(),
                month,
            }),
            working_settings: database.get_settings().unwrap(),
            selected_date: None,
            rt: tokio::runtime::Runtime::new().unwrap(),
            client: reqwest::Client::new()
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let month_list = vec!["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let date = chrono::offset::Local::now();
        let date_month = Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();
        let date_year = date.year() as u64;
        // date.year()
        // Year::try_from
        let year = self.current_month.clone().unwrap().year;
        //date.year() as u64;
        let month = self.current_month.clone().unwrap().month;
        //Month::try_from(u8::try_from(date.month()).unwrap()).unwrap();
        // let month = Some(Month::September);
        let current_width = ui.available_width();
        let current_height = ui.available_width();
        //self.show_events = false;
        Popup::new(
            "events".into(),
            ui.ctx().clone(),
            PopupAnchor::Position(egui::pos2(current_width / 16.0, current_height / 32.0)),
            LayerId::new(Order::Foreground, "first-layer".into()),
        )
        .open(self.show_events)
        .show(|ui| {
            ui.set_height(300.0);
            ui.set_width(300.0);
            // egui::Frame::new()
            //     .show(ui, |ui| {
            // if self.selected_date.is_none() {
            //     self.selected_date = Some(WorkingEvent {
            //         time: DayTime { year, month, day: 1 },
            //         working_hour: String::new(),
            //         working_minute: String::new(),
            //         working_name: String::new(),
            //         working_desc: String::new(),
            //     });
            // }
            //let binding = &mut WorkingEvent::default();
            //if self.show_events {}
            let selected_date = self.selected_date.as_mut().unwrap();
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(50, 50, 200))
                .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if let Ok(events) = self.database.get_events_on_day(selected_date.time.clone()) {
                        for event in events {
                            egui::Frame::new()
                                .fill(egui::Color32::from_rgb(0, 0, 0))
                                .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.label(event.name);
                                    if ui.button("Remove event").clicked(){
                                        let _ = self.database.remove_event_by_time(event.time);
                                    }
                                });
                        }
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
                        //self.show_events = false;
                        // if let Some(selected_date) = selected_date {
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
                            };
                            let add_event_result = self.database.add_event(event.clone());
                            if add_event_result.is_ok(){
                                if let Ok(settings) = self.database.get_settings(){
                                    forward_event(&self.rt, self.client.clone(), settings, event);
                                }
                                // if let Ok(settings) = self.database.get_settings(){
                                //     if settings.enabled_websockets {
                                //         // forward_event(&mut self, event);
                                //     }
                                // }
                            }
                        }
                        //}
                    }
                });
            ui.horizontal(|ui| {
                // if ui.button("Save").clicked() {
                //     self.show_events = false;
                // }
                if ui.button("Close").clicked() {
                    //Popup::close_id(ui.ctx(), "events".into());
                    self.show_events = false;
                }
            });
            //});
        });
        if !self.show_settings {
            if let Ok(settings) = self.database.get_settings(){
                self.working_settings = settings;
            }
        }
        Popup::new(
            "settings".into(),
            ui.ctx().clone(),
            PopupAnchor::Position(egui::pos2(current_width / 16.0, current_height / 32.0)),
            LayerId::new(Order::Foreground, "first-layer".into()),
        )
        .open(self.show_settings)
        .show(|ui| {
            ui.set_height(300.0);
            ui.set_width(300.0);
            ui.checkbox(&mut self.working_settings.enabled_websockets, "Enable websocket forwarding");
            ui.add(
                egui::TextEdit::singleline(&mut self.working_settings.websocket_url)
                    .hint_text("Type websocket url..."),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.working_settings.websocket_header)
                    .hint_text("Type websocket authorization header..."),
            );
            ui.horizontal(|ui| {
                // if ui.button("Save").clicked() {
                //     self.show_events = false;
                // }
                if ui.button("Save").clicked() {
                    let _ = self.database.set_settings(self.working_settings.clone());
                }
                if ui.button("Close").clicked() {
                    //Popup::close_id(ui.ctx(), "events".into());
                    self.working_settings = Settings::default();
                    self.show_settings = false;
                }
            });
        });
        egui::TopBottomPanel::top("menu_bar").show_inside(ui, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Configure settings").clicked(){
                    self.show_settings = true;
                }
            })
        });
        //if let Some(month) = month {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Egui calendar");
            let days: u64 = get_days_in_month_chrono("", month);
            //println!("{}", days);
            // println!("{} {}", days, days/7);
            // let size = egui::vec2(200.0, 100.0);
            //egui::Frame::new().show(ui, |ui| {
            // ui.set_min_size(egui::vec2(20.0, 20.0));
            ui.horizontal(|ui| {
                if ui.button("<").clicked() {
                    let month_name = self.current_month.clone().unwrap().month.name();
                    let current_month_index = month_list.iter().position(|current_month| *current_month == month_name).unwrap();
                    if current_month_index == 0 {
                        self.current_month.as_mut().unwrap().month = Month::from_str(month_list[month_list.len()-1]).unwrap()  
                    } else {
                        self.current_month.as_mut().unwrap().month = Month::from_str(month_list[current_month_index-1]).unwrap()
                    }
                }
                ui.label(month.name());
                if ui.button(">").clicked() {
                    let month_name = self.current_month.clone().unwrap().month.name();
                    let current_month_index = month_list.iter().position(|current_month| *current_month == month_name).unwrap();
                    if current_month_index+1 == month_list.len(){
                        self.current_month.as_mut().unwrap().month = Month::from_str(month_list[0]).unwrap()  
                    } else {
                        self.current_month.as_mut().unwrap().month = Month::from_str(month_list[current_month_index+1]).unwrap()  
                    } 
                }
                ui.add_space(16.0);
                if ui.button("<").clicked() {
                    self.current_month.as_mut().unwrap().year = self.current_month.clone().unwrap().year-1;
                }
                ui.label(year.to_string());
                if ui.button(">").clicked() {
                    self.current_month.as_mut().unwrap().year = self.current_month.clone().unwrap().year+1;
                }
            });
            egui::Grid::new("date_grid")
                //.min_col_width(100.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for week in 0..days / 7 {
                            ui.allocate_ui(egui::vec2(current_width, current_height), |ui| {
                                ui.horizontal(|ui| {
                                    for day in 0..7 {
                                        let full_day = week * 7 + day;
                                        let is_current_period = self.current_month.clone().unwrap().month == date_month && self.current_month.clone().unwrap().year == date_year;
                                        let is_current_day = u32::try_from(full_day).unwrap() == date.day();
                                        //println!("{} {} {}", u32::try_from(full_day).unwrap(), date.day(), is_current_day);
                                        egui::Frame::new()
                                            .fill(if is_current_day && is_current_period {
                                                egui::Color32::from_rgb(255, 255, 0)
                                            } else {
                                                egui::Color32::from_rgb(0, 0, 0)
                                            })
                                            .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                                            .inner_margin(12.0)
                                            .show(ui, |ui| date_cell(self, ui, full_day));
                                        ui.end_row();
                                    }
                                });
                            });
                        }
                        //days
                        ui.horizontal(|ui| {
                            for remaining_days in (0..days % 7 + 1).rev() {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(0, 0, 0))
                                    .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        let full_day = days - remaining_days;
                                        let is_current_period = self.current_month.clone().unwrap().month == date_month && self.current_month.clone().unwrap().year == date_year;
                                        let is_current_day = u32::try_from(full_day).unwrap() == date.day();
                                        //println!("{} {} {}", u32::try_from(full_day).unwrap(), date.day(), is_current_day);
                                        egui::Frame::new()
                                            .fill(if is_current_day && is_current_period{
                                                egui::Color32::from_rgb(255, 255, 0)
                                            } else {
                                                egui::Color32::from_rgb(0, 0, 0)
                                            })
                                            // .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                                            // .inner_margin(12.0)
                                            .show(ui, |ui| date_cell(self, ui, full_day));
                                        ui.end_row();
                                        // ui.vertical(|ui| {
                                        //     ui.label(
                                        //         (days - remaining_days).to_string(),
                                        //     );
                                        //     ui.button("Edit");
                                        //     ui.button("Veiw events");
                                        // })
                                    });
                            }
                        });
                    });
                });
            if let Ok(settings) = self.database.get_settings() {
                if settings.enabled_websockets {
                    if ui.button("Forward all events over websocket").clicked() {
                        if let Ok(events) = self.database.get_events(){
                            for event in events {
                                forward_event(&self.rt, self.client.clone(), settings.clone(), event);
                            }
                        }
                    };
                }
            }
            //})
            // ui.heading("My egui Application");
            // ui.horizontal(|ui| {
            //     let name_label = ui.label("Your name: ");
            //     ui.text_edit_singleline(&mut self.name)
            //         .labelled_by(name_label.id);
            // });
            // ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            // if ui.button("Increment").clicked() {
            //     self.age += 1;
            // }
            // ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
        //}
    }
}
fn success_frame(ui: &mut egui::Ui, text: String) -> egui::InnerResponse<egui::Response> {
    egui::Frame::new()
    .fill(egui::Color32::from_rgb(0, 0, 0))
    .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
    .inner_margin(12.0)
    .show(ui, |ui| {
        ui.label(text)
    })
}
fn failure_frame(ui: &mut egui::Ui, text: String) -> egui::InnerResponse<egui::Response> {
    egui::Frame::new()
    .fill(egui::Color32::from_rgb(0, 0, 0))
    .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
    .inner_margin(12.0)
    .show(ui, |ui| {
        ui.label(text)
    })
}

fn date_cell(state: &mut MyApp, ui: &mut egui::Ui, day: u64) -> egui::InnerResponse<()> {
    ui.vertical(|ui: &mut egui::Ui| {
        ui.label(day.to_string());
        if let Some(month_time) = &state.current_month {
        let day_time = DayTime {
                        year: month_time.year,
                        month: month_time.month,
                        day,
                    };
        // println!("{}", highlight);
        // let color = {
        //     if highlight {
        //         egui::Color32::from_rgb(255, 255, 0)
        //     } else {
        //         egui::Color32::from_rgb(0, 0, 0)
        //     }
        // };
        if let Ok(events) = state.database.get_events_on_day(day_time.clone()) {
            for event in events {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(0, 0, 0))
                    .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(event.name)
                    });
            }
        }
        if ui.button("Events").clicked() {
            
                state.selected_date = Some(WorkingEvent {
                    time: day_time,
                    working_hour: String::new(),
                    working_minute: String::new(),
                    working_desc: String::new(),
                    working_name: String::new(),
                });
                state.show_events = true;
            }
        }
        // if ui.button("Veiw events").clicked() {
        // }
    })
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
fn forward_event(rt: &tokio::runtime::Runtime, client: reqwest::Client, settings: Settings, event: Event) {
    rt.spawn(async move {
        let _ = client
            .post(&settings.websocket_url)
            .header("authorization", &settings.websocket_header)
            .json(&event)
            .send()
            .await;
    });
}
//    egui::Frame::new()
//             .fill(egui::Color32::from_rgb(50, 50, 200))
//             .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
//             .inner_margin(12.0)
//             .show(ui, |ui| {
//                 ui.label("Hello inside a square!");
//                 ui.button("Click me");
//             });
