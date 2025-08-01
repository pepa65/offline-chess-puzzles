#![windows_subsystem = "windows"]

use chess::{ALL_SQUARES, Board, BoardStatus, ChessMove, Color, File, Game, Piece, Rank, Square};
use dirs_next::home_dir;
use eval::{Engine, EngineStatus};
use iced::{Alignment, Element, Length, Rectangle, Size, Subscription, Task, Theme, alignment};
use iced::{
	advanced::widget::Id as GenericId,
	color,
	event::{self, Event},
	widget::{
		Button, Column, Container, Radio, Row, Svg, Text, button, center, container, container::Id, responsive, row, svg::Handle, text, text::LineHeight,
		text_input,
	},
	window::{self, Screenshot},
};
use iced_aw::{TabLabel, Tabs};
use image::RgbaImage;
use include_dir::{Dir, include_dir};
use rand::seq::SliceRandom;
use rfd::AsyncFileDialog;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use std::{borrow::Cow, collections::HashMap, env, fs, io::Cursor, path::Path, str::FromStr};
use styles::PieceTheme;
use tokio::sync::mpsc::{self, Sender};

mod config;
pub mod download_db;
use download_db::download_lichess_db;
mod search_tab;
use search_tab::{SearchMesssage, SearchTab};
mod styles;

mod settings;
use settings::{SettingsMessage, SettingsTab};

mod puzzles;
use puzzles::{GameStatus, PuzzleMessage, PuzzleTab};

mod eval;
mod export;
mod lang;
mod openings;

mod db;
pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;
extern crate serde;
#[macro_use]
extern crate serde_derive;

