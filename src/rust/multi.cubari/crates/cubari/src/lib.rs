#![no_std]
#![feature(let_chains)]
extern crate alloc;
mod database;
mod helper;
mod remotestorage;
use aidoku::{
	error::{AidokuError, AidokuErrorKind, Result},
	prelude::*,
	std::{
		net::{HttpMethod, Request},
		ArrayRef, Kind, ObjectRef, String, ValueRef, Vec, defaults::*, StringRef
	},
	Chapter, DeepLink, Filter, FilterType, Manga, MangaContentRating, MangaPageResult, MangaStatus,
	MangaViewer, Page,
};
use alloc::vec;
use float_ord::FloatOrd;
use helper::*;

static mut CACHED_SLUG: Option<String> = None;
static mut CACHED_JSON: Option<ObjectRef> = None;
fn cache_api_request<T: AsRef<str>>(slug: T) -> Result<()> {
	let slug = slug.as_ref();
	unsafe {
		if CACHED_JSON.is_some() && slug == CACHED_SLUG.clone().unwrap() {
			return Ok(());
		}

		let fragments = slug.split('/').collect::<Vec<_>>();

		CACHED_SLUG = Some(String::from(slug));
		match Request::new(
			&format!(
				"https://cubari.moe/read/api/{}/series/{}/",
				fragments[0], fragments[1]
			),
			HttpMethod::Get,
		)
		.json()
		.as_object()
		{
			Ok(obj) => {
				CACHED_JSON = Some(obj);
				Ok(())
			}
			Err(_) => Err(AidokuError {
				reason: AidokuErrorKind::Unimplemented,
			}),
		}
	}
}

#[no_mangle]
#[export_name = "initialize"]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn __wasm_initialize() {
	database::initialize();
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, _: i32) -> Result<MangaPageResult> {
	let mut query = String::new();
	for filter in filters {
		match filter.kind {
			FilterType::Title => query = filter.value.as_string()?.read(),
			_ => continue,
		}
	}
	let slug = url_to_slug(query);
	// Assume it's a title search
	if !slug.contains('/') {
		let series_list = database::series_list().unwrap_or_default();
		let mut manga: Vec<Manga> = Vec::new();
		if !series_list.is_empty() {
			for series in series_list {
				match database::get_manga(series) {
					Ok(res) => {
						if res.title.to_lowercase().contains(&slug) {
							manga.push(res);
						} else {
							continue;
						}
					}
					Err(_) => continue,
				}
			}
		}
		if aidoku::std::defaults::defaults_get("showHelp")
			.as_bool()
			.unwrap_or(true)
		{
			manga.push(cubari_guide())
		}
		Ok(MangaPageResult {
			manga,
			has_more: false,
		})
	} else {
		Ok(MangaPageResult {
			manga: match get_manga_details(slug) {
				Ok(manga) => vec![manga],
				Err(_) => Vec::new(),
			},
			has_more: false,
		})
	}
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
	if id == "aidoku/guide" {
		Ok(cubari_guide())
	} else {
		cache_api_request(&id)?;
		let json = unsafe { CACHED_JSON.clone().unwrap() };
		let manga = Manga {
			url: format!("https://cubari.moe/read/{}", &id),
			nsfw: if id.contains("nhentai") {
				MangaContentRating::Nsfw
			} else {
				MangaContentRating::Safe
			},
			id,
			cover: img_url_handler(json.get("cover").as_string()?.read()),
			title: json.get("title").as_string()?.read(),
			author: json.get("author").as_string()?.read(),
			artist: json.get("artist").as_string()?.read(),
			description: json.get("description").as_string()?.read(),
			categories: Vec::new(),
			status: MangaStatus::Unknown,
			viewer: MangaViewer::Rtl,
		};
		database::add_or_update_manga(&manga).ok();
		Ok(manga)
	}
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
	let chapters = if id == "aidoku/guide" {
		vec![Chapter {
			id: String::from("1"),
			title: String::from("Guide"),
			url: String::new(),
			volume: -1.0,
			chapter: -1.0,
			date_updated: -1.0,
			scanlator: String::new(),
			lang: String::from("en"),
		}]
	} else {
		cache_api_request(&id)?;
		let json = unsafe { CACHED_JSON.clone().unwrap() };

		let scanlators_map = json.get("groups").as_object()?;
		let chapters_object = json.get("chapters").as_object()?;
		let mut chapters = Vec::new();

		for chapter in chapters_object.keys() {
			let chapter = chapter.as_string()?.read();
			let chapter_object = chapters_object.get(&chapter).as_object()?;
			let groups_object = chapter_object.get("groups").as_object()?;
			for group in groups_object.keys() {
				let group = group.as_string()?.read();
				let scanlator = scanlators_map.get(&group).as_string()?.read();
				let date_updated = match chapter_object.get("release_date").as_object() {
					Ok(obj) => obj.get(&group).as_float().unwrap_or(-1.0),
					Err(_) => -1.0,
				};
				let chapter_id = format!("{chapter},{group}");
				chapters.push(Chapter {
					id: chapter_id,
					url: format!("https://cubari.moe/read/{}/{}/1", id, chapter),
					title: chapter_object.get("title").as_string()?.read(),
					volume: chapter_object
						.get("volume")
						.as_string()?
						.read()
						.parse()
						.unwrap_or(-1.0),
					chapter: chapter.parse().unwrap_or(-1.0),
					date_updated,
					scanlator,
					lang: String::new(),
				})
			}
		}
		chapters.sort_unstable_by_key(|item| (FloatOrd(item.volume), FloatOrd(item.chapter)));
		chapters.reverse();
		chapters
	};
	Ok(chapters)
}

