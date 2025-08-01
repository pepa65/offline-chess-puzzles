use chess::{Color, Piece};
use iced::widget::{Button, Container, Scrollable, Text, TextInput, column as col, row};
use iced::window::Id;
use iced::{Alignment, Element, Length, Task, Theme, alignment};
use iced_aw::TabLabel;
use rfd::AsyncFileDialog;

use crate::{Message, Tab, config, lang};

#[derive(Debug, Clone)]
pub enum PuzzleMessage {
	ChangeTextInputs(String),
	CopyText(String),
	OpenLink(String),
	TakeScreenshot,
	ExportToPDF,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GameStatus {
	Playing,
	PuzzleEnded,
	NoPuzzles,
}

#[derive(Debug, Clone)]
pub struct PuzzleTab {
	pub window_id: Option<Id>,
	pub puzzles: Vec<config::Puzzle>,
	pub current_puzzle: usize,
	pub current_puzzle_move: usize,
	pub current_puzzle_side: Color,
	pub game_status: GameStatus,
	pub current_puzzle_fen: String,
	pub lang: lang::Language,
}

impl PuzzleTab {
	pub fn new() -> Self {
		PuzzleTab {
			window_id: None,
			puzzles: Vec::new(),
			current_puzzle: 0,
			current_puzzle_move: 1,
			current_puzzle_side: Color::White,
			game_status: GameStatus::NoPuzzles,
			current_puzzle_fen: String::new(),
			lang: config::SETTINGS.lang,
		}
	}

	pub fn update(&mut self, message: PuzzleMessage) -> Task<Message> {
		match message {
			PuzzleMessage::ChangeTextInputs(_) => Task::none(),
			PuzzleMessage::CopyText(text) => iced::clipboard::write::<Message>(text),
			PuzzleMessage::OpenLink(link) => {
				let _ = open::that_detached(link);
				Task::none()
			}
			PuzzleMessage::TakeScreenshot => iced::window::screenshot(self.window_id.unwrap()).map(Message::ScreenshotCreated),
			PuzzleMessage::ExportToPDF => Task::perform(PuzzleTab::export(), Message::ExportPDF),
		}
	}

	pub async fn export() -> Option<String> {
		let file_path = AsyncFileDialog::new().save_file().await;
		file_path.map(|file_path| file_path.path().display().to_string())
	}

	// Checks if the notation indicates a promotion and return the piece if that's the case
	pub fn check_promotion(notation: &str) -> Option<Piece> {
		let mut promotion = None;
		if notation.len() > 4 {
			promotion = match &notation[4..5] {
				"r" => Some(Piece::Rook),
				"n" => Some(Piece::Knight),
				"b" => Some(Piece::Bishop),
				_ => Some(Piece::Queen),
			}
		}
		promotion
	}
}

impl Tab for PuzzleTab {
	type Message = Message;

	fn title(&self) -> String {
		lang::tr(&self.lang, "current_puzzle")
	}

	fn tab_label(&self) -> TabLabel {
		TabLabel::Text(self.title())
	}

	fn content(&self) -> Element<'_, Message> {
		let col_puzzle_info = if !self.puzzles.is_empty() && self.current_puzzle < self.puzzles.len() {
			Scrollable::new(
				col![
					Text::new(lang::tr(&self.lang, "puzzle_link")),
					row![
						TextInput::new("", &("https://lichess.org/training/".to_owned() + &self.puzzles[self.current_puzzle].puzzle_id),)
							.on_input(PuzzleMessage::ChangeTextInputs),
						Button::new(Text::new(lang::tr(&self.lang, "copy")))
							.on_press(PuzzleMessage::CopyText("https://lichess.org/training/".to_owned() + &self.puzzles[self.current_puzzle].puzzle_id)),
						Button::new(Text::new(lang::tr(&self.lang, "open")))
							.on_press(PuzzleMessage::OpenLink("https://lichess.org/training/".to_owned() + &self.puzzles[self.current_puzzle].puzzle_id)),
					],
					Text::new(lang::tr(&self.lang, "fen")),
					row![
						TextInput::new(&self.current_puzzle_fen, &self.current_puzzle_fen,).on_input(PuzzleMessage::ChangeTextInputs),
						Button::new(Text::new(lang::tr(&self.lang, "copy"))).on_press(PuzzleMessage::CopyText(self.current_puzzle_fen.clone())),
					],
					Text::new(lang::tr(&self.lang, "rating") + &self.puzzles[self.current_puzzle].rating.to_string()),
					Text::new(lang::tr(&self.lang, "rd") + &self.puzzles[self.current_puzzle].rating_deviation.to_string()),
					Text::new(lang::tr(&self.lang, "popularity") + &self.puzzles[self.current_puzzle].popularity.to_string()),
					Text::new(lang::tr(&self.lang, "times_played") + &self.puzzles[self.current_puzzle].nb_plays.to_string()),
					Text::new(lang::tr(&self.lang, "themes")),
					Text::new(&self.puzzles[self.current_puzzle].themes),
					Text::new(lang::tr(&self.lang, "url")),
					row![
						TextInput::new(&self.puzzles[self.current_puzzle].game_url, &self.puzzles[self.current_puzzle].game_url,)
							.on_input(PuzzleMessage::ChangeTextInputs),
						Button::new(Text::new(lang::tr(&self.lang, "copy")))
							.on_press(PuzzleMessage::CopyText(self.puzzles[self.current_puzzle].game_url.clone())),
						Button::new(Text::new(lang::tr(&self.lang, "open")))
							.on_press(PuzzleMessage::OpenLink(self.puzzles[self.current_puzzle].game_url.clone())),
					],
					Button::new(Text::new(lang::tr(&self.lang, "screenshot"))).on_press(PuzzleMessage::TakeScreenshot),
					Button::new(Text::new(lang::tr(&self.lang, "export_pdf_btn"))).on_press(PuzzleMessage::ExportToPDF),
				]
				.padding([0, 30])
				.spacing(10)
				.align_x(Alignment::Center),
			)
		} else {
			Scrollable::new(col![Text::new(lang::tr(&self.lang, "no_puzzle")).align_x(alignment::Horizontal::Center).width(Length::Fill)].spacing(10))
		};
		let content: Element<PuzzleMessage, Theme, iced::Renderer> =
			Container::new(col_puzzle_info).align_x(alignment::Horizontal::Center).height(Length::Fill).into();

		content.map(Message::PuzzleInfo)
	}
}
