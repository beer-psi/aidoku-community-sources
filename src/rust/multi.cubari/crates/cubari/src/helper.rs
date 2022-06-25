use aidoku::{
	std::{String, Vec},
	Manga, MangaContentRating, MangaStatus, MangaViewer,
};
pub use cubari_url_parser::*;

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

pub fn img_url_handler(url: String) -> String {
	if url.contains(".imgbox.com") {
		url.replace("thumbs", "images")
	} else {
		url
	}
}
