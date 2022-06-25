#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::vec;

#[cfg(test)]
use alloc::{format, string::String, vec::Vec};

#[cfg(not(test))]
use aidoku::{
	prelude::format,
	std::{String, Vec},
};

fn base64_encode<T: AsRef<[u8]>>(str: T) -> String {
	let str = str.as_ref();
	let mut buf = vec![0; str.len() * 4 / 3 + 4];
	let bytes_written = base64::encode_config_slice(str, base64::URL_SAFE_NO_PAD, &mut buf);
	buf.resize(bytes_written, 0);
	String::from_utf8(buf).unwrap_or_default()
}

/// Convert a compatible URL to a Cubari slug.
///
/// Currently works with:
/// - Imgur, Reddit, imgbox gallery URLs
/// - GitHub Gists raw URL
/// - git.io URLs
/// - ReadManhwa, nhentai, mangasee, mangalife, mangadex, mangakatana,
/// sources using the MangAdventure CMS
/// - cubari.moe reader page URL
///
/// # Returns
/// Returns the original URL if not parsable.
pub fn url_to_slug<T: AsRef<str>>(url: T) -> String {
	let url = url.as_ref();
	let slash_count = url.matches('/').count();
	let query = url
		.trim_start_matches("http")
		.trim_start_matches('s')
		.trim_start_matches("://")
		.trim_end_matches('/');
	if query.contains("imgur") && query.replace("/a/", "/gallery/").contains("/gallery/")
		|| query.contains("reddit.com/gallery")
		|| query.contains("imgbox.com/g/")
		|| query.contains("readmanhwa.com")
		|| query.contains("nhentai.net/g/")
	{
		// Common parser for any URL with this structure
		// https://{source}.{tld}/path/{slug}
		// where slug is always the last part of the URL.
		let domain = query.split('/').next().unwrap_or_default();
		let source = domain.split('.').nth_back(1).unwrap_or_default();
		let slug = query.split('/').last().unwrap_or_default();
		format!("{source}/{slug}")
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
	} else if (query.contains("assortedscans.com") || query.contains("arc-relight.com"))
		&& slash_count >= 4
	{
		// MangAdventure CMS
		let split = url.split('/').collect::<Vec<_>>();
		let slug = format!(
			"{}/{}/{}",
			split[0].trim_end_matches(':'),
			split[2],
			split[4]
		);

		format!("mangadventure/{}", base64_encode(slug))
	} else if query.contains("cubari.moe/read") && slash_count >= 3 {
		let split = query
			.trim_start_matches("cubari.moe/read/")
			.trim_end_matches('/')
			.split('/')
			.collect::<Vec<_>>();
		format!("{}/{}", split[0], split[1])
	} else {
		String::from(url)
	}
}

#[cfg(test)]
mod tests {
	use crate::url_to_slug;

	macro_rules! generate_test {
		($url:expr, $matches:expr) => {
			assert_eq!(url_to_slug($url), $matches);
			assert_eq!(url_to_slug("http://".to_owned() + $url), $matches);
			assert_eq!(url_to_slug("https://".to_owned() + $url), $matches);
			assert_eq!(url_to_slug("http://".to_owned() + $url + "/"), $matches);
			assert_eq!(url_to_slug("https://".to_owned() + $url + "/"), $matches);
		};
	}

	#[test]
	fn test_source_slug_parser() {
		generate_test!("reddit.com/gallery/vjry2h", "reddit/vjry2h");
		generate_test!("new.reddit.com/gallery/vjry2h", "reddit/vjry2h");
		generate_test!("www.reddit.com/gallery/vjry2h", "reddit/vjry2h");

		generate_test!("imgur.com/gallery/hYhqG7b", "imgur/hYhqG7b");
		generate_test!("imgur.io/gallery/hYhqG7b", "imgur/hYhqG7b");
		generate_test!("m.imgur.com/gallery/hYhqG7b", "imgur/hYhqG7b");

		generate_test!("imgur.com/a/hYhqG7b", "imgur/hYhqG7b");
		generate_test!("imgur.io/a/hYhqG7b", "imgur/hYhqG7b");
		generate_test!("m.imgur.com/a/hYhqG7b", "imgur/hYhqG7b");

		generate_test!("nhentai.net/g/177013", "nhentai/177013");
		generate_test!("imgbox.com/g/YMWC88hgjM", "imgbox/YMWC88hgjM");
		generate_test!(
			"readmanhwa.com/en/webtoon/keep-it-a-secret-from-your-mother",
			"readmanhwa/keep-it-a-secret-from-your-mother"
		);
	}