const HEADER_SIZE: u16 = 32;
const TAB_PADDING: u16 = 16;
const LICHESS_DB_URL: &str = "https://database.lichess.org/lichess_db_puzzle.csv.zst";
const RED: iced::Color = color!(0xff0000);
const GREEN: iced::Color = color!(0x00ff00);
const YELLOW: iced::Color = color!(0xffff00);
const ONE_PIECE: &[u8] = include_bytes!("../include/1piece.ogg");
const TWO_PIECES: &[u8] = include_bytes!("../include/2pieces.ogg");
pub const PIECES: Dir = include_dir!("include/pieces");

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionGUI {
	row: i32,
	col: i32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TabId {
	Search,
	Settings,
	CurrentPuzzle,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
enum PieceWithColor {
	WhitePawn,
	WhiteRook,
	WhiteKnight,
	WhiteBishop,
	WhiteQueen,
	WhiteKing,
	BlackPawn,
	BlackRook,
	BlackKnight,
	BlackBishop,
	BlackQueen,
	BlackKing,
}

impl PieceWithColor {
	fn index(&self) -> usize {
		*self as usize
	}
}

#[derive(Debug, Clone)]
pub enum Message {
	WindowInitialized(Option<iced::window::Id>),
	SelectSquare(Square),
	Search(SearchMesssage),
	Settings(SettingsMessage),
	PuzzleInfo(PuzzleMessage),
	SelectMode(config::GameMode),
	TabSelected(TabId),
	ShowHint,
	ShowNextPuzzle,
	ShowPreviousPuzzle,
	GoBackMove,
	RedoPuzzle,
	DropPiece(Square, iced::Point, iced::Rectangle),
	HandleDropZones(Square, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),
	ScreenshotCreated(Screenshot),
	SaveScreenshot(Option<(Screenshot, String)>),
	ExportPDF(Option<String>),
	LoadPuzzle(Option<Vec<config::Puzzle>>),
	ChangeSettings(Option<config::OfflinePuzzlesConfig>),
	EventOccurred(iced::Event),
	StartEngine,
	EngineStopped(bool),
	UpdateEval((Option<String>, Option<String>)),
	EngineReady(mpsc::Sender<String>),
	EngineFileChosen(Option<String>),
	FavoritePuzzle,
	MinimizeUI,
	SaveMaximizedStatusAndExit(bool),
	StartDBDownload,
	DBDownloadFinished,
	DownloadProgress(String),
	PuzzleInputIndexChange(String),
	JumpToPuzzle,
}

struct SoundPlayback {
	#[allow(dead_code)]
	stream: OutputStream,
}

impl SoundPlayback {
	pub fn init_sound() -> Option<Self> {
		let mut sound_playback = None;
		if let Ok(mut stream) = OutputStreamBuilder::open_default_stream() {
			stream.log_on_drop(false);
			sound_playback = Some(SoundPlayback { stream });
		}
		sound_playback
	}
	pub fn play_one(&self) {
		let cursor = Cursor::new(ONE_PIECE);
		let one_piece = Decoder::new(cursor).unwrap();
		let sink = Sink::connect_new(self.stream.mixer());
		sink.append(one_piece);
	}
	pub fn play_two(&self) {
		let cursor = Cursor::new(TWO_PIECES);
		let two_pieces = Decoder::new(cursor).unwrap();
		let sink = Sink::connect_new(self.stream.mixer());
		sink.append(two_pieces);
	}
}

fn get_image_handles(theme: &PieceTheme) -> Vec<Handle> {
	let mut handles = Vec::<Handle>::with_capacity(12); // All different pieces & colors
	let theme_str = &theme.to_string();
	let svgs = ["wP.svg", "wR.svg", "wN.svg", "wB.svg", "wQ.svg", "wK.svg", "bP.svg", "bR.svg", "bN.svg", "bB.svg", "bQ.svg", "bK.svg"];
	for (i, svg) in svgs.iter().enumerate() {
		let f = PIECES.get_file(theme_str.to_owned() + "/" + svg).unwrap();
		handles.insert(i, Handle::from_memory(f.contents()));
	}
	handles
}

fn gen_square_hashmap() -> HashMap<GenericId, Square> {
	let mut squares_map = HashMap::new();
	for square in ALL_SQUARES {
		squares_map.insert(GenericId::new(square.to_string()), square);
	}
	squares_map
}

// The chess crate has a bug on how it returns the en passant square
// https://github.com/jordanbray/chess/issues/36
// For communication with the engine we need to pass the correct value,
// so this ugly solution is needed.
fn san_correct_ep(fen: String) -> String {
	let mut tokens_vec: Vec<&str> = fen.split_whitespace().collect::<Vec<&str>>();
	let mut new_ep_square = String::from("-");
	if let Some(en_passant) = tokens_vec.get(3) {
		if en_passant != &"-" {
			let rank = if String::from(&en_passant[1..2]).parse::<usize>().unwrap() == 4 { 3 } else { 6 };
			new_ep_square = String::from(&en_passant[0..1]) + &rank.to_string();
		}
	}
	tokens_vec[3] = &new_ep_square;
	tokens_vec.join(" ")
}

fn get_notation_string(board: Board, promo_piece: Piece, from: Square, to: Square) -> String {
	let mut move_made_notation = from.to_string() + &to.to_string();
	let piece = board.piece_on(from);
	let color = board.color_on(from);

	// Check for promotion and adjust the notation accordingly
	if let (Some(piece), Some(color)) = (piece, color) {
		if piece == Piece::Pawn && ((color == Color::White && to.get_rank() == Rank::Eighth) || (color == Color::Black && to.get_rank() == Rank::First)) {
			match promo_piece {
				Piece::Rook => move_made_notation += "r",
				Piece::Knight => move_made_notation += "n",
				Piece::Bishop => move_made_notation += "b",
				_ => move_made_notation += "q",
			}
		}
	}
	move_made_notation
}

//#[derive(Clone)]
struct OfflinePuzzles {
	pub window_id: Option<iced::window::Id>,
	has_lichess_db: bool,
	from_square: Option<Square>,
	board: Board,
	last_move_from: Option<Square>,
	last_move_to: Option<Square>,
	hint_square: Option<Square>,
	puzzle_status: String,
	puzzle_status_color: iced::Color,
	puzzle_number_ui: String,

	analysis: Game,
	analysis_history: Vec<Board>,
	engine_state: EngineStatus,
	engine_eval: String,
	engine: Engine,
	engine_sender: Option<Sender<String>>,
	engine_move: String,

	downloading_db: bool,
	download_progress: String,
	active_tab: TabId,
	search_tab: SearchTab,
	settings_tab: SettingsTab,
	puzzle_tab: PuzzleTab,
	game_mode: config::GameMode,
	sound_playback: Option<SoundPlayback>,
	lang: lang::Language,
	mini_ui: bool,
	square_ids: HashMap<GenericId, Square>,
	piece_imgs: Vec<Handle>,
}

impl Default for OfflinePuzzles {
	fn default() -> Self {
		OfflinePuzzles::new(false)
	}
}

impl OfflinePuzzles {
	pub fn new(has_lichess_db: bool) -> Self {
		Self {
			window_id: None,
			has_lichess_db,
			from_square: None,
			board: Board::default(),
			last_move_from: None,
			last_move_to: None,
			hint_square: None,

			analysis: Game::new(),
			analysis_history: vec![Board::default()],
			engine_state: EngineStatus::TurnedOff,
			engine_eval: String::new(),
			engine: Engine::new(
				config::SETTINGS.engine_path.clone(),
				config::SETTINGS.engine_limit.clone(),
				String::from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
			),
			engine_sender: None,
			engine_move: String::new(),

			downloading_db: false,
			download_progress: String::new(),
			puzzle_status: lang::tr(&config::SETTINGS.lang, "use_search"),
			puzzle_status_color: RED,
			puzzle_number_ui: String::from("1"),
			search_tab: SearchTab::new(),
			settings_tab: SettingsTab::new(),
			puzzle_tab: PuzzleTab::new(),
			active_tab: TabId::Search,

			game_mode: config::GameMode::Puzzle,
			sound_playback: SoundPlayback::init_sound(),
			lang: config::SETTINGS.lang,
			mini_ui: false,
			square_ids: gen_square_hashmap(),
			piece_imgs: get_image_handles(&config::SETTINGS.piece_theme),
		}
	}

	fn verify_and_make_move(&mut self, from: Square, to: Square) {
		let side = match self.game_mode {
			config::GameMode::Analysis => self.analysis.side_to_move(),
			config::GameMode::Puzzle => self.board.side_to_move(),
		};
		let color = match self.game_mode {
			config::GameMode::Analysis => self.analysis.current_position().color_on(to),
			config::GameMode::Puzzle => self.board.color_on(to),
		};
		// If the user clicked on another piece of his own side,
		// just replace the previous selection and exit
		if self.puzzle_tab.game_status == GameStatus::Playing && color == Some(side) {
			self.from_square = Some(to);
			return;
		}
		self.from_square = None;

		if self.game_mode == config::GameMode::Analysis {
			let move_made_notation = get_notation_string(self.analysis.current_position(), self.search_tab.piece_to_promote_to, from, to);

			let move_made = ChessMove::new(
				Square::from_str(&String::from(&move_made_notation[..2])).unwrap(),
				Square::from_str(&String::from(&move_made_notation[2..4])).unwrap(),
				PuzzleTab::check_promotion(&move_made_notation),
			);

			if self.analysis.make_move(move_made) {
				self.analysis_history.push(self.analysis.current_position());
				self.engine.position = self.analysis.current_position().to_string();
				if let Some(sender) = &self.engine_sender {
					if let Err(e) = sender.blocking_send(san_correct_ep(self.analysis.current_position().to_string())) {
						eprintln!("Lost contact with the engine: {}", e);
					}
				}
				if self.settings_tab.saved_configs.play_sound {
					if let Some(audio) = &self.sound_playback {
						audio.play_one();
					}
				}
			}
		} else if !self.puzzle_tab.puzzles.is_empty() {
			let movement;
			let move_made_notation = get_notation_string(self.board, self.search_tab.piece_to_promote_to, from, to);

			let move_made = ChessMove::new(
				Square::from_str(&String::from(&move_made_notation[..2])).unwrap(),
				Square::from_str(&String::from(&move_made_notation[2..4])).unwrap(),
				PuzzleTab::check_promotion(&move_made_notation),
			);

			let is_mate = self.board.legal(move_made) && self.board.make_move_new(move_made).status() == BoardStatus::Checkmate;

			let correct_moves: Vec<&str> = self.puzzle_tab.puzzles[self.puzzle_tab.current_puzzle].moves.split_whitespace().collect::<Vec<&str>>();
			let correct_move = ChessMove::new(
				Square::from_str(&String::from(&correct_moves[self.puzzle_tab.current_puzzle_move][..2])).unwrap(),
				Square::from_str(&String::from(&correct_moves[self.puzzle_tab.current_puzzle_move][2..4])).unwrap(),
				PuzzleTab::check_promotion(correct_moves[self.puzzle_tab.current_puzzle_move]),
			);

			// If the move is correct we can apply it to the board
			if is_mate || (move_made == correct_move) {
				self.board = self.board.make_move_new(move_made);
				self.analysis_history.push(self.board);

				self.puzzle_tab.current_puzzle_move += 1;

				if self.puzzle_tab.current_puzzle_move == correct_moves.len() {
					if self.settings_tab.saved_configs.play_sound {
						if let Some(audio) = &self.sound_playback {
							audio.play_one();
						}
					}
					if self.puzzle_tab.current_puzzle < self.puzzle_tab.puzzles.len() - 1 {
						if self.settings_tab.saved_configs.auto_load_next {
							self.load_puzzle(true);
						} else {
							self.puzzle_tab.game_status = GameStatus::PuzzleEnded;
							self.puzzle_status = lang::tr(&self.lang, "correct_puzzle");
							self.puzzle_status_color = GREEN;
						}
					} else {
						if self.settings_tab.saved_configs.auto_load_next {
							self.board = Board::default();
							// quite meaningless but allows the user to use the takeback button
							// to analyze a full game in analysis mode after the puzzles ended.
							self.analysis_history = vec![self.board];
							self.puzzle_tab.current_puzzle_move = 1;
							self.puzzle_tab.game_status = GameStatus::NoPuzzles;
						} else {
							self.puzzle_tab.game_status = GameStatus::PuzzleEnded;
						}
						self.last_move_from = None;
						self.last_move_to = None;
						self.puzzle_status = lang::tr(&self.lang, "all_puzzles_done");
						self.puzzle_status_color = YELLOW;
					}
				} else {
					if self.settings_tab.saved_configs.play_sound {
						if let Some(audio) = &self.sound_playback {
							audio.play_two();
						}
					}
					movement = ChessMove::new(
						Square::from_str(&String::from(&correct_moves[self.puzzle_tab.current_puzzle_move][..2])).unwrap(),
						Square::from_str(&String::from(&correct_moves[self.puzzle_tab.current_puzzle_move][2..4])).unwrap(),
						PuzzleTab::check_promotion(correct_moves[self.puzzle_tab.current_puzzle_move]),
					);

					self.last_move_from = Some(movement.get_source());
					self.last_move_to = Some(movement.get_dest());

					self.board = self.board.make_move_new(movement);
					self.analysis_history.push(self.board);

					self.puzzle_tab.current_puzzle_move += 1;
					self.puzzle_status = lang::tr(&self.lang, "correct_move");
					self.puzzle_status_color = YELLOW;
				}
			} else {
				#[allow(clippy::collapsible_else_if)]
				if self.board.side_to_move() == Color::White {
					self.puzzle_status = lang::tr(&self.lang, "wrong_move_white_play");
				} else {
					self.puzzle_status = lang::tr(&self.lang, "wrong_move_black_play");
				}
				self.puzzle_status_color = RED;
			}
		}
	}

	fn load_puzzle(&mut self, inc_counter: bool) {
		self.hint_square = None;
		self.puzzle_tab.current_puzzle_move = 1;
		if inc_counter {
			self.inc_puzzle_counter();
		}
		let puzzle_moves: Vec<&str> = self.puzzle_tab.puzzles[self.puzzle_tab.current_puzzle].moves.split_whitespace().collect();

		// The opponent's last move (before the puzzle starts)
		// is in the "moves" field of the cvs, so we need to apply it.
		self.board = Board::from_str(&self.puzzle_tab.puzzles[self.puzzle_tab.current_puzzle].fen).unwrap();

		let movement = ChessMove::new(
			Square::from_str(&String::from(&puzzle_moves[0][..2])).unwrap(),
			Square::from_str(&String::from(&puzzle_moves[0][2..4])).unwrap(),
			PuzzleTab::check_promotion(puzzle_moves[0]),
		);

		self.last_move_from = Some(movement.get_source());
		self.last_move_to = Some(movement.get_dest());

		self.board = self.board.make_move_new(movement);
		self.analysis_history = vec![self.board];

		if self.board.side_to_move() == Color::White {
			self.puzzle_status = lang::tr(&self.lang, "white_to_move");
		} else {
			self.puzzle_status = lang::tr(&self.lang, "black_to_move");
		}
		self.puzzle_status_color = YELLOW;

		self.puzzle_tab.current_puzzle_side = self.board.side_to_move();
		self.puzzle_tab.current_puzzle_fen = san_correct_ep(self.board.to_string());
		self.puzzle_tab.game_status = GameStatus::Playing;
		self.game_mode = config::GameMode::Puzzle;
	}

	fn inc_puzzle_counter(&mut self) {
		self.puzzle_tab.current_puzzle += 1;
		self.puzzle_number_ui = (self.puzzle_tab.current_puzzle + 1).to_string();
	}

	// Redundant, but just to make the function names clear
	fn dec_puzzle_counter(&mut self) {
		self.puzzle_tab.current_puzzle -= 1;
		self.puzzle_number_ui = (self.puzzle_tab.current_puzzle + 1).to_string();
	}

	// Old Iced application trait stuff
	fn init() -> (OfflinePuzzles, Task<Message>) {
		let has_lichess_db = std::path::Path::new(&config::SETTINGS.puzzle_db_location).exists();
		(
			Self::new(has_lichess_db),
			Task::discard(iced::font::load(Cow::from(config::CHESS_ALPHA_BYTES)))
				.chain(window::get_latest())
				.map(Message::WindowInitialized),
		)
	}

	fn update(&mut self, message: self::Message) -> Task<Message> {
		match (self.from_square, message) {
			(None, Message::SelectSquare(pos)) => {
				let side = match self.game_mode {
					config::GameMode::Analysis => self.analysis.side_to_move(),
					config::GameMode::Puzzle => self.board.side_to_move(),
				};
				let color = match self.game_mode {
					config::GameMode::Analysis => self.analysis.current_position().color_on(pos),
					config::GameMode::Puzzle => self.board.color_on(pos),
				};

				if (self.puzzle_tab.game_status == GameStatus::Playing || self.game_mode == config::GameMode::Analysis) && color == Some(side) {
					self.hint_square = None;
					self.from_square = Some(pos);
				}
				Task::none()
			}
			(Some(from), Message::SelectSquare(to)) if from != to => {
				self.verify_and_make_move(from, to);
				Task::none()
			}
			(Some(_), Message::SelectSquare(to)) => {
				self.from_square = Some(to);
				Task::none()
			}
			(_, Message::TabSelected(selected)) => {
				self.active_tab = selected;
				Task::none()
			}
			(_, Message::Settings(message)) => self.settings_tab.update(message),
			(_, Message::SelectMode(message)) => {
				self.game_mode = message;
				if message == config::GameMode::Analysis {
					self.analysis = Game::new_with_board(self.board);
				} else {
					if self.engine_state != EngineStatus::TurnedOff {
						if let Some(sender) = &self.engine_sender {
							sender.blocking_send(String::from(eval::STOP_COMMAND)).expect("Error stopping engine.");
						}
					}
					self.analysis_history.truncate(self.puzzle_tab.current_puzzle_move);
				}
				Task::none()
			}
			(_, Message::ShowHint) => {
				let moves = self.puzzle_tab.puzzles[self.puzzle_tab.current_puzzle].moves.split_whitespace().collect::<Vec<&str>>();
				if !moves.is_empty() && moves.len() > self.puzzle_tab.current_puzzle_move {
					self.hint_square = Some(Square::from_str(&moves[self.puzzle_tab.current_puzzle_move][..2]).unwrap());
				} else {
					self.hint_square = None;
				}

				Task::none()
			}
			(_, Message::ShowNextPuzzle) => {
				self.inc_puzzle_counter();
				self.load_puzzle(false);
				Task::none()
			}
			(_, Message::ShowPreviousPuzzle) => {
				if self.puzzle_tab.current_puzzle > 0 && self.game_mode == config::GameMode::Puzzle {
					self.dec_puzzle_counter();
					self.load_puzzle(false);
				}
				Task::none()
			}
			(_, Message::GoBackMove) => {
				if self.game_mode == config::GameMode::Analysis && self.analysis_history.len() > self.puzzle_tab.current_puzzle_move {
					self.analysis_history.pop();
					self.analysis = Game::new_with_board(*self.analysis_history.last().unwrap());
					if let Some(sender) = &self.engine_sender {
						if let Err(e) = sender.blocking_send(san_correct_ep(self.analysis.current_position().to_string())) {
							eprintln!("Lost contact with the engine: {}", e);
						}
					}
				}
				Task::none()
			}
			(_, Message::RedoPuzzle) => {
				self.load_puzzle(false);
				Task::none()
			}
			(_, Message::LoadPuzzle(puzzles_vec)) => {
				self.from_square = None;
				self.search_tab.show_searching_msg = false;
				self.game_mode = config::GameMode::Puzzle;
				if self.engine_state != EngineStatus::TurnedOff {
					if let Some(sender) = &self.engine_sender {
						sender.blocking_send(String::from(eval::STOP_COMMAND)).expect("Error stopping engine.");
					}
				}
				if let Some(puzzles_vec) = puzzles_vec {
					if !puzzles_vec.is_empty() {
						self.puzzle_tab.puzzles = puzzles_vec;
						self.puzzle_tab.puzzles.shuffle(&mut rand::rng());
						self.puzzle_tab.current_puzzle = 0;
						self.puzzle_number_ui = String::from("1");
						self.load_puzzle(false);
					} else {
						// Just putting the default position to make it obvious the search ended.
						self.board = Board::default();
						self.last_move_from = None;
						self.last_move_to = None;
						self.puzzle_tab.game_status = GameStatus::NoPuzzles;
						self.puzzle_status = lang::tr(&self.lang, "no_puzzle_found");
						self.puzzle_status_color = RED;
					}
				} else {
					self.board = Board::default();
					self.last_move_from = None;
					self.last_move_to = None;
					self.puzzle_tab.game_status = GameStatus::NoPuzzles;
					self.puzzle_status = lang::tr(&self.lang, "no_puzzle_found");
					self.puzzle_status_color = RED;
				}
				Task::none()
			}
			(_, Message::ChangeSettings(message)) => {
				if let Some(settings) = message {
					self.search_tab.piece_theme_promotion = self.settings_tab.piece_theme;
					self.engine.engine_path = self.settings_tab.engine_path.clone();
					self.lang = settings.lang;
					self.search_tab.lang = self.lang;
					self.search_tab.theme.lang = self.lang;
					self.search_tab.opening.lang = self.lang;
					self.puzzle_tab.lang = self.lang;
					self.settings_tab.saved_configs = settings;
					self.piece_imgs = get_image_handles(&self.settings_tab.piece_theme);
					self.search_tab.promotion_piece_img = search_tab::gen_piece_vec(&self.settings_tab.piece_theme);
				}
				Task::none()
			}
			(_, Message::PuzzleInfo(message)) => self.puzzle_tab.update(message),
			(_, Message::Search(message)) => self.search_tab.update(message),
			(_, Message::PuzzleInputIndexChange(puzzle_input)) => {
				self.puzzle_number_ui = puzzle_input;
				Task::none()
			}
			(_, Message::JumpToPuzzle) => {
				// Test if puzzle index typed is valid
				let puzzle_index = self.puzzle_number_ui.parse::<usize>();
				if let Ok(index) = puzzle_index {
					if index > 0 && index <= self.puzzle_tab.puzzles.len() {
						// The user typed value starts on 1, not zero, so we subtract 1
						self.puzzle_tab.current_puzzle = index - 1;
					}
				}
				self.load_puzzle(false);
				Task::none()
			}
			(_, Message::ScreenshotCreated(screenshot)) => Task::perform(screenshot_save_dialog(screenshot), Message::SaveScreenshot),
			(_, Message::SaveScreenshot(img_and_path)) => {
				let (crop_height, crop_width) = if self.settings_tab.show_coordinates {
					(self.settings_tab.window_height - 118., self.settings_tab.window_height - 123.)
				} else {
					(self.settings_tab.window_height - 128., self.settings_tab.window_height - 128.)
				};
				if let Some(img_and_path) = img_and_path {
					let screenshot = img_and_path.0;
					let path = img_and_path.1;
					let crop = screenshot.crop(Rectangle::<u32> { x: 0, y: 0, width: crop_width as u32, height: crop_height as u32 });
					if let Ok(screenshot) = crop {
						let img = RgbaImage::from_raw(screenshot.size.width, screenshot.size.height, screenshot.bytes.to_vec());
						if let Some(image) = img {
							let _ = image.save_with_format(path, image::ImageFormat::Jpeg);
						}
					}
				}
				Task::none()
			}
			(_, Message::ExportPDF(file_path)) => {
				if let Some(file_path) = file_path {
					export::to_pdf(&self.puzzle_tab.puzzles, self.settings_tab.export_pgs.parse::<i32>().unwrap(), &self.lang, file_path);
				}
				Task::none()
			}
			(_, Message::EventOccurred(event)) => {
				if let Event::Window(window::Event::CloseRequested) = event {
					match self.engine_state {
						EngineStatus::TurnedOff => iced::window::get_maximized(self.window_id.unwrap()).map(Message::SaveMaximizedStatusAndExit),
						_ => {
							if let Some(sender) = &self.engine_sender {
								sender.blocking_send(String::from(eval::EXIT_APP_COMMAND)).expect("Error stopping engine.");
							}
							Task::none()
						}
					}
				} else if let Event::Window(window::Event::Resized(size)) = event {
					if !self.mini_ui {
						self.settings_tab.window_width = size.width;
						self.settings_tab.window_height = size.height;
					}
					Task::none()
				} else {
					Task::none()
				}
			}
			(_, Message::SaveMaximizedStatusAndExit(is_maximized)) => {
				self.settings_tab.maximized = is_maximized;
				self.settings_tab.save_window_size();
				window::close(self.window_id.unwrap())
			}
			(_, Message::EngineFileChosen(engine_path)) => {
				if let Some(engine_path) = engine_path {
					self.settings_tab.engine_path = engine_path.clone();
					self.engine.engine_path = engine_path;
				}
				Task::none()
			}
			(_, Message::StartEngine) => {
				match self.engine_state {
					EngineStatus::TurnedOff => {
						//Check if the path is correct first
						if Path::new(&self.engine.engine_path).exists() {
							self.engine.position = san_correct_ep(self.analysis.current_position().to_string());
							self.engine_state = EngineStatus::Started;
						}
					}
					_ => {
						if let Some(sender) = &self.engine_sender {
							sender.blocking_send(String::from(eval::STOP_COMMAND)).expect("Error stopping engine.");
							self.engine_sender = None;
						}
					}
				}
				Task::none()
			}
			(_, Message::EngineStopped(exit)) => {
				self.engine_state = EngineStatus::TurnedOff;
				if exit {
					self.settings_tab.save_window_size();
					window::close(self.window_id.unwrap())
				} else {
					self.engine_eval = String::new();
					self.engine_move = String::new();
					Task::none()
				}
			}
			(_, Message::EngineReady(sender)) => {
				self.engine_sender = Some(sender);
				Task::none()
			}
			(_, Message::UpdateEval(eval)) => {
				match self.engine_state {
					EngineStatus::TurnedOff => Task::none(),
					_ => {
						let (eval, best_move) = eval;
						if let Some(eval_str) = eval {
							if eval_str.contains("Mate") {
								let tokens: Vec<&str> = eval_str.split_whitespace().collect();
								let distance_to_mate_num = tokens[2].parse::<i32>().unwrap();
								match distance_to_mate_num {
									1.. => {
										self.engine_eval = lang::tr(&self.lang, "mate_in") + &distance_to_mate_num.to_string();
									}
									0 => {
										self.engine_eval = lang::tr(&self.lang, "mate");
										self.engine_move = String::from("");
										return Task::none();
									}
									_ => {
										self.engine_eval = lang::tr(&self.lang, "mate_in") + &(-distance_to_mate_num).to_string();
									}
								};
							} else if self.analysis.side_to_move() == Color::White {
								self.engine_eval = eval_str;
							} else {
								// Invert to keep the values relative to white,
								// like it's usually done in GUIs.
								let eval = (eval_str.parse::<f32>().unwrap() * -1.).to_string();
								self.engine_eval = eval.to_string().clone();
							}
						}
						if let Some(best_move) = best_move {
							if let Some(best_move) = config::coord_to_san(&self.analysis.current_position(), best_move, &self.lang) {
								self.engine_move = best_move;
							}
						}
						Task::none()
					}
				}
			}
			(_, Message::StartDBDownload) => {
				self.downloading_db = true;
				Task::none()
			}
			(_, Message::DBDownloadFinished) => {
				self.downloading_db = false;
				self.has_lichess_db = true;
				Task::none()
			}
			(_, Message::DownloadProgress(progress)) => {
				self.download_progress = progress;
				Task::none()
			}
			(_, Message::FavoritePuzzle) => {
				db::toggle_favorite(self.puzzle_tab.puzzles[self.puzzle_tab.current_puzzle].clone());
				Task::none()
			}
			(_, Message::WindowInitialized(id)) => {
				self.window_id = id;
				self.puzzle_tab.window_id = id;
				iced::window::maximize(self.window_id.unwrap(), self.settings_tab.maximized)
			}
			(_, Message::MinimizeUI) => {
				if self.mini_ui {
					self.mini_ui = false;
					let new_size = Size::new(self.settings_tab.window_width, self.settings_tab.window_height);
					iced::window::resize(self.window_id.unwrap(), new_size)
				} else {
					self.mini_ui = true;
					// "110" accounts for the buttons below the board, since the board
					// is a square, we make the width the same as the height,
					// with just a bit extra for the > button
					let new_size = Size::new((self.settings_tab.window_height - 120.) + 25., self.settings_tab.window_height);
					iced::window::resize(self.window_id.unwrap(), new_size)
				}
			}
			(_, Message::DropPiece(square, cursor_pos, _bounds)) => {
				if self.puzzle_tab.game_status == GameStatus::Playing {
					iced_drop::zones_on_point(move |zones| Message::HandleDropZones(square, zones), cursor_pos, None, None)
				} else {
					Task::none()
				}
			}
			(_, Message::HandleDropZones(from, zones)) => {
				if !zones.is_empty() {
					let id: &GenericId = &zones[0].0.clone();
					if let Some(to) = self.square_ids.get(id) {
						self.verify_and_make_move(from, *to);
					}
				}
				Task::none()
			}
		}
	}

	fn subscription(&self) -> Subscription<Message> {
		match self.engine_state {
			EngineStatus::TurnedOff => {
				if self.downloading_db {
					Subscription::batch(vec![
						download_lichess_db(String::from(LICHESS_DB_URL), config::SETTINGS.puzzle_db_location.clone()),
						event::listen().map(Message::EventOccurred),
					])
				} else {
					event::listen().map(Message::EventOccurred)
				}
			}
			_ => Subscription::batch(vec![self.engine.run_engine(), event::listen().map(Message::EventOccurred)]),
		}
	}

	fn view(&self) -> Element<Message, Theme, iced::Renderer> {
		if self.has_lichess_db {
			let has_previous = !self.puzzle_tab.puzzles.is_empty() && self.puzzle_tab.current_puzzle > 0;
			let has_more_puzzles = !self.puzzle_tab.puzzles.is_empty() && self.puzzle_tab.current_puzzle < self.puzzle_tab.puzzles.len() - 1;
			let is_fav = if self.puzzle_tab.puzzles.is_empty() {
				false
			} else {
				db::is_favorite(&self.puzzle_tab.puzzles[self.puzzle_tab.current_puzzle].puzzle_id)
			};
			let resp = responsive(move |size| {
				gen_view(
					self.game_mode,
					self.puzzle_tab.current_puzzle_side,
					self.settings_tab.flip_board,
					self.settings_tab.show_coordinates,
					&self.board,
					&self.analysis.current_position(),
					self.from_square,
					self.last_move_from,
					self.last_move_to,
					self.hint_square,
					self.settings_tab.saved_configs.piece_theme,
					&self.puzzle_status,
					self.puzzle_status_color,
					is_fav,
					has_more_puzzles,
					has_previous,
					self.analysis_history.len(),
					&self.puzzle_number_ui,
					self.puzzle_tab.puzzles.len(),
					self.puzzle_tab.current_puzzle_move,
					self.puzzle_tab.game_status,
					&self.active_tab,
					&self.engine_eval,
					&self.engine_move,
					self.engine_state != EngineStatus::TurnedOff,
					self.search_tab.tab_label(),
					self.settings_tab.tab_label(),
					self.puzzle_tab.tab_label(),
					self.search_tab.view(),
					self.settings_tab.view(),
					self.puzzle_tab.view(),
					&self.lang,
					size,
					self.mini_ui,
					&self.piece_imgs,
				)
			});
			Container::new(resp).padding(1).into()
		} else {
			let mut col = Column::new()
				.push(container(
					Text::new(lang::tr(&self.lang, "db_not_found")).size(30).width(Length::Fill).align_x(alignment::Horizontal::Center),
				))
				.push(Text::new(lang::tr(&self.lang, "do_you_wanna_download")).width(Length::Fill).align_x(alignment::Horizontal::Center))
				.push(Text::new(lang::tr(&self.lang, "download_size_info")).width(Length::Fill).align_x(alignment::Horizontal::Center));
			if self.downloading_db {
				col = col
					.push(container(button(Text::new(lang::tr(&self.lang, "downloading")))).width(Length::Fill).center_x(Length::Fill).padding(20))
					.push(Text::new(&self.download_progress).size(20).width(Length::Fill).align_x(alignment::Horizontal::Center));
			} else {
				col = col.push(
					container(button(Text::new(lang::tr(&self.lang, "download_btn"))).on_press(Message::StartDBDownload))
						.width(Length::Fill)
						.center_x(Length::Fill)
						.padding(20),
				);
			};
			center(col).padding(1).into()
		}
	}

	fn theme(&self) -> iced::Theme {
		iced::Theme::custom(String::from("Theme"), self.settings_tab.board_theme.palette().into())
	}
}

pub async fn screenshot_save_dialog(img: Screenshot) -> Option<(Screenshot, String)> {
	let file_path = AsyncFileDialog::new().add_filter("jpg", &["jpg", "jpeg"]).save_file().await;
	file_path.map(|file_path| (img, file_path.path().display().to_string()))
}

fn gen_view<'a>(
	game_mode: config::GameMode, current_puzzle_side: Color, flip_board: bool, show_coordinates: bool, board: &Board, analysis: &Board,
	from_square: Option<Square>, last_move_from: Option<Square>, last_move_to: Option<Square>, hint_square: Option<Square>, piece_theme: styles::PieceTheme,
	puzzle_status: &'a str, puzzle_status_color: iced::Color, is_fav: bool, has_more_puzzles: bool, has_previous: bool, analysis_history_len: usize,
	puzzle_number_ui: &'a str, total_puzzles: usize, current_puzzle_move: usize, game_status: GameStatus, active_tab: &TabId, engine_eval: &str,
	engine_move: &str, engine_started: bool, search_tab_label: TabLabel, settings_tab_label: TabLabel, puzzle_tab_label: TabLabel,
	search_tab: Element<'a, Message, Theme, iced::Renderer>, settings_tab: Element<'a, Message, Theme, iced::Renderer>,
	puzzle_tab: Element<'a, Message, Theme, iced::Renderer>, lang: &lang::Language, size: Size, mini_ui: bool, imgs: &[Handle],
) -> Element<'a, Message, Theme, iced::Renderer> {
	let font = piece_theme == PieceTheme::FontAlpha;
	let mut board_col = Column::new().spacing(0).align_x(Alignment::Center);
	let mut board_row = Row::new().spacing(0).align_y(Alignment::Center);

	let is_white = (current_puzzle_side == Color::White) ^ flip_board;

	// Reserve more space below the board if we'll show the engine eval
	let board_height = if engine_eval.is_empty() {
		if show_coordinates { ((size.height - 165. - 12.) / 8.) as u16 } else { ((size.height - 135. - 12.) / 8.) as u16 }
	} else if show_coordinates {
		((size.height - 195. - 12.) / 8.) as u16
	} else {
		((size.height - 165. - 12.) / 8.) as u16
	};

	let ranks;
	let files;
	if is_white {
		ranks = (0..8).rev().collect::<Vec<i32>>();
		files = (0..8).collect::<Vec<i32>>();
	} else {
		ranks = (0..8).collect::<Vec<i32>>();
		files = (0..8).rev().collect::<Vec<i32>>();
	};
	for rank in ranks {
		for file in &files {
			let pos = Square::make_square(Rank::from_index(rank as usize), File::from_index(*file as usize));

			let (piece, color) = match game_mode {
				config::GameMode::Analysis => (analysis.piece_on(pos), analysis.color_on(pos)),
				config::GameMode::Puzzle => (board.piece_on(pos), board.color_on(pos)),
			};

			let mut text;
			let light_square = (rank + file) % 2 != 0;

			let selected = if game_mode == config::GameMode::Puzzle {
				from_square == Some(pos) || last_move_from == Some(pos) || last_move_to == Some(pos) || hint_square == Some(pos)
			} else {
				from_square == Some(pos)
			};
			if font {
				let square_style: styles::ChessBtn = if selected { styles::btn_style_light_square } else { styles::btn_style_paper };

				if let Some(piece) = piece {
					if color.unwrap() == Color::White {
						text = match piece {
							Piece::Pawn => String::from("P"),
							Piece::Rook => String::from("R"),
							Piece::Knight => String::from("H"),
							Piece::Bishop => String::from("B"),
							Piece::Queen => String::from("Q"),
							Piece::King => String::from("K"),
						};
					} else {
						text = match piece {
							Piece::Pawn => String::from("O"),
							Piece::Rook => String::from("T"),
							Piece::Knight => String::from("J"),
							Piece::Bishop => String::from("N"),
							Piece::Queen => String::from("W"),
							Piece::King => String::from("L"),
						};
					}
					if light_square {
						text = text.to_lowercase();
					}
				} else if light_square {
					text = String::from(" ");
				} else {
					text = String::from("+");
				}

				board_row = board_row.push(
					Button::new(
						Text::new(text)
							.width(board_height)
							.height(board_height)
							.font(config::CHESS_ALPHA)
							.size(board_height)
							.align_y(alignment::Vertical::Center)
							.line_height(LineHeight::Absolute(board_height.into())),
					)
					.padding(0)
					.on_press(Message::SelectSquare(pos))
					.style(square_style),
				);
			} else {
				let square_style: styles::ChessBtn;
				let container_style: styles::ChessboardContainer;

				if light_square {
					if selected {
						square_style = styles::btn_style_selected_light_square;
						container_style = styles::container_style_selected_light_square;
					} else {
						square_style = styles::btn_style_light_square;
						container_style = styles::container_style_light_square;
					}
				} else if selected {
					square_style = styles::btn_style_selected_dark_square;
					container_style = styles::container_style_selected_dark_square;
				} else {
					square_style = styles::btn_style_dark_square;
					container_style = styles::container_style_dark_square;
				}

				if let Some(piece) = piece {
					let piece_index = if color.unwrap() == Color::White {
						match piece {
							Piece::Pawn => PieceWithColor::WhitePawn.index(),
							Piece::Rook => PieceWithColor::WhiteRook.index(),
							Piece::Knight => PieceWithColor::WhiteKnight.index(),
							Piece::Bishop => PieceWithColor::WhiteBishop.index(),
							Piece::Queen => PieceWithColor::WhiteQueen.index(),
							Piece::King => PieceWithColor::WhiteKing.index(),
						}
					} else {
						match piece {
							Piece::Pawn => PieceWithColor::BlackPawn.index(),
							Piece::Rook => PieceWithColor::BlackRook.index(),
							Piece::Knight => PieceWithColor::BlackKnight.index(),
							Piece::Bishop => PieceWithColor::BlackBishop.index(),
							Piece::Queen => PieceWithColor::BlackQueen.index(),
							Piece::King => PieceWithColor::BlackKing.index(),
						}
					};

					board_row = board_row.push(
						container(
							iced_drop::droppable(Svg::new(imgs[piece_index].clone()).width(board_height).height(board_height))
								.drag_hide(true)
								.drag_center(true)
								.on_drop(move |point, rect| Message::DropPiece(pos, point, rect))
								.on_click(Message::SelectSquare(pos)),
						)
						.style(container_style)
						.id(Id::new(pos.to_string())),
					);
				} else {
					board_row = board_row.push(
						container(
							Button::new(Text::new(""))
								.width(board_height)
								.height(board_height)
								.on_press(Message::SelectSquare(pos))
								.style(square_style),
						)
						.id(Id::new(pos.to_string())),
					);
				}
			}
		}

		if show_coordinates {
			board_row = board_row.push(
				Container::new(Text::new((rank + 1).to_string()).size(25))
					.align_y(iced::alignment::Vertical::Center)
					.align_x(iced::alignment::Horizontal::Right)
					.padding(3)
					.height(board_height),
			);
		}
		board_col = board_col.push(board_row);
		board_row = Row::new().spacing(0).align_y(Alignment::Center);
	}
	if show_coordinates {
		if is_white {
			board_col = board_col.push(row![
				Text::new("     a").size(25).width(board_height),
				Text::new("     b").size(25).width(board_height),
				Text::new("     c").size(25).width(board_height),
				Text::new("     d").size(25).width(board_height),
				Text::new("     e").size(25).width(board_height),
				Text::new("     f").size(25).width(board_height),
				Text::new("     g").size(25).width(board_height),
				Text::new("     h").size(25).width(board_height),
			]);
		} else {
			board_col = board_col.push(row![
				Text::new("     h").size(25).width(board_height),
				Text::new("     g").size(25).width(board_height),
				Text::new("     f").size(25).width(board_height),
				Text::new("     e").size(25).width(board_height),
				Text::new("     d").size(25).width(board_height),
				Text::new("     c").size(25).width(board_height),
				Text::new("     b").size(25).width(board_height),
				Text::new("     a").size(25).width(board_height),
			]);
		}
	}

	let game_mode_row = row![
		Text::new(lang::tr(lang, "mode")),
		Radio::new(lang::tr(lang, "mode_puzzle"), config::GameMode::Puzzle, Some(game_mode), Message::SelectMode),
		Radio::new(lang::tr(lang, "mode_analysis"), config::GameMode::Analysis, Some(game_mode), Message::SelectMode)
	]
	.spacing(10)
	.padding(10)
	.align_y(Alignment::Center);

	let fav_label = if is_fav { lang::tr(lang, "unfav") } else { lang::tr(lang, "fav") };
	let mut navigation_row = Row::new().padding(3).spacing(10);
	if game_mode == config::GameMode::Analysis {
		if analysis_history_len > current_puzzle_move {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "takeback"))).on_press(Message::GoBackMove));
		} else {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "takeback"))));
		}
		if engine_started {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "stop_engine"))).on_press(Message::StartEngine));
		} else {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "start_engine"))).on_press(Message::StartEngine));
		}
	} else {
		if has_previous {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "previous"))).on_press(Message::ShowPreviousPuzzle))
		} else {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "previous"))));
		}
		if has_more_puzzles {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "next"))).on_press(Message::ShowNextPuzzle))
		} else {
			navigation_row = navigation_row.push(Button::new(Text::new(lang::tr(lang, "next"))));
		}
		if game_status == GameStatus::NoPuzzles {
			navigation_row = navigation_row
				.push(Button::new(Text::new(lang::tr(lang, "redo"))))
				.push(Button::new(Text::new(fav_label)))
				.push(Button::new(Text::new(lang::tr(lang, "hint"))));
		} else if game_status == GameStatus::PuzzleEnded {
			navigation_row = navigation_row
				.push(Button::new(Text::new(lang::tr(lang, "redo"))).on_press(Message::RedoPuzzle))
				.push(Button::new(Text::new(fav_label)).on_press(Message::FavoritePuzzle))
				.push(Button::new(Text::new(lang::tr(lang, "hint"))));
		} else {
			navigation_row = navigation_row
				.push(Button::new(Text::new(lang::tr(lang, "redo"))).on_press(Message::RedoPuzzle))
				.push(Button::new(Text::new(fav_label)).on_press(Message::FavoritePuzzle))
				.push(Button::new(Text::new(lang::tr(lang, "hint"))).on_press(Message::ShowHint));
		}
	}

	let (input_index, btn_go) = if game_status == GameStatus::Playing {
		(
			text_input(puzzle_number_ui, puzzle_number_ui).on_input(Message::PuzzleInputIndexChange).width(Length::Fixed(150.)),
			button(text(lang::tr(lang, "go"))).on_press(Message::JumpToPuzzle),
		)
	} else {
		(text_input(puzzle_number_ui, puzzle_number_ui).width(Length::Fixed(150.)), button(text(lang::tr(lang, "go"))))
	};

	let pagination_row = row![text(lang::tr(lang, "puzzle")), input_index, text(lang::tr(lang, "of") + &total_puzzles.to_string()), btn_go]
		.spacing(10)
		.align_y(Alignment::Center);

	board_col = board_col
		.push(Text::new(puzzle_status).color(puzzle_status_color).size(30))
		.push(game_mode_row)
		.push(navigation_row)
		.push(pagination_row);
	if !engine_eval.is_empty() {
		board_col = board_col.push(
			row![
				Text::new(lang::tr(lang, "eval") + engine_eval).size(20).color(YELLOW),
				Text::new(lang::tr(lang, "best_move") + engine_move).size(20).color(GREEN)
			]
			.padding(5)
			.spacing(15),
		);
	}
	if mini_ui {
		let button_mini = Button::new(Text::new(">")).on_press(Message::MinimizeUI);
		row![board_col, button_mini].spacing(5).align_y(Alignment::Start).into()
	} else {
		let button_mini = Button::new(Text::new("<")).on_press(Message::MinimizeUI);
		let tabs = Tabs::new(Message::TabSelected)
			.push(TabId::Search, search_tab_label, search_tab)
			.push(TabId::Settings, settings_tab_label, settings_tab)
			.push(TabId::CurrentPuzzle, puzzle_tab_label, puzzle_tab)
			.tab_bar_position(iced_aw::TabBarPosition::Top)
			.tab_bar_style(styles::tab_style)
			.set_active_tab(active_tab);

		row![board_col, button_mini, tabs].spacing(5).align_y(Alignment::Start).into()
	}
}

