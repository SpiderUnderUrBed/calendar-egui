use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::error::Error;

use serde::Deserialize;
use serde::Serialize;

use crate::databasespec::Event;
use crate::EventTime;
use crate::DayTime;
use crate::databasespec::Settings;
use crate::databasespec::EventsDatabase;
use crate::databasespec::SettingsDatabase;
use crate::intersects_by_day;

#[derive(Clone)]
pub struct JsonBackend {
    file: PathBuf
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JsonBackendContent {
    events: Vec<Event>,
    settings: Settings
}

impl Default for JsonBackendContent {
    fn default() -> JsonBackendContent {
        JsonBackendContent {
            events: vec![],
            settings: Settings::default()
        }
    }
}

#[derive(Clone)]
pub struct Database {
    pub connection: JsonBackend
}
impl Default for JsonBackend {
    fn default() -> Self {
        JsonBackend {
            file: PathBuf::from("database.json")
        }
    }
}


impl JsonBackend {
    pub fn new(mut file: Option<PathBuf>) -> Self {
        if let Some(path) = &file {
            let _ = OpenOptions::new()
                .create(true)
                .write(true)
                .open(path);
        } else {
            let _ = OpenOptions::new()
                .create(true)
                .write(true)
                .open(JsonBackend::default().file);
            file = Some(JsonBackend::default().file);
        }
        let mut open_file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&file.clone().unwrap())
            .map_err(|e| format!("Failed to open file: {}", e)).unwrap();

        let mut rewrite_file: bool = false;
        let mut database: JsonBackendContent = 
            serde_json::from_reader(&open_file).unwrap_or_else(|e| {
                let mut backup_path = file.clone().unwrap();
                backup_path.set_extension("old");
                rewrite_file = true;
                println!("Failed to parse JSON: {}", e); 
                JsonBackendContent::default()
            });
        if rewrite_file {
            open_file.write_all(serde_json::to_string_pretty(&database).unwrap().as_bytes());
        }
        if file.is_some() {
            JsonBackend {
                file: file.unwrap()
            }
        } else {
            JsonBackend::default()
        }
    }
}

impl Database {
    pub fn new(conn: Option<JsonBackend>) -> Database {  
        let connection = conn.unwrap_or_default();
        Database {
            connection,
        }
    }
    pub fn fix_connection(conn: Option<JsonBackend>) -> Database {  
        let connection = conn.unwrap_or_default();
        Database {
            connection,
        }
    }
    pub fn ensure_database_conn(&self) -> Result<(), String> {
        if Path::new("database.json").exists(){
            Ok(())
        } else {
            let mut file = File::create("database.json").unwrap();
            file.write_all(&serde_json::to_vec(&JsonBackendContent::default()).unwrap())
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    } 
    pub fn clear_db(&self) -> Result<(), String> {
        let clear_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .read(true)
            .open(&self.connection.file)
            .map_err(|e| format!("Failed to open file: {}", e));
        if clear_file.is_err(){
            println!("{:#?}", clear_file);
            return Err("Error".to_string())
        }

        Ok(())
    }
    fn write_database(&self, database: JsonBackendContent) -> Result<String, String> {
        let file_path = &self.connection.file;
        let json = serde_json::to_string_pretty(&database)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)
            .map_err(|e| format!("Failed to open file for writing: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;

        file.sync_all()
            .map_err(|e| format!("Failed to sync data to disk: {}", e))?;

        Ok("Wrote file successfully".to_string())
    }
    fn get_database(&self) -> Result<JsonBackendContent, String> {
        let file_path = &self.connection.file;
    
        let mut read_file = File::open(file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let mut contents = String::new();
        read_file.read_to_string(&mut contents).map_err(|e| format!("Read error: {}", e))?;
    
        let database: Result<JsonBackendContent, String> = if contents.trim().is_empty() {
            Ok(JsonBackendContent::default())
        } else {
            //println!("{:#?}", contents.clone());
            let result: Result<JsonBackendContent, String> = serde_json::from_str(&contents).map_err(|e| {
                println!("Failed to parse JSON (1): {}", e); 
                format!("Error: {}", e)
            });
            if result.is_err(){
                Err(result.err().unwrap())
            } else {
                Ok(result?)
            }
        };
        database
    }
}

impl EventsDatabase for Database {
    fn add_event(&self, event: Event) -> Result<(), Box<dyn Error>>{
        let mut database = self.get_database()?;
        if !database.events.iter().any(|existing_event| {
            if existing_event.time.get_year() == event.time.get_year() && existing_event.time.get_month() == event.time.get_month() && existing_event.time.get_day() == event.time.get_day() {
                if existing_event.time.get_hour() == event.time.get_hour(){
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }){
            database.events.push(event)
        } else {
            return Err("You cannot have duplicate events".into());
        }
        self.write_database(database)?;
        Ok(())
    }
    fn remove_event_by_name(&self, event_name: String) -> Result<(), Box<dyn Error>>{
        let mut database = self.get_database()?;
        database.events.retain(|existing_event| existing_event.name != event_name);
        self.write_database(database)?;
        Ok(())
    }
    fn get_events(&self) -> Result<Vec<Event>, Box<dyn Error>>{
        let database = self.get_database()?;
        
        Ok(database.events)
    }
    fn set_events(&self, events: Vec<Event>) -> Result<(), Box<dyn Error>> {
        let mut database = self.get_database()?;
        database.events = events;
        self.write_database(database)?;
        Ok(())
    }
    fn get_events_on_day(&self, time: DayTime) -> Result<Vec<Event>, Box<dyn Error>>{
        let database = self.get_database()?;
        let events_that_day: Vec<Event> = database.events.iter().filter(|existing_events| 
            existing_events.time.clone().into_day_time() == time
            || intersects_by_day(existing_events.time.clone().into_day_time() , existing_events.repeat_times.clone(), time.clone(), existing_events.repeat_until_current_day)
        ).cloned().collect();
        Ok(events_that_day)
    }
    fn remove_event_by_time(&self, time: EventTime) -> Result<(), Box<dyn Error>>{
        let mut database = self.get_database()?;
        database.events.retain(|existing_event| existing_event.time != time);
        self.write_database(database)?;
        Ok(())
    }
}
impl SettingsDatabase for Database {
    fn set_settings(&self, value: Settings) -> Result<(), Box<dyn Error>> {
        let mut database = self.get_database()?;
        database.settings = value;
        self.write_database(database)?;
        Ok(())
    }
    fn get_settings(&self) -> Result<Settings, Box<dyn Error>> {
        let database = self.get_database()?;
        
        Ok(database.settings)
    }
}