	#[test]
	fn test_git_io_parser() {
		generate_test!("git.io/JO7JN", "gist/JO7JN");
	}

	#[test]
	fn test_github_gist_parser() {
		generate_test!(
			"gist.github.com/NightA/99cf38923b5b80d62b83158c141a1226/raw/9eed3fad738ed66943804cbb27df5404d5586b07/Yofukashi.JSON", 
			"gist/Z2lzdC9OaWdodEEvOTljZjM4OTIzYjViODBkNjJiODMxNThjMTQxYTEyMjYvcmF3LzllZWQzZmFkNzM4ZWQ2Njk0MzgwNGNiYjI3ZGY1NDA0ZDU1ODZiMDcvWW9mdWthc2hpLkpTT04"
		);
		generate_test!(
			"gist.githubusercontent.com/NightA/99cf38923b5b80d62b83158c141a1226/raw/9eed3fad738ed66943804cbb27df5404d5586b07/Yofukashi.JSON", 
			"gist/Z2lzdC9OaWdodEEvOTljZjM4OTIzYjViODBkNjJiODMxNThjMTQxYTEyMjYvcmF3LzllZWQzZmFkNzM4ZWQ2Njk0MzgwNGNiYjI3ZGY1NDA0ZDU1ODZiMDcvWW9mdWthc2hpLkpTT04"
		);
	}

	#[test]
	fn test_nepnep_parser() {
		generate_test!("mangasee123.com/manga/Anima", "mangasee/Anima");
		generate_test!("manga4life.com/manga/Anima", "mangasee/Anima");
	}

	#[test]
	fn test_mangadex_parser() {
		generate_test!(
			"mangadex.org/title/801513ba-a712-498c-8f57-cae55b38cc92/berserk",
			"mangadex/801513ba-a712-498c-8f57-cae55b38cc92"
		);
		generate_test!(
			"mangadex.org/title/801513ba-a712-498c-8f57-cae55b38cc92",
			"mangadex/801513ba-a712-498c-8f57-cae55b38cc92"
		);
	}

	#[test]
	fn test_mangakatana_parser() {
		assert_eq!(
			url_to_slug("https://mangakatana.com/manga/the-human-hating-demon-lord-has-no-mercy-for-little-girls.26241"),
			"mangakatana/aHR0cHM6Ly9tYW5nYWthdGFuYS5jb20vbWFuZ2EvdGhlLWh1bWFuLWhhdGluZy1kZW1vbi1sb3JkLWhhcy1uby1tZXJjeS1mb3ItbGl0dGxlLWdpcmxzLjI2MjQx",
		);
	}

	#[test]
	fn test_mangadventure_parser() {
		assert_eq!(
			url_to_slug("https://assortedscans.com/reader/maou-to-yuri-volume-version/"),
			"mangadventure/aHR0cHMvYXNzb3J0ZWRzY2Fucy5jb20vbWFvdS10by15dXJpLXZvbHVtZS12ZXJzaW9u"
		);
		assert_eq!(
			url_to_slug("https://arc-relight.com/reader/childrens-collapse/"),
			"mangadventure/aHR0cHMvYXJjLXJlbGlnaHQuY29tL2NoaWxkcmVucy1jb2xsYXBzZQ",
		)
	}

	#[test]
	fn test_cubari_parser() {
		generate_test!(
			"cubari.moe/read/gist/Z2lzdC9OaWdodEEvOTljZjM4OTIzYjViODBkNjJiODMxNThjMTQxYTEyMjYvcmF3LzllZWQzZmFkNzM4ZWQ2Njk0MzgwNGNiYjI3ZGY1NDA0ZDU1ODZiMDcvWW9mdWthc2hpLkpTT04",
			"gist/Z2lzdC9OaWdodEEvOTljZjM4OTIzYjViODBkNjJiODMxNThjMTQxYTEyMjYvcmF3LzllZWQzZmFkNzM4ZWQ2Njk0MzgwNGNiYjI3ZGY1NDA0ZDU1ODZiMDcvWW9mdWthc2hpLkpTT04"
		);
		generate_test!("cubari.moe/read/nhentai/408179", "nhentai/408179");
		generate_test!("cubari.moe/read/nhentai/408179/1", "nhentai/408179");
	}

	#[test]
	fn test_unknown_parser() {
		assert_eq!(
			url_to_slug("https://www.google.com"),
			"https://www.google.com"
		);
		assert_eq!(url_to_slug("nhentai/177013"), "nhentai/177013");
	}
}
