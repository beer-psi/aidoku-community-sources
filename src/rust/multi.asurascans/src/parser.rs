use crate::{helper::status_map, BASE_URL};
use aidoku::{
	error::{AidokuError, AidokuErrorKind, Result},
	prelude::format,
	std::{ObjectRef, Vec},
	Manga, MangaViewer,
};
use alloc::string::ToString;

pub fn parse_entry(obj: ObjectRef) -> Result<Manga> {
	let slug = obj.get("slug").as_string()?.read();
	let seriesid = if let Ok(numeric_id) = obj.get("id").as_int() {
		numeric_id.to_string()
	} else if let Ok(seriesid) = obj.get("seriesid").as_string() {
		seriesid.read()
	} else {
		return Err(AidokuError {
			reason: AidokuErrorKind::Unimplemented,
		});
	};

	let title = obj.get("title").as_string()?.read();
	let cover = obj.get("image").as_string()?.read();
	let status = status_map(obj.get("status").as_string()?.read());

	let author = obj
		.get("author")
		.as_string()
		.map(|v| v.read())
		.unwrap_or_default();
	let description = obj
		.get("description")
		.as_string()
		.map(|v| v.read())
		.unwrap_or_default();

	let genres = obj.get("genres").as_object()?;
	let categories = genres
		.values()
		.filter_map(|g| g.as_string().map(|v| v.read()).ok())
		.collect::<Vec<_>>();

	Ok(Manga {
		id: format!("{seriesid}-{slug}"),
		title,
		author,
		cover,
		description,
		url: format!("{BASE_URL}/comics/{seriesid}-{slug}"),
		categories,
		status,
		viewer: MangaViewer::Scroll,
		..Default::default()
	})
}
