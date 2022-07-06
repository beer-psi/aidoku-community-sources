#![no_std]

mod helper;
use aidoku::{
	error::Result,
	prelude::*,
	helpers::substring::Substring,
	std::{json, String, Vec, net::{Request, HttpMethod}, StringRef, html::Node},
	Chapter, Filter, FilterType, Listing, Manga, MangaPageResult, Page, DeepLink, MangaStatus, MangaContentRating, MangaViewer
};

static mut CACHED_MANGA_LIST: Option<Node> = None;
fn cache_manga_list() -> Node {
	unsafe {
		if CACHED_MANGA_LIST.is_none() {
			CACHED_MANGA_LIST = Some(
				Request::new("https://scantrad.net/mangas", HttpMethod::Get)
					.header("Accept-Language", "fr")
					.html()
					.unwrap()
			);
		} 
		CACHED_MANGA_LIST.clone().unwrap()
	}
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
	let html = cache_manga_list();
	let html_str = html.html().read();

	let json = json::parse(
		html_str
			.substring_after("var json = JSON.parse('")
			.unwrap_or_default()
			.substring_before("');")
			.unwrap_or_default()
	)?.as_array()?;
	

	let mut filter_title = String::new();
	let mut demo: Vec<String> = Vec::new();
	let mut notdemo: Vec<String> = Vec::new();
	let mut typ: Vec<String> = Vec::new();
	let mut nottyp: Vec<String> = Vec::new();
	let mut genre: Vec<String> = Vec::new();
	let mut notgenre: Vec<String> = Vec::new();
	let mut status: Vec<String> = Vec::new();
	let mut notstatus: Vec<String> = Vec::new();
	let mut is_recent = -1;
	let mut is_nouveau = -1;
	let mut is_scantrad = -1;
	for filter in filters {
		match filter.kind {
			FilterType::Title => filter_title = filter.value.as_string()?.read().to_lowercase(),
			FilterType::Genre => {
				match filter.value.as_int().unwrap_or(-1) {
					0 => notgenre.push(filter.value.as_string()?.read()),
					1 => genre.push(filter.value.as_string()?.read()),
					_ => continue,
				}
			}
			FilterType::Check => {
				let id = filter.object.get("id")
					.as_string()
					.unwrap_or_else(|_| StringRef::from(""))
					.read();
				let value = filter.value.as_int().unwrap_or(-1);
				if value < 0 { continue }
				match filter.name.as_str() {
					"Seinen" | "Shojo" | "Shonen" | "Josei" => {
						match value {
							0 => notdemo.push(id),
							1 => demo.push(id),
							_ => continue,
						}
					}
					"Manga" | "Manwha" | "Manhua" => {
						match value {
							0 => nottyp.push(id),
							1 => typ.push(id),
							_ => continue,
						}
					}
					"En cours" | "Terminé" | "En pause ou Arrêté" => {
						match value {
							0 => notstatus.push(id),
							1 => status.push(id),
							_ => continue,
						}
					}
					"Récemment mis à jour" => is_recent = value,
					"Nouveaux" => is_nouveau = value,
					"Scantrad France" => is_scantrad = value,
					_ => continue,
				}
			},
			_ => continue,
		}
	}

	let mut slugs = Vec::new();
	for item in json {
		let obj = item.as_object()?;
		let demographie = obj.get("demographie").as_string()?.read();
		if !demo.is_empty() && !demo.contains(&demographie) { continue }
		if !notdemo.is_empty() && notdemo.contains(&demographie) { continue }

		let genres = obj.get("genres")
			.as_array()?
			.map(|genre| genre.as_string().unwrap_or_else(|_| StringRef::from("")).read())
			.collect::<Vec<_>>();
		if !genre.is_empty() && !genre.iter().any(|g| genres.contains(g)) { continue }
		if !notgenre.is_empty() && notgenre.iter().any(|g| genres.contains(g)) { continue }

		let statut = obj.get("status").as_string()?.read();
		if !status.is_empty() && !status.contains(&statut) { continue }
		if !notstatus.is_empty() && notstatus.contains(&statut) { continue }

		let recent = obj.get("isRecent").as_bool()?;
		if is_recent >= 0 && recent != (is_recent == 1) { continue }

		let nouveau = obj.get("isNouveau").as_bool()?;
		if is_nouveau >= 0 && nouveau != (is_nouveau == 1) { continue }

		let scantrad = obj.get("isScantrad").as_bool()?;
		if is_scantrad >= 0 && scantrad != (is_scantrad == 1) { continue }

		// Congratulations, you've passed all the filters
		let slug = obj.get("slug").as_string()?.read();
		slugs.push(slug);
	}

	let mut manga = Vec::with_capacity(slugs.len());
	let elems = html.select("div.manga");
	for elem in elems.array() {
		let node = elem.as_node()?;

		let slug = node.attr("data-slug").read();
		if !slugs.contains(&slug) { continue }

		let title = node.select("a.mri-top").text().read();
		if !filter_title.is_empty() && !title.to_lowercase().contains(&filter_title) { continue }

		manga.push(Manga {
			url: format!("https://scantrad.net/{slug}"),
			id: slug,
			cover: node.select("div.manga_img img").attr("abs:data-src").read(),
			title,
			author: String::new(),
			artist: String::new(),
			description: String::new(),
			categories: Vec::new(),
			status: MangaStatus::Unknown,
			nsfw: MangaContentRating::Safe,
			viewer: MangaViewer::Rtl,
		})
	}
	Ok(MangaPageResult { manga, has_more: false })
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
	todo!()
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
	todo!()
}

#[get_page_list]
fn get_page_list(manga_id: String, id: String) -> Result<Vec<Page>> {
	todo!()
}

#[modify_image_request]
fn modify_image_request(request: Request) {
	request.header("Referer", "https://scantrad.net/");
}

#[handle_url]
fn handle_url(url: String) -> Result<DeepLink> {
	todo!()
}
