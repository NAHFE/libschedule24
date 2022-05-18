pub mod data;
#[cfg(feature = "svg")]
pub mod image;

use std::{convert::TryInto, str::FromStr};
use std::fmt;

use chrono:: {Local, NaiveTime, Datelike, Utc};

macro_rules! impl_from {
    ($e:ty, $enum:tt) => {
        impl From<$e> for RequestError {
            fn from(v: $e) -> Self {
                Self::$enum(v)
            }
        }
    };
}

#[derive(Debug)]
pub struct EmptyError {}

#[derive(Debug)]
pub enum RequestError {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Schema(data::SchemaError),
    BaseDirectories(xdg::BaseDirectoriesError),
    IO(std::io::Error),
    Utf8(std::str::Utf8Error),
    Cacache(cacache::Error),
    ParseInt(std::num::ParseIntError),
    Empty(EmptyError),
}

impl_from!(reqwest::Error, Reqwest);
impl_from!(serde_json::Error, Serde);
impl_from!(data::SchemaError, Schema);
impl_from!(xdg::BaseDirectoriesError, BaseDirectories);
impl_from!(std::io::Error, IO);
impl_from!(std::str::Utf8Error, Utf8);
impl_from!(cacache::Error, Cacache);
impl_from!(std::num::ParseIntError, ParseInt);
impl_from!(EmptyError, Empty);

pub async fn get_key() -> Result<String, RequestError>{
    let client = reqwest::Client::new();
    let res = client
        .get("https://web.skola24.se/api/get/timetable/render/key")
        .header("X-Scope", "8a22163c-8662-4535-9050-bc5e1923df48")
        .send()
        .await?
        .error_for_status()?;

    let key_res: serde_json::Value = serde_json::from_str(&res.text().await?)?;
    let key = key_res["data"]["key"].as_str().unwrap().to_string();

    Ok(key)
}

#[derive(Copy, Clone)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Default for Dimensions {
    #[inline]
    fn default() -> Self {
        Dimensions {
            width: 800,
            height: 600,
        }
    }
}

#[derive(Debug)]
pub struct ParseDimensionError;

impl fmt::Display for ParseDimensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid dimensions specified")
    }
}

impl From<std::num::ParseIntError> for ParseDimensionError {
    fn from(_: std::num::ParseIntError) -> Self {
        ParseDimensionError
    }
}

impl std::error::Error for ParseDimensionError {}

impl FromStr for Dimensions {
    type Err = ParseDimensionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('x');

        Ok(Dimensions {
            width: split.next() .ok_or(ParseDimensionError)?.parse()?,
            height: split.next().ok_or(ParseDimensionError)?.parse()?,
        })
    }
}

pub async fn domain_exists(domain: &str, should_cache: bool) -> Result<bool, RequestError> {
    let result = get_schools(domain, should_cache).await;
    match result {
        Ok(_) => Ok(true),
        Err(RequestError::Schema(e)) => {
            if let data::SchemaError::API(e) = e {
                if e.validation_errors.len() == 1 && e.validation_errors[0].id == 1 {
                    Ok(false)
                } else {
                    Err(RequestError::Schema(data::SchemaError::API(e)))
                }
            }
            else {
                Err(RequestError::Schema(e))
            }
        },
        Err(e) => Err(e),
    }
}

pub async fn school_exists(domain: &str, school: &str, should_cache: bool) -> Result<bool, RequestError> {
    let schools = get_schools(domain, should_cache).await?;
    for s in schools {
        if school == s.unit_id {
            return Ok(true)
        }
    }
    Ok(false)
}

pub async fn class_exists(domain: &str, school: &str, class: &str, should_cache: bool) -> Result<bool, RequestError> {
    let classes = get_classes(domain, &get_school_guid(domain, school, should_cache).await?, should_cache).await?;
    for c in classes {
        if class == c.group_name {
            return Ok(true)
        }
    }
    Ok(false)
}

pub async fn cache_request(ckey: String, reqdata: serde_json::value::Value, api: &str, post: bool, should_cache: bool) -> Result<String, RequestError> {
    let cache = xdg::BaseDirectories::new()?
                                    .create_cache_directory(env!("CARGO_PKG_NAME"))?
                                    .to_str().unwrap().to_owned();
    let data = if should_cache {
        match cacache::read(&cache, &ckey).await {
            Ok(data) => Ok(data),
            Err(e) => Err(RequestError::Cacache(e))
        }
    }
    else {
        Err(RequestError::Empty(EmptyError{}))
    };

    match data {
        Ok(data) => Ok(std::str::from_utf8(&data)?.to_owned()),
        Err(_) => {
            let data = {
                let client = reqwest::Client::new();
                let mut reqdata = reqdata;
                reqdata["renderKey"] = serde_json::json!(get_key().await?);
                let client = if post {
                    client.post("https://web.skola24.se/api".to_string() + api)
                }
                else {
                    client.get("https://web.skola24.se/api".to_string() + api)
                };

                client
                    .header("Content-Type", "application/json")
                    .header("X-Scope", "8a22163c-8662-4535-9050-bc5e1923df48")
                    .json(&reqdata)
                    .send()
                    .await?
                    .error_for_status()?
                    .text().await?
            };

            cacache::write(&cache, &ckey, &data).await?;
            Ok(data)
        }
    }
}

