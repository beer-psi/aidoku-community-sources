use aidoku::{std::Vec, MangaStatus};

pub fn incl_excl_check<T: core::cmp::PartialEq>(item: &T, incl: &Vec<T>, excl: &Vec<T>) -> bool {
	if !incl.is_empty() && !incl.contains(item) {
		return false;
	}
	if !excl.is_empty() && excl.contains(item) {
		return false;
	}
	true
}

pub fn status_map<T: AsRef<str>>(status: T) -> MangaStatus {
	match status.as_ref() {
		"Ongoing" => MangaStatus::Ongoing,
		"Dropped" => MangaStatus::Cancelled,
		"Completed" => MangaStatus::Completed,
		"Hiatus" => MangaStatus::Hiatus,
		_ => MangaStatus::Unknown,
	}
}
