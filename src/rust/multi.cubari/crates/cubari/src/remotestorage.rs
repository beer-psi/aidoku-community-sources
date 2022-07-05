use aidoku::{
    prelude::format,
    error::Result,
    std::{String, Vec, net::{Request, HttpMethod}, defaults::*},
    Manga, StringRef
};
use alloc::string::ToString;
use crate::database;

pub struct RemoteStorage {
    url: String,
    token: String,
}

impl RemoteStorage {
    fn new<T: AsRef<str>>(url: T, token: T) -> Self {
        Self {
            url: url.as_ref().to_string(),
            token: token.as_ref().to_string(),
        }
    }

    fn get_all_series(&self) -> Result<Vec<Manga>> {
        let json = Request::new(&self.url, HttpMethod::Get)
            .header("Authorization", &format!("Bearer {}", self.token))
            .json()
            .as_object()?;
        let items = json.get("items").as_object()?;
        let series = items.get("series/").as_object()?;
        let revision = series.get("ETag").as_string()?.read();
        if defaults_get("history.revision").as_string().unwrap_or_else(|_| StringRef::from("")).read() == revision {
            Ok(
                database::series_list()
                .unwrap_or_default()
                .iter()
                .filter_map(|series| database::get_manga(series).ok())
                .collect::<Vec<_>>()
            )
        } else {
            defaults_set("history.revision", StringRef::from(revision).0);
            let json = Request::new(
                &format!("{}/series/", self.url),
                HttpMethod::Get
            )
            .header("Authorization", &format!("Bearer {}", self.token))
            .json()
            .as_object()?;
            let items = json.get("items").as_object()?;
        }
        

    }
}

