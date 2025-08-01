use chess::{Board, ChessMove, Piece, Square};
use diesel::prelude::*;
use iced::Font;
use once_cell::sync::Lazy;
use std::str::FromStr;

use crate::{
	lang,
	openings::{Openings, Variation},
	search_tab::OpeningSide,
	search_tab::TacticalThemes,
	styles,
};

pub static SETTINGS: Lazy<OfflinePuzzlesConfig> = Lazy::new(load_config);

pub const CHESS_ALPHA_BYTES: &[u8] = include_bytes!("../include/Alpha.ttf");
pub const CHESS_ALPHA: Font = iced::Font::with_name("Chess Alpha");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
	Puzzle,
	Analysis,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OfflinePuzzlesConfig {
	pub engine_path: Option<String>,
	pub engine_limit: String,
	pub window_width: f32,
	pub window_height: f32,
	pub maximized: bool,
	pub puzzle_db_location: String,
	pub piece_theme: styles::PieceTheme,
	pub search_results_limit: usize,
	pub play_sound: bool,
	pub auto_load_next: bool,
	pub flip_board: bool,
	pub show_coordinates: bool,
	pub board_theme: styles::BoardTheme,
	pub lang: lang::Language,
	pub export_pgs: i32,
	pub last_min_rating: i32,
	pub last_max_rating: i32,
	pub last_min_popularity: i32,
	pub last_theme: TacticalThemes,
	pub last_opening: Openings,
	pub last_variation: Variation,
	pub last_opening_side: Option<OpeningSide>,
}

impl ::std::default::Default for OfflinePuzzlesConfig {
	fn default() -> Self {
		Self {
			engine_path: Some(String::from("/usr/games/stockfish")),
			engine_limit: String::from("depth 40"),
			window_width: 1297.,
			window_height: 1025.,
			maximized: false,
			puzzle_db_location: String::from("lichess_db_puzzle.csv"),
			piece_theme: styles::PieceTheme::Alpha,
			search_results_limit: 200000,
			play_sound: true,
			auto_load_next: false,
			flip_board: false,
			show_coordinates: false,
			board_theme: styles::BoardTheme::BrownDark,
			lang: lang::Language::English,
			export_pgs: 50,
			last_min_rating: 1250,
			last_max_rating: 1750,
			last_min_popularity: 0,
			last_theme: TacticalThemes::All,
			last_opening: Openings::Any,
			last_variation: Variation::ANY,
			last_opening_side: Some(OpeningSide::Any),
		}
	}
}

pub fn load_config() -> OfflinePuzzlesConfig {
	let config;
	let file = std::fs::File::open("settings.json");
	match file {
		Ok(file) => {
			let reader = std::io::BufReader::new(file);
			let config_json = serde_json::from_reader(reader);
			match config_json {
				Ok(cfg) => config = cfg,
				Err(_) => config = OfflinePuzzlesConfig::default(),
			}
		}
		Err(_) => config = OfflinePuzzlesConfig::default(),
	}
	config
}

fn piece_localized(lang: &lang::Language, piece: &str) -> String {
	match piece {
		"B" => lang::tr(lang, "bishop"),
		"N" => lang::tr(lang, "knight"),
		"R" => lang::tr(lang, "rook"),
		"Q" => lang::tr(lang, "queen"),
		_ => lang::tr(lang, "king"),
	}
}

