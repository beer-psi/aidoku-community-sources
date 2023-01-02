#![no_std]
#![feature(let_chains, lint_reasons)]
#![allow(clippy::mut_range_bound)]
extern crate alloc;
mod helper;
mod parser;
use aidoku::{
	error::{Result, AidokuError},
	prelude::*,
	std::{copy, json, net::Request, String, ValueRef, Vec},
	Chapter, DeepLink, Filter, FilterType, Manga, MangaPageResult, Page,
};
use alloc::string::ToString;
use helper::incl_excl_check;
use parser::parse_entry;

static BASE_URL: &str = "https://beta.asurascans.com";
static IMG_URL: &str = "https://img.asurascans.com";

static mut DIRECTORY_RID: i32 = -1;
static mut BUILD_ID: Option<String> = None;
static mut CACHED_MANGA_ID: Option<String> = None;
static mut CACHED_MANGA: Option<String> = None;

pub fn get_data_href<T: AsRef<str>>(link: T) -> Option<String> {
	unsafe {
		let link = link.as_ref();
		BUILD_ID
			.as_ref()
			.map(|build_id| format!("{BASE_URL}/_next/data/{build_id}{link}"))
	}
}

fn populate_build_id() -> Result<()> {
	let document = Request::get(BASE_URL).header("Referer", BASE_URL).html()?;

	let next_data_node = document.select("#__NEXT_DATA__");

	let next_data = json::parse(next_data_node.html().read())?;
	let next_data_obj = next_data.as_object()?;

	let build_id_ref = next_data_obj.get("buildId").as_string()?;
	let build_id = build_id_ref.read();
	unsafe {
		BUILD_ID = Some(build_id);
	}
	Ok(())
}

fn initialize_directory() -> Result<()> {
	let build_id = unsafe { BUILD_ID.clone().expect("BUILD_ID not populated") };

	let directory = Request::get(format!("{BASE_URL}/_next/data/{build_id}/comics.json"))
		.header("Referer", BASE_URL)
		.json()?;
	let directory_obj = directory.as_object()?;

	let page_props = directory_obj.get("pageProps").as_object()?;

	#[allow(clippy::redundant_clone, reason = "Needed to not drop")]
	let mut series_list = page_props.get("seriesList").clone();
	series_list.1 = false;
	unsafe {
		DIRECTORY_RID = series_list.0;
	}
	Ok(())
}

// Cache manga page html
pub fn cache_manga_page(id: &str) -> Result<()> {
	if unsafe { CACHED_MANGA.is_some() } && unsafe { CACHED_MANGA_ID.clone().unwrap() } == id {
		return Ok(());
	}
	let url =
		get_data_href(format!("/comics/{id}.json?seriesid={id}")).expect("BUILD_ID not populated");
	unsafe {
		CACHED_MANGA = Some(Request::get(url).header("Referer", BASE_URL).string()?);
		CACHED_MANGA_ID = Some(String::from(id));
	}
	Ok(())
}

