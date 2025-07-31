use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dirs_next::home_dir;
use std::env;

use crate::{
	config::Puzzle,
	models::NewFavorite,
	openings::{Openings, Variation},
	schema::favs,
	schema::favs::dsl::*,
	search_tab::{OpeningSide, TacticalThemes},
};

pub fn establish_connection() -> SqliteConnection {
	let home = home_dir().unwrap().display().to_string();
	let ocp_home = env::var("OCP_HOME").unwrap_or(home + "/.offline-chess-puzzles");
	let ocp_favorites = ocp_home + "/favorites.db";
	let mut conn = SqliteConnection::establish(&ocp_favorites).unwrap_or_else(|_| panic!("Error connecting to {}", ocp_favorites));
	let init = concat!(
		"CREATE TABLE IF NOT EXISTS favs (",
		"puzzle_id TEXT NOT NULL PRIMARY KEY,",
		"fen TEXT NOT NULL,",
		"moves TEXT NOT NULL,",
		"rating INTEGER NOT NULL,",
		"rd INTEGER NOT NULL,",
		"popularity INTEGER NOT NULL,",
		"nb_plays INTEGER NOT NULL,",
		"themes TEXT NOT NULL,",
		"game_url TEXT NOT NULL NOT NULL,",
		"opening_tags TEXT NOT NULL",
		");",
		"PRAGMA foreign_keys=OFF;",
		"COMMIT;"
	);
	if let Err(err) = diesel::sql_query(init.to_string()).execute(&mut conn) {
		panic!("{:?}: can't initialize {}", err, ocp_favorites);
	}
	conn
}

pub fn get_favorites(
	min_rating: i32, max_rating: i32, min_popularity: i32, theme: TacticalThemes, opening: Openings, variation: Variation, op_side: Option<OpeningSide>,
	result_limit: usize,
) -> Option<Vec<Puzzle>> {
	let mut conn = establish_connection();
	let results;
	let theme_filter = String::from("%") + theme.get_tag_name() + "%";
	let limit = result_limit as i64;
	if opening == Openings::Any {
		results = favs
			.filter(rating.between(min_rating, max_rating))
			.filter(popularity.ge(min_popularity))
			.filter(themes.like(theme_filter))
			.limit(limit)
			.load::<Puzzle>(&mut conn);
	} else {
		let opening_tag: &str = if variation.name != Variation::ANY_STR { &variation.name } else { opening.get_field_name() };
		let opening_filter = opening_tags.like(String::from("%") + opening_tag + "%");
		let side = match op_side {
			None => OpeningSide::Any,
			Some(x) => x,
		};
		if side == OpeningSide::White {
			results = favs
				.filter(rating.between(min_rating, max_rating))
				.filter(popularity.ge(min_popularity))
				.filter(themes.like(theme_filter))
				.filter(opening_filter)
				.filter(game_url.like("%black%"))
				.limit(limit)
				.load::<Puzzle>(&mut conn);
		} else if side == OpeningSide::Black {
			results = favs
				.filter(rating.between(min_rating, max_rating))
				.filter(popularity.ge(min_popularity))
				.filter(themes.like(theme_filter))
				.filter(opening_filter)
				.filter(game_url.not_like("%black%"))
				.limit(limit)
				.load::<Puzzle>(&mut conn);
		} else {
			results = favs
				.filter(rating.between(min_rating, max_rating))
				.filter(popularity.ge(min_popularity))
				.filter(themes.like(theme_filter))
				.filter(opening_filter)
				.limit(limit)
				.load::<Puzzle>(&mut conn);
		}
	}
	results.ok()
}

pub fn is_favorite(id: &str) -> bool {
	let mut conn = establish_connection();
	let results = favs.filter(puzzle_id.eq(id)).first::<Puzzle>(&mut conn);

	results.is_ok()
}

pub fn toggle_favorite(puzzle: Puzzle) {
	let mut conn = establish_connection();
	let is_fav = favs.filter(puzzle_id.eq(&puzzle.puzzle_id)).first::<Puzzle>(&mut conn).is_ok();

	if is_fav {
		diesel::delete(favs::table).filter(puzzle_id.eq(&puzzle.puzzle_id)).execute(&mut conn).expect("Error removing favorite");
	} else {
		let new_fav = NewFavorite {
			puzzle_id: &puzzle.puzzle_id,
			fen: &puzzle.fen,
			moves: &puzzle.moves,
			rating: puzzle.rating,
			rd: puzzle.rating_deviation,
			popularity: puzzle.popularity,
			nb_plays: puzzle.nb_plays,
			themes: &puzzle.themes,
			game_url: &puzzle.game_url,
			opening_tags: &puzzle.opening,
		};

		diesel::insert_into(favs::table).values(&new_fav).execute(&mut conn).expect("Error saving new favorite");
	}
}