#[no_mangle]
#[export_name = "get_page_list"]
pub extern "C" fn __wasm_get_page_list(chapter_rid: i32) -> i32 {
	let obj = ObjectRef(ValueRef::new(chapter_rid));
	let manga_id = match obj.get("mangaId").as_string() {
		Ok(id) => id.read(),
		Err(_) => return -1,
	};
	let id = match obj.get("id").as_string() {
		Ok(id) => id.read(),
		Err(_) => return -1,
	};
	let resp: Result<Vec<Page>> = get_page_list(manga_id, id);
	match resp {
		Ok(resp) => {
			let mut arr = aidoku::std::ArrayRef::new();
			for item in resp {
				let rid = item.create();
				arr.insert(aidoku::std::ValueRef::new(rid));
			}
			let rid = arr.0 .0;
			core::mem::forget(arr.0);
			rid
		}
		Err(_) => -1,
	}
}

fn get_page_list(manga_id: String, id: String) -> Result<Vec<Page>> {
	fn parse_page_array(pages: ArrayRef) -> Result<Vec<Page>> {
		let mut result = Vec::with_capacity(pages.len());
		for (idx, page) in pages.enumerate() {
			let url = match page.kind() {
				Kind::String => page.as_string()?.read(),
				Kind::Object => page.as_object()?.get("src").as_string()?.read(),
				_ => continue,
			};
			result.push(Page {
				index: idx as i32,
				url: img_url_handler(url),
				base64: String::new(),
				text: String::new(),
			})
		}

		Ok(result)
	}
	if manga_id == "aidoku/guide" {
		Ok(vec![Page {
			index: 1,
			url: format!(
				"https://placehold.jp/42/ffffff/000000/1440x2160.png?css={}&text={}",
				helper::urlencode(String::from(r#"{"text-align":"left", "padding": "100px"}"#)),
				helper::urlencode(String::from(
					r#"Cubari is a proxy for image galleries, and as such there are no mangas available.

To find a gallery for Cubari, search using the search bar in the format of <source>/<slug>, for example, imgur/hYhqG7b.

Alternatively, you can paste the link to:
- an imgur/imgbox gallery
- a **raw** GitHub gist link (git.io links may or may not work)
- a manga details page from any of nhentai, readmanhwa, mangasee/mangalife, mangadex, assortedscans, arc-relight
- even a cubari.moe reader page URL

This source locally tracks and saves any series found, which can be disabled in settings.
"#
				)),
			),
			base64: String::new(),
			text: String::new(),
		}])
	} else {
		let mut split = id.splitn(2, ',');
		let id = split.next().unwrap_or_default();
		let group = split.next().unwrap_or_default();

		cache_api_request(&manga_id)?;
		let json = unsafe { CACHED_JSON.clone().unwrap() };
		let chapters_object = json.get("chapters").as_object()?;

		let chapter_object = chapters_object.get(id).as_object()?;
		let groups_object = chapter_object.get("groups").as_object()?;
		let pages = groups_object.get(group);
		match pages.kind() {
			Kind::Array => {
				let pages = pages.as_array()?;
				parse_page_array(pages)
			}
			Kind::String => {
				let endpoint = pages.as_string()?.read();
				parse_page_array(
					Request::new(&format!("https://cubari.moe{}", endpoint), HttpMethod::Get)
						.json()
						.as_array()?,
				)
			}
			_ => Err(AidokuError {
				reason: AidokuErrorKind::Unimplemented,
			}),
		}
	}
}

#[modify_image_request]
fn modify_image_request(_: Request) {}

#[handle_url]
fn handle_url(url: String) -> Result<DeepLink> {
	// https://cubari.moe/read/imgur/hYhqG7b/
	// ['imgur', 'hYhqG7b']
	let slug = url_to_slug(url.clone());
	let manga = Some(get_manga_details(slug.clone())?);
	let chapter = if url.starts_with("https://cubari.moe/read/") {
		let clone = url.clone();
		let split = clone
			.trim_start_matches("https://cubari.moe/read/")
			.trim_end_matches('/')
			.split('/')
			.collect::<Vec<_>>();
		if split.len() > 2 {
			cache_api_request(&slug)?;
			let json = unsafe { CACHED_JSON.clone().unwrap() };
			let scanlators_map = json.get("groups").as_object()?;
			let chapters_object = json.get("chapters").as_object()?;

			let chapter_object = chapters_object.get(split[2]).as_object()?;
			let chapter_groups = chapter_object.get("groups").as_object()?;
			let scanlator_ids = chapter_groups.keys();
			let scanlator_id = scanlator_ids.get(0).as_string()?.read();

			let date_updated = match chapter_object.get("release_date").as_object() {
				Ok(obj) => obj.get(&scanlator_id).as_float().unwrap_or(-1.0),
				Err(_) => -1.0,
			};
			let scanlator = scanlators_map.get(&scanlator_id).as_string()?.read();
			Some(Chapter {
				id: format!("{},{}", split[2], scanlator_id),
				url,
				title: chapter_object.get("title").as_string()?.read(),
				chapter: split[2].parse().unwrap_or(-1.0),
				volume: chapter_object
					.get("volume")
					.as_string()?
					.read()
					.parse()
					.unwrap_or(-1.0),
				date_updated,
				scanlator,
				lang: String::new(),
			})
		} else {
			None
		}
	} else {
		None
	};
	Ok(DeepLink { manga, chapter })
}

#[handle_notification]
fn handle_notification(notif: String) -> Result<()> {
	match notif.as_str() {
		"deleteHistory" => {
			database::delete_all_manga().ok();
		}
		"rsAddress" => {
			if let Ok(address) = defaults_get("rsAddress").as_string() {
				let address = address.read();
				if !address.is_empty() && address.contains('@') {
					let provider = address.split('@').last().unwrap_or_default();
					let webfinger = format!("https://{provider}/.well-known/webfinger?resource=acct:{address}");
					let json = Request::new(&webfinger, HttpMethod::Get).json().as_object()?;
					let links = json.get("links").as_array()?;
					let props = links.get(0).as_object()?;
					let properties = props.get("properties").as_object()?;
					let oauth_url = properties.get("http://tools.ietf.org/html/rfc6749#section-4.2").as_string()?.read();
					defaults_set(
						"rsOauthUrl",
						StringRef::from(
							format!("{oauth_url}?redirect_uri=aidoku%3A%2F%2Fcubari-auth&scope=cubari%3Arw&client_id=aidoku&response_type=token")
						).0,
					);
				}
			}
		}
		"rsAuthComplete" => {
			if let Ok(callback) = defaults_get("rsToken").as_string() {
				let callback = callback.read();
				let token = callback.split('=').last().unwrap_or_default();
				defaults_set("rsToken", StringRef::from(token).0);
			}
		}
		_ => {},
	};
	Ok(())
}
