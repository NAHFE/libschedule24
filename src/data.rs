use serde_json::Value;
use serde::{Deserialize, Deserializer, Serialize};

use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<T> {
    pub error: Value,
    pub data: T,
    pub exception: Value,
    pub validation: Vec<Value>,
    pub session_expires: Value,
    pub need_session_refresh: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub text_list: Vec<Text>,
    pub box_list: Vec<Box>,
    pub line_list: Vec<Line>,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub lesson_info: Vec<LessonInfo>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Text {
    pub x: i64,
    pub y: i64,
    pub f_color: String,
    pub fontsize: f64,
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub id: i64,
    pub parent_id: i64,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Box {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub b_color: String,
    pub f_color: String,
    pub id: i64,
    pub parent_id: Option<i64>,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub lesson_guids: Option<Vec<String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub p1x: i64,
    pub p1y: i64,
    pub p2x: i64,
    pub p2y: i64,
    pub color: String,
    pub id: i64,
    pub parent_id: i64,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonInfo {
    pub guid_id: String,
    pub texts: Vec<String>,
    pub time_start: String,
    pub time_end: String,
    pub day_of_week_number: i64,
    pub block_name: String,
    #[serde(default)]
    pub block: Box,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassList {
    // pub courses: Vec<Value>,
    // pub subjects: Vec<Value>,
    // pub periods: Vec<Value>,
    // pub groups: Vec<Value>,
    pub classes: Vec<Class>,
    // pub rooms: Vec<Value>,
    // pub teachers: Vec<Value>,
    // pub students: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Class {
    // pub id: Value,
    pub group_guid: String,
    pub group_name: String,
    // pub absence_message_not_delivered_count: i64,
    // pub is_responsible: bool,
    // pub is_class: bool,
    // pub is_admin: bool,
    // pub is_principal: bool,
    // pub is_mentor: bool,
    // pub is_preschool_group: bool,
    // pub teachers: Value,
    // pub selectable_by: Value,
    // pub substitute_teacher_guid: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainInfo {
    // pub errors: Value,
    // pub validation_errors: Value,
    #[serde(rename = "getTimetableViewerUnitsResponse")]
    pub domain_school_list: SchoolList,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolList {
    pub host_name: String,
    pub units: Vec<School>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct School {
    pub unit_guid: String,
    pub unit_id: String,
    // pub allow_calendar_export: bool,
    // pub private: Value,
    // pub staff: Value,
    // pub anonymous: Anonymous,
}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Anonymous {
//     pub students: bool,
//     pub classes: bool,
//     pub groups: bool,
//     pub teachers: bool,
//     pub rooms: bool,
//     pub subjects: bool,
//     pub courses: bool,
// }


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub id: u32,
    pub description: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorData {
    pub errors: Value,
    pub validation_errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum APIResult<T> {
    Success(T),
    Failure(ErrorData),
}

#[derive(Debug)]
pub enum SchemaError {
    API(ErrorData),
    Request(reqwest::Error),
    APIRoot,
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SchemaError {}

impl From<ErrorData> for SchemaError {
    fn from(v: ErrorData) -> Self {
        Self::API(v)
    }
}

impl From<reqwest::Error> for SchemaError {
    fn from(v: reqwest::Error) -> Self {
        Self::Request(v)
    }
}

impl<T> TryFrom<Response<APIResult<T>>> for Response<T> {
    type Error = SchemaError;

    fn try_from(v: Response<APIResult<T>>) -> Result<Response<T>, SchemaError> {
        if !v.error.is_null() {
            return Err(SchemaError::APIRoot);
        }

        match v.data {
            APIResult::Success(x) => Ok(Response {
                data: x,
                error: v.error,
                exception: v.exception,
                validation: v.validation,
                session_expires: v.session_expires,
                need_session_refresh: v.need_session_refresh,
            }),
            APIResult::Failure(e) => Err(SchemaError::API(e))
        }
    }
}


fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}
