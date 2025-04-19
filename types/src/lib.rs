use chrono::{DateTime, Local};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FaxMessage {
    pub time: DateTime<Local>,
    pub message: String,
    pub from: String,
    pub ip: String,
}