#[initialize]
fn initialize() {
	populate_build_id().expect("Failed to populate BUILD_ID");
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
	if unsafe { DIRECTORY_RID } < 0 {
		initialize_directory().ok();
	}
	let mut directory = unsafe { ValueRef::new(copy(DIRECTORY_RID)) }
		.as_array()
		.expect("Directory should be an array");

	let offset = (page as usize - 1) * 12;

	let mut query = String::new();
	let mut included_genres: Vec<String> = Vec::new();
	let mut excluded_genres: Vec<String> = Vec::new();
	let mut included_statuses: Vec<String> = Vec::new();
	let mut excluded_statuses: Vec<String> = Vec::new();
	for filter in filters {
		match filter.kind {
			FilterType::Title => {
				query = filter
					.value
					.as_string()
					.unwrap_or_default()
					.read()
					.trim()
					.to_lowercase();
			}
			FilterType::Genre => {
				if let Ok(id) = filter.object.get("id").as_string() {
					let id = id.read();
					match filter.value.as_int().unwrap_or(-1) {
						0 => excluded_genres.push(id),
						1 => included_genres.push(id),
						_ => continue,
					}
				}
			}
			FilterType::Check => match filter.value.as_int().unwrap_or(-1) {
				0 => excluded_statuses.push(filter.name),
				1 => included_statuses.push(filter.name),
				_ => continue,
			},
			_ => continue,
		}
	}

	let mut i = 0;
	let mut size = directory.len();
	for _ in 0..size {
		if i >= size || i >= offset + 12 {
			break;
		}
		let entry = directory.get(i);
		if let Ok(entry_object) = entry.as_object()
		   && let Ok(title) = entry_object.get("title").as_string()
		   && let Ok(genres) = entry_object.get("genres").as_object()
		   && let Ok(status) = entry_object.get("status").as_string() {

			let title = title.read();
			if !query.is_empty() && !title.to_lowercase().contains(&query) {
				directory.remove(i);
				size -= 1;
				continue
			}

			let status = status.read();
			if (!included_statuses.is_empty() || !excluded_statuses.is_empty())
			   && !incl_excl_check(&status, &included_statuses, &excluded_statuses) {
				directory.remove(i);
				size -= 1;
				continue
			}

			if !included_genres.is_empty() || !excluded_genres.is_empty() {
				let genre_ids = genres.keys();
				for genre_id in genre_ids {
					if let Ok(id) = genre_id.as_string() {
						let id = id.read();
						if !incl_excl_check(&id, &included_genres, &excluded_genres) {
							directory.remove(i);
							size -= 1;
							continue
						}
					}
				}
			}
			i += 1;
		} else {
			directory.remove(i);
			size -= 1;
		}
	}

	let end = if directory.len() > offset + 12 {
		offset + 12
	} else {
		directory.len()
	};

	let mut manga: Vec<Manga> = Vec::with_capacity(12);
	for i in offset..end {
		if let Ok(obj) = directory.get(i).as_object()
		   && let Ok(entry) = parse_entry(obj) {
			manga.push(entry);
		}
	}

	Ok(MangaPageResult {
		manga,
		has_more: directory.len() > end,
	})
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
	cache_manga_page(&id).ok();
	let data = json::parse(unsafe { CACHED_MANGA.clone().expect("CACHED_MANGA should exist") })?
		.as_object()?;

	let page_props = data.get("pageProps").as_object()?;
	let series = page_props.get("series").as_object()?;
	parse_entry(series)
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
	cache_manga_page(&id).ok();
	let data = json::parse(unsafe { CACHED_MANGA.clone().expect("CACHED_MANGA should exist") })?
		.as_object()?;

	let page_props = data.get("pageProps").as_object()?;
	let series = page_props.get("series").as_object()?;

	let chapters = series.get("chapters").as_array()?;
	Ok(chapters
		.filter_map(|chapter| {
			if let Ok(chapter) = chapter.as_object() {
				let link = chapter
					.get("link")
					.as_string()
					.map(|v| v.read())
					.unwrap_or_default();
				let id = link
					.split('/')
					.last()
					.expect("Should have last element")
					.to_string();
				let title = chapter
					.get("chapterTitle")
					.as_string()
					.map(|v| v.read())
					.unwrap_or_default();
				let chapter = chapter.get("order").as_float().unwrap_or(-1.0) as f32;
				Some(Chapter {
					id,
					title,
					chapter,
					url: format!("{BASE_URL}{link}"),
					..Default::default()
				})
			} else {
				None
			}
		})
		.collect::<Vec<_>>())
}

#[get_page_list]
fn get_page_list(manga_id: String, id: String) -> Result<Vec<Page>> {
	let document = Request::get(format!("{BASE_URL}/read/{manga_id}/{id}"))
		.header("Referer", BASE_URL)
		.html()?;

	let next_data_node = document.select("#__NEXT_DATA__");
	let next_data = json::parse(next_data_node.html().read())?;
	let next_data_obj = next_data.as_object()?;

	let props = next_data_obj.get("props").as_object()?;
	let page_props = props.get("pageProps").as_object()?;

	let mut chapter_list = page_props.get("chapterList").as_array()?;
	let numeric_id = chapter_list
		.find_map(|chapter| {
			if let Ok(chapter) = chapter.as_object() {
				let slug = chapter
					.get("slug")
					.as_string()
					.map(|v| v.read())
					.unwrap_or_default();
				if slug == id {
					Some(chapter.get("id").as_int().unwrap_or(-1))
				} else {
					None
				}
			} else {
				None
			}
		})
		.expect("Should find numeric chapter ID");

	let pages_array = page_props.get("data").as_array()?;
	Ok(pages_array
		.filter_map(|obj| {
			if let Ok(obj) = obj.as_object()
		       && let Ok(uuid) = obj.get("uuid").as_string() 
		       && let Ok(order) = obj.get("order").as_int() {
			Some(Page {
				index: order as i32 - 1,
				url: format!("{IMG_URL}/pages/{numeric_id}/{uuid}.jpg"),
				..Default::default()
			})
		} else {
			None
		}
		})
		.collect::<Vec<_>>())
}

#[modify_image_request]
fn modify_image_request(request: Request) {
	request.header("Referer", BASE_URL);
}

#[handle_url]
fn handle_url(url: String) -> Result<DeepLink> {
	if url.contains("/comics/") {
		let id = url.split('/').last().expect("url should have last element").to_string();
		Ok(DeepLink { manga: get_manga_details(id).ok(), chapter: None })
	} else if url.contains("/read/") {
		let elements = url.split('/').collect::<Vec<_>>();
		let id = elements[5].to_string();
		let manga_id = elements[4].to_string();
		Ok(DeepLink {
			manga: get_manga_details(manga_id).ok(),
			chapter: Some(Chapter { id, ..Default::default() })
		})
	} else {
		Err(AidokuError { reason: aidoku::error::AidokuErrorKind::Unimplemented })
	}
}
