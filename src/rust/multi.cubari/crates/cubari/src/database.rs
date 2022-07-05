use aidoku::{
	error::Result,
	prelude::format,
	std::{defaults::*, ArrayRef, ObjectRef, String, StringRef, Vec},
	Manga,
};

pub fn initialize() {
	if defaults_get("history.series").is_none() {
		defaults_set("history.series", ArrayRef::new().0);
	}
}

pub fn series_list() -> Result<Vec<String>> {
	let series = defaults_get("history.series").as_array()?;
	let mut result = Vec::with_capacity(series.len());
	for item in series {
		result.push(item.as_string()?.read());
	}
	Ok(result)
}

pub fn add_or_update_manga(manga: &Manga) -> Result<()> {
	if aidoku::std::defaults::defaults_get("saveSeries")
		.as_bool()
		.unwrap_or(true)
	{
		let key = String::from(&manga.id);

		// Add manga in index if it doesn't already exist
		if !series_list().unwrap_or_default().contains(&key) {
			let mut series = defaults_get("history.series").as_array()?;
			series.insert(StringRef::from(&key).0);
			defaults_set("history.series", series.0);
		}

		// Update manga in index
		let mut obj = if let Ok(object) = defaults_get(&format!("history.{key}")).as_object() {
			object
		} else {
			ObjectRef::new()
		};
		obj.set("cover", StringRef::from(&manga.cover).0);
		obj.set("title", StringRef::from(&manga.title).0);
		defaults_set(&format!("history.{key}"), obj.0);
	}
	Ok(())
}

pub fn get_manga<T: AsRef<str>>(id: T) -> Result<Manga> {
	let id = id.as_ref();
	let obj = defaults_get(&format!("history.{id}")).as_object()?;
	let cover = obj.get("cover").as_string()?.read();
	let title = obj.get("title").as_string()?.read();
	Ok(Manga {
		id: String::from(id),
		cover,
		title,
		author: String::new(),
		artist: String::new(),
		description: String::new(),
		url: String::new(),
		categories: Vec::new(),
		status: aidoku::MangaStatus::Unknown,
		viewer: aidoku::MangaViewer::Rtl,
		nsfw: aidoku::MangaContentRating::Safe,
	})
}

pub fn delete_all_manga() -> Result<()> {
	let series = defaults_get("history.series").as_array()?;
	for item in series {
		let id = item.as_string()?.read();
		defaults_set(&format!("history.{id}"), ObjectRef::new().0);
	}
	defaults_set("history.series", ArrayRef::new().0);
	Ok(())
}
