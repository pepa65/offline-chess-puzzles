use crate::schema::favs;
use diesel::prelude::*;

#[derive(Insertable)]
#[diesel(table_name = favs)]
pub struct NewFavorite<'a> {
	pub puzzle_id: &'a str,
	pub fen: &'a str,
	pub moves: &'a str,
	pub rating: i32,
	pub rd: i32,
	pub popularity: i32,
	pub nb_plays: i32,
	pub themes: &'a str,
	pub game_url: &'a str,
	pub opening_tags: &'a str,
}
