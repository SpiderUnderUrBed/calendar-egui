use std::error::Error;

use chrono::Month;
use serde::{Deserialize, Serialize};

use crate::DayTime;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
enum SerializeableMonth {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SimpleTimes {
    Year(u64),
    Month(u64),
    Week(u64),
    Day(u64),
    Hour(u64),
    Minuite(u64),
    Stop(Box<SimpleTimes>),
    None,
}
// impl From<&str> for SimpleTimes {
//     fn from(value: &str) -> Self {
//         if value.len() > 0 {
//             match value.chars().last().unwrap() {
//                 "h" => Self::Hour(())
//                 _ => Self::Minuite(0)
//             }
//         } else {
//             SimpleTimes::Minuite(0)
//         }
//     }
// }

impl Into<String> for SerializeableMonth {
    fn into(self) -> String {
        match self {
            SerializeableMonth::January => "January".into(),
            SerializeableMonth::February => "February".into(),
            SerializeableMonth::March => "March".into(),
            SerializeableMonth::April => "April".into(),
            SerializeableMonth::May => "May".into(),
            SerializeableMonth::June => "June".into(),
            SerializeableMonth::July => "July".into(),
            SerializeableMonth::August => "August".into(),
            SerializeableMonth::September => "September".into(),
            SerializeableMonth::October => "October".into(),
            SerializeableMonth::November => "November".into(),
            SerializeableMonth::December => "December".into(),
        }
    }
}
impl From<&str> for SerializeableMonth {
    fn from(value: &str) -> Self {
        match value {
            "January" => SerializeableMonth::January,
            "February" => SerializeableMonth::February,
            "March" => SerializeableMonth::March,
            "April" => SerializeableMonth::April,
            "May" => SerializeableMonth::May,
            "June" => SerializeableMonth::June,
            "July" => SerializeableMonth::July,
            "August" => SerializeableMonth::August,
            "September" => SerializeableMonth::September,
            "October" => SerializeableMonth::October,
            "November" => SerializeableMonth::November,
            "December" => SerializeableMonth::December,
            _ => SerializeableMonth::January,
        }
    }
}
impl Into<Month> for SerializeableMonth {
    fn into(self) -> Month {
        match self {
            SerializeableMonth::January => Month::January,
            SerializeableMonth::February => Month::February,
            SerializeableMonth::March => Month::March,
            SerializeableMonth::April => Month::April,
            SerializeableMonth::May => Month::May,
            SerializeableMonth::June => Month::June,
            SerializeableMonth::July => Month::July,
            SerializeableMonth::August => Month::August,
            SerializeableMonth::September => Month::September,
            SerializeableMonth::October => Month::October,
            SerializeableMonth::November => Month::November,
            SerializeableMonth::December => Month::December,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct EventTime {
    year: u64,
    month: SerializeableMonth,
    day: u64,
    hour: u64,
    minuite: u64,
}
impl Into<DayTime> for EventTime {
    fn into(self) -> DayTime {
        DayTime {
            year: self.get_year(),
            month: self.get_month(),
            day: self.get_day(),
        }
    }
}

impl EventTime {
    pub fn new(year: u64, month: Month, day: u64, hour: u64, minuite: u64) -> EventTime {
        EventTime {
            year,
            month: EventTime::map_chrono_month(month),
            day,
            hour,
            minuite,
        }
    }
    pub fn into_day_time(self) -> DayTime {
        self.into()
    }
    pub fn map_chrono_month(month: Month) -> SerializeableMonth {
        let mapped_month = match month {
            Month::January => SerializeableMonth::January,
            Month::February => SerializeableMonth::February,
            Month::March => SerializeableMonth::March,
            Month::April => SerializeableMonth::April,
            Month::May => SerializeableMonth::May,
            Month::June => SerializeableMonth::June,
            Month::July => SerializeableMonth::July,
            Month::August => SerializeableMonth::August,
            Month::September => SerializeableMonth::September,
            Month::October => SerializeableMonth::October,
            Month::November => SerializeableMonth::November,
            Month::December => SerializeableMonth::December,
        };
        mapped_month
    }
    pub fn set_month(&mut self, month: Month) {
        self.month = EventTime::map_chrono_month(month);
    }
    pub fn set_day(&mut self, day: u64) {
        self.day = day;
    }
    pub fn set_hour(&mut self, hour: u64) {
        self.hour = hour;
    }
    pub fn set_minute(&mut self, minute: u64) {
        self.minuite = minute;
    }
    pub fn set_year(&mut self, year: u64) {
        self.year = year;
    }
    pub fn get_year(&self) -> u64 {
        self.year
    }
    pub fn get_hour(&self) -> u64 {
        self.hour
    }
    pub fn get_day(&self) -> u64 {
        self.day
    }
    pub fn get_minute(&self) -> u64 {
        self.minuite
    }
    pub fn get_month(&self) -> Month {
        let mapped_month = match self.month {
            SerializeableMonth::January => Month::January,
            SerializeableMonth::February => Month::February,
            SerializeableMonth::March => Month::March,
            SerializeableMonth::April => Month::April,
            SerializeableMonth::May => Month::May,
            SerializeableMonth::June => Month::June,
            SerializeableMonth::July => Month::July,
            SerializeableMonth::August => Month::August,
            SerializeableMonth::September => Month::September,
            SerializeableMonth::October => Month::October,
            SerializeableMonth::November => Month::November,
            SerializeableMonth::December => Month::December,
        };
        mapped_month
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Event {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) time: EventTime,
    pub(crate) notify_times: Vec<SimpleTimes>,
    pub(crate) repeat_times: Vec<SimpleTimes>,
    pub(crate) repeat_until_current_day: bool,
    pub(crate) mirror_times: Vec<SimpleTimes>,
    pub(crate) freeze_at_time: Option<EventTime>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Settings {
    pub(crate) enabled_webhooks: bool,
    pub(crate) webhook_url_push: String,
    pub(crate) webhook_url_all: String,
    pub(crate) override_local_webhook: bool,
    pub(crate) webhook_url_remove: String,
    pub(crate) webhook_header: String,
}

pub trait EventsDatabase {
    fn add_event(&self, event: Event) -> Result<(), Box<dyn Error>>;
    fn remove_event_by_name(&self, event_name: String) -> Result<(), Box<dyn Error>>;
    fn set_events(&self, events: Vec<Event>) -> Result<(), Box<dyn Error>>;
    fn get_events(&self) -> Result<Vec<Event>, Box<dyn Error>>;
    fn get_events_on_day(&self, time: DayTime) -> Result<Vec<Event>, Box<dyn Error>>;
    fn remove_event_by_time(&self, time: EventTime) -> Result<(), Box<dyn Error>>;
}
pub trait SettingsDatabase {
    fn set_settings(&self, value: Settings) -> Result<(), Box<dyn Error>>;
    fn get_settings(&self) -> Result<Settings, Box<dyn Error>>;
}