pub async fn get_schema(selection: (String, String, String), day_of_week: i32, week: i32, dimensions: Option<Dimensions>, should_cache: bool) -> Result<data::Response<data::Schema>, RequestError> {
    let ckey = (&selection.0).to_string() + &selection.1 + &selection.2 + &week.to_string() + &day_of_week.to_string();
    let dimensions = dimensions.unwrap_or_default();
    let now = Local::now();
    let data = serde_json::json!({
        "host": selection.0,
        "unitGuid": selection.1,
        "scheduleDay": day_of_week,
        "blackAndWhite": false,
        "width": dimensions.width,
        "height": dimensions.height,
        "selectionType": 0,
        "selection": selection.2,
        "showHeader": false,
        "periodText": "",
        "week": week,
        "year": now.year(),
        "privateSelectionMode": false,
        "customerKey": "",
    });

    let data = cache_request(ckey, data, "/render/timetable", false, should_cache).await?;
    match serde_json::from_str::<data::Response<data::Schema>>(&data) {
        Ok(data) => Ok(data),
        Err(err) => Err(RequestError::Serde(err))
    }
}

pub async fn get_classes(domain: &str, unit_guid: &str, should_cache: bool) -> Result<Vec<data::Class>, RequestError> {
    let ckey = Utc::now().format("%Y%m%d").to_string() + domain + unit_guid;

    let data = serde_json::json!({
        "hostName": domain,
        "unitGuid": unit_guid,
        "filters": {"class":true}
    });

    let data = cache_request(ckey, data, "/get/timetable/selection", false, should_cache).await?;
    let result: data::Response<data::ClassList> = serde_json::from_str::<data::Response<data::APIResult<data::ClassList>>>(&data)?.try_into()?;

    Ok(result.data.classes)
}

pub async fn get_schools(domain: &str, should_cache: bool) -> Result<Vec<data::School>, RequestError> {
    let ckey = Utc::now().format("%Y%m%d").to_string() + domain;
    let data: serde_json::Value = serde_json::json!({
        "getTimetableViewerUnitsRequest": {"hostName": domain}
    });

    let data = cache_request(ckey, data, "/services/skola24/get/timetable/viewer/units", true, should_cache).await?;
    let result: data::Response<data::DomainInfo> = serde_json::from_str::<data::Response<data::APIResult<data::DomainInfo>>>(&data)?.try_into()?;

    Ok(result.data.domain_school_list.units)
}

pub async fn get_class_guid(domain: &str, unit_guid: &str, name: &str, should_cache: bool) -> Result<String, RequestError> {
    let classes = get_classes(domain, unit_guid, should_cache).await?;

    for class in classes {
        if class.group_name == name {
            return Ok(class.group_guid);
        }
    }
    Ok(String::new())
}

pub async fn get_school_guid(domain: &str, name: &str, should_cache: bool) -> Result<String, RequestError> {
    let schools = get_schools(domain, should_cache).await?;

    for school in schools {
        if school.unit_id == name {
            return Ok(school.unit_guid);
        }
    }
    Ok(String::new())
}

pub fn print_lessons(lesson_info: &[data::LessonInfo], next_day: bool) -> Result<(), reqwest::Error> {
    let now = if next_day {NaiveTime::from_hms(0,0,0)}
    else {Local::now().time()};

    let mut next_lesson_time = NaiveTime::from_hms(23, 59, 59);
    let mut next_lesson = 0;

    let mut current_lesson_bool = false;
    let mut next_lesson_bool = false;

    for (i, lesson) in lesson_info.iter().enumerate() {
        let time_start = NaiveTime::parse_from_str(&lesson.time_start.to_string(), "%H:%M:%S").unwrap_or_else(|_| panic!("Failed to parse time!"));
        let time_end = NaiveTime::parse_from_str(&lesson.time_end.to_string(), "%H:%M:%S").unwrap_or_else(|_| panic!("Failed to parse time!"));

        if time_start > now {
            if time_start < next_lesson_time {
                next_lesson_bool = true;
                next_lesson_time = time_start;
                next_lesson = i;
            }
        }
        else if time_end > now {
            current_lesson_bool = true;
            print!("{}-{}", &lesson.texts[0].to_string()[..3], time_end.format("%H:%M"));
        };
    }

    if next_lesson_bool {
        if current_lesson_bool {
            print!(", ");
        }
        println!("{}-{}", next_lesson_time.format("%H:%M"), &lesson_info[next_lesson].texts[0].to_string()[..3]);
    }
    else {
        println!();
    }

    Ok(())
}

pub async fn get_lesson_info(selection: (String, String, String), day: i32, week: i32, should_cache: bool) -> Result<Vec<data::LessonInfo>, RequestError> {
    let schema = get_schema(selection, day, week, None, should_cache).await?;
    let lesson_info = add_box_info(&schema.data)?;

    Ok(lesson_info)
}

fn add_box_info(data: &data::Schema) -> Result<Vec<data::LessonInfo>, RequestError> {
    let mut lesson_info = data.lesson_info.clone();
    for i in 0..data.lesson_info.len() {
        for j in 0..data.box_list.len() {
            if data.box_list[j].type_field != "Lesson" {continue;}
            for k in 0..data.box_list[j].lesson_guids.as_ref().unwrap().len() {
                if data.lesson_info[i].guid_id == data.box_list[j].lesson_guids.as_ref().unwrap()[k] {
                    lesson_info[i].block = data.box_list[j].clone();
                }
            }
        }
    }

    Ok(lesson_info)
}