pub fn coord_to_san(board: &Board, coords: String, lang: &lang::Language) -> Option<String> {
	let (promotion_piece, coords) = if coords.len() > 4 { (coords[4..5].to_uppercase(), String::from(&coords[0..4])) } else { (String::from(""), coords) };

	let mut san = None;
	let orig_square = Square::from_str(&coords[0..2]).unwrap();
	let dest_square = Square::from_str(&coords[2..4]).unwrap();
	let piece = board.piece_on(orig_square);
	if let Some(piece) = piece {
		if piece == Piece::King && (coords == "e1g1" || coords == "e8g8") {
			san = Some(String::from("0-0"));
		} else if piece == Piece::King && (coords == "e1c1" || coords == "e8c8") {
			san = Some(String::from("0-0-0"));
		} else {
			let mut san_str = String::new();
			let mut san_localized = String::new();
			let is_en_passant = piece == Piece::Pawn && board.piece_on(dest_square).is_none() && dest_square.get_file() != orig_square.get_file();
			let is_capture = board.piece_on(dest_square).is_some();
			match piece {
				Piece::Pawn => {
					// We're also creating the san in English notation because
					// we use the chess crate to check if it's valid (in order
					// to know if it needs disambiguation or not)
					san_str.push_str(&coords[0..1]);
					san_localized.push_str(&coords[0..1]);
				}
				Piece::Bishop => {
					san_str.push('B');
					san_localized.push_str(&lang::tr(lang, "bishop"));
				}
				Piece::Knight => {
					san_str.push('N');
					san_localized.push_str(&lang::tr(lang, "knight"));
				}
				Piece::Rook => {
					san_str.push('R');
					san_localized.push_str(&lang::tr(lang, "rook"));
				}
				Piece::Queen => {
					san_str.push('Q');
					san_localized.push_str(&lang::tr(lang, "queen"));
				}
				Piece::King => {
					san_str.push('K');
					san_localized.push_str(&lang::tr(lang, "king"));
				}
			}
			// Checking fist the cases of capture
			if is_en_passant {
				san_localized.push_str(&(String::from("x") + &coords[2..4] + " e.p."));
			} else if is_capture {
				let capture = if piece == Piece::Pawn {
					// Note: For the from_san() function we really can't use the equal sign: https://github.com/jordanbray/chess/issues/80
					san_str.clone() + "x" + &coords[2..] + &promotion_piece
				} else {
					san_str.clone() + "x" + &coords[2..]
				};
				let try_move = ChessMove::from_san(board, &capture);
				if try_move.is_ok() {
					if promotion_piece.is_empty() {
						san_str.push_str(&(String::from("x") + &coords[2..]));
						san_localized.push_str(&(String::from("x") + &coords[2..]));
					} else {
						san_str.push_str(&(String::from("x") + &coords[2..] + &promotion_piece));
						san_localized.push_str(&(String::from("x") + &coords[2..] + "=" + &piece_localized(lang, &promotion_piece)));
					}
				} else {
					//the simple notation can only fail because of ambiguity, so we try to specify
					//either the file or the rank
					let capture_with_file = san_str.clone() + &coords[0..1] + "x" + &coords[2..];
					let try_move_file = ChessMove::from_san(board, &capture_with_file);
					if try_move_file.is_ok() {
						san_localized.push_str(&(String::from(&coords[0..1]) + "x" + &coords[2..]));
					} else {
						san_localized.push_str(&(String::from(&coords[1..2]) + "x" + &coords[2..]));
					}
				}
			// And now the regular moves
			} else if piece == Piece::Pawn {
				if promotion_piece.is_empty() {
					san_localized = String::from(&coords[2..]);
				} else {
					san_str = san_str + &coords[2..] + &promotion_piece;
					san_localized = String::from(&coords[2..]) + "=" + &piece_localized(lang, &promotion_piece);
				}
			} else {
				let move_with_regular_notation = san_str.clone() + &coords[2..];
				let move_to_try = ChessMove::from_san(board, &move_with_regular_notation);
				if move_to_try.is_ok() {
					san_str.push_str(&coords[2..]);
					san_localized.push_str(&coords[2..]);
				} else {
					//the simple notation can only fail because of ambiguity, so we try to specify
					//either the file or the rank
					let move_notation_with_file = san_str.clone() + &coords[0..1] + &coords[2..];
					let try_move_file = ChessMove::from_san(board, &move_notation_with_file);
					if try_move_file.is_ok() {
						san_localized.push_str(&(String::from(&coords[0..1]) + &coords[2..]));
					} else {
						san_localized.push_str(&(String::from(&coords[1..2]) + &coords[2..]));
					}
				}
			}
			let chess_move = ChessMove::from_san(board, &san_str);
			// Note: It can indeed return Err for a moment when using the engine (and quickly taking
			// back moves), I guess for a sec the engine & board may get desynced, so we can't just unwrap it.
			if let Ok(chess_move) = chess_move {
				let current_board = board.make_move_new(chess_move);
				if current_board.status() == chess::BoardStatus::Checkmate {
					san_localized.push('#');
				} else if current_board.checkers().popcnt() != 0 {
					san_localized.push('+');
				}
			}
			san = Some(san_localized);
		}
	}
	san
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Queryable)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Puzzle {
	#[serde(rename = "PuzzleId")]
	pub puzzle_id: String,
	#[serde(rename = "FEN")]
	pub fen: String,
	#[serde(rename = "Moves")]
	pub moves: String,
	#[serde(rename = "Rating")]
	pub rating: i32,
	#[serde(rename = "RatingDeviation")]
	pub rating_deviation: i32,
	#[serde(rename = "Popularity")]
	pub popularity: i32,
	#[serde(rename = "NbPlays")]
	pub nb_plays: i32,
	#[serde(rename = "Themes")]
	pub themes: String,
	#[serde(rename = "GameUrl")]
	pub game_url: String,
	#[serde(rename = "OpeningTags")]
	#[serde(default)]
	pub opening: String,
}
