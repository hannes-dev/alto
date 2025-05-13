use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct FaxMessage {
    pub time: DateTime<Local>,
    pub message: String,
    pub from: String,
    pub ip: String,
}