trait Tab {
	type Message;

	fn title(&self) -> String;

	fn tab_label(&self) -> TabLabel;

	fn view(&self) -> Element<'_, Self::Message> {
		let column = Column::new().spacing(20).push(Text::new(self.title()).size(HEADER_SIZE)).push(self.content());

		Container::new(column)
			.width(Length::Fill)
			.height(Length::Fill)
			.align_x(alignment::Horizontal::Center)
			.align_y(alignment::Vertical::Center)
			.padding(TAB_PADDING)
			.into()
	}

	fn content(&self) -> Element<'_, Self::Message>;
}

fn main() -> iced::Result {
	let mut def_home = home_dir().unwrap();
	def_home.push(".offline-chess-puzzles");
	let ocp_home = env::var("OCP_HOME").unwrap_or(def_home.display().to_string());
	if let Err(e) = fs::create_dir_all(&ocp_home) {
		eprintln!("{}: can't create directory {}", e, ocp_home);
	}
	_ = env::set_current_dir(&ocp_home);

	let window_settings = iced::window::Settings {
		size: Size { width: config::SETTINGS.window_width, height: config::SETTINGS.window_height },
		resizable: true,
		exit_on_close_request: false,
		..iced::window::Settings::default()
	};

	iced::application("Offline Chess Puzzles", OfflinePuzzles::update, OfflinePuzzles::view)
		.theme(OfflinePuzzles::theme)
		.subscription(OfflinePuzzles::subscription)
		.window(window_settings)
		.run_with(OfflinePuzzles::init)
}
