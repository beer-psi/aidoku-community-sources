use aidoku::{
	prelude::format,
	std::{String, Vec},
	Manga, MangaContentRating, MangaStatus, MangaViewer,
};
use alloc::vec;

pub fn urlencode(string: String) -> String {
	let mut result: Vec<u8> = Vec::with_capacity(string.len() * 3);
	let hex = "0123456789abcdef".as_bytes();
	let bytes = string.as_bytes();

	for byte in bytes {
		let curr = *byte;
		if curr.is_ascii_alphanumeric() {
			result.push(curr);
		} else {
			result.push(b'%');
			result.push(hex[curr as usize >> 4]);
			result.push(hex[curr as usize & 15]);
		}
	}

	String::from_utf8(result).unwrap_or_default()
}

pub fn cubari_guide() -> Manga {
	Manga {
		id: String::from("aidoku/guide"),
		cover: String::from("https://fakeimg.pl/550x780/ffffff/6e7b91/?font=museo&text=Guide"),
		title: String::from("Cubari Guide"),
		author: String::new(),
		artist: String::new(),
		description: String::new(),
		url: String::new(),
		categories: Vec::new(),
		status: MangaStatus::Unknown,
		nsfw: MangaContentRating::Safe,
		viewer: MangaViewer::Rtl,
	}
}

pub fn base64_encode<T: AsRef<[u8]>>(str: T) -> String {
    let str = str.as_ref();
    let mut buf = vec![0; str.len() * 4 / 3 + 4];
    let bytes_written = base64::encode_config_slice(str, base64::URL_SAFE_NO_PAD, &mut buf);
    buf.resize(bytes_written, 0);
    String::from_utf8(buf).unwrap_or_default()
}

pub fn img_url_handler(url: String) -> String {
	if url.contains(".imgbox.com") {
		url.replace("thumbs", "images")
	} else {
		url
	}
}

pub fn url_to_slug<T: AsRef<str>>(url: T) -> String {
    let url = url.as_ref();
    let slash_count = url.matches('/').count();
	let query = url
		.trim_start_matches("http")
		.trim_start_matches('s')
		.trim_start_matches("://")
		.trim_end_matches('/');
	if query.contains("imgur") && query.replace("/a/", "/gallery/").contains("/gallery/") {
		format!(
			"imgur/{}",
			query
				.replace("/a/", "/gallery/")
				.trim_start_matches("m.")
				.trim_start_matches("imgur")
				.trim_start_matches(".com")
				.trim_start_matches(".io")
				.trim_start_matches("/gallery/"),
		)
	} else if query.contains("git.io") {
		format!("gist/{}", query.trim_start_matches("git.io/"))
	} else if query.contains("gist.githubusercontent.com/")
		|| query.contains("gist.github.com/") && query.contains("raw")
	{
		let temp = format!(
			"gist/{}",
			query
				.trim_start_matches("gist.githubusercontent.com/")
				.trim_start_matches("gist.github.com/"),
		);
		format!("gist/{}", base64_encode(temp))
	} else if query.contains("imgbox.com/g/")
		|| query.contains("readmanhwa.com")
		|| query.contains("nhentai.net/g/")
	{
        // Generic parser for anything whose slug is the last part of the URL.
		let url = query.split('/').next().unwrap_or_default();

		let source = url.split('.').next().unwrap_or_default();
		let slug = query.split('/').last().unwrap_or_default();

		format!("{source}/{slug}")
	} else if query.contains("mangasee123.com/manga") || query.contains("manga4life.com/manga") {
		format!(
			"mangasee/{}",
			query
				.trim_start_matches("manga")
				.trim_start_matches("see123")
				.trim_start_matches("4life")
				.trim_start_matches(".com/manga/")
		)
    } else if query.contains("mangadex.org/title") {
        let split = query.split('/').collect::<Vec<_>>();
        format!("mangadex/{}", split[2])
    } else if query.contains("mangakatana") {
        // Generic parser for anything that has the entire URL base64-encoded as a slug.
        let domain = query.split('/').next().unwrap_or_default();
		let source = domain.split('.').next().unwrap_or_default();

		format!("{source}/{}", base64_encode(url))
	} else if (query.contains("assortedscans.com") || query.contains("arc-relight.com")) && slash_count >= 4 {
        // MangAdventure CMS
        let split = url.split('/').collect::<Vec<_>>();
        let slug = format!("{}/{}/{}", split[0].trim_end_matches(':'), split[2], split[4]);

        format!("mangadventure/{}", base64_encode(slug))
    } else if query.contains("cubari.moe/read") && slash_count >= 3 {
		let split = query
			.trim_start_matches("cubari.moe/read/")
			.trim_end_matches('/')
			.split('/')
			.collect::<Vec<_>>();
		format!("{}/{}", split[0], split[1])
	} else {
		String::from(query)
	}
}
