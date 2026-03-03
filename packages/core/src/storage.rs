use crate::models::ParsedPlaylist;

pub trait PlaylistStorage {
    fn save_playlist(&self, playlist: &ParsedPlaylist) -> Result<(), String>;
    fn load_playlist(&self) -> Result<Option<ParsedPlaylist>, String>;
    fn save_favorites(&self, favorites: &[String]) -> Result<(), String>;
    fn load_favorites(&self) -> Result<Vec<String>, String>;
    fn save_recents(&self, recents: &[String]) -> Result<(), String>;
    fn load_recents(&self) -> Result<Vec<String>, String>;
}
