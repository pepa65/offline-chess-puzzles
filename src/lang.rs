use fluent_bundle::FluentResource;
use fluent_bundle::bundle::FluentBundle;
use once_cell::sync::Lazy;
use unic_langid::langid;

static BUNDLE_ENUS: Lazy<FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>> = Lazy::new(|| {
	let source = include_str!("../include/translations/en-US/ocp.ftl").to_string();
	let res = FluentResource::try_new(source).expect("Could not parse the FTL file.");
	let mut bundle = FluentBundle::new_concurrent(vec![langid!("en-US")]);
	bundle.add_resource(res).expect("Failed to add FTL resources to the bundle.");
	bundle
});

static BUNDLE_NL: Lazy<FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>> = Lazy::new(|| {
	let source = include_str!("../include/translations/nl/ocp.ftl").to_string();
	let res = FluentResource::try_new(source).expect("Could not parse the FTL file.");
	let mut bundle = FluentBundle::new_concurrent(vec![langid!("nl")]);
	bundle.add_resource(res).expect("Failed to add FTL resources to the bundle.");
	bundle
});

static BUNDLE_PTBR: Lazy<FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>> = Lazy::new(|| {
	let source = include_str!("../include/translations/pt-BR/ocp.ftl").to_string();
	let res = FluentResource::try_new(source).expect("Could not parse the FTL file.");
	let mut bundle = FluentBundle::new_concurrent(vec![langid!("pt-BR")]);
	bundle.add_resource(res).expect("Failed to add FTL resources to the bundle.");
	bundle
});

static BUNDLE_ES: Lazy<FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>> = Lazy::new(|| {
	let source = include_str!("../include/translations/es/ocp.ftl").to_string();
	let res = FluentResource::try_new(source).expect("Could not parse the FTL file.");
	let mut bundle = FluentBundle::new_concurrent(vec![langid!("es")]);
	bundle.add_resource(res).expect("Failed to add FTL resources to the bundle.");
	bundle
});

static BUNDLE_FR: Lazy<FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>> = Lazy::new(|| {
	let source = include_str!("../include/translations/fr/ocp.ftl").to_string();
	let res = FluentResource::try_new(source).expect("Could not parse the FTL file.");
	let mut bundle = FluentBundle::new_concurrent(vec![langid!("fr")]);
	bundle.add_resource(res).expect("Failed to add FTL resources to the bundle.");
	bundle
});

pub fn tr(lang: &Language, key: &str) -> String {
	let bundle = match lang {
		Language::Portuguese => &BUNDLE_PTBR,
		Language::English => &BUNDLE_ENUS,
		Language::Dutch => &BUNDLE_NL,
		Language::Spanish => &BUNDLE_ES,
		Language::French => &BUNDLE_FR,
	};
	let msg = bundle.get_message(key).expect(&("Missing translation key ".to_owned() + key));
	let mut errors = vec![];
	let pattern = msg.value().expect("Missing Value.");
	bundle.format_pattern(pattern, None, &mut errors).to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Language {
	Dutch,
	English,
	Portuguese,
	Spanish,
	French,
}

impl Language {
	pub const ALL: [Language; 5] = [Language::Dutch, Language::English, Language::Portuguese, Language::Spanish, Language::French];
}

impl DisplayTranslated for Language {
	fn to_str_tr(&self) -> &str {
		match self {
			Language::Dutch => "dutch",
			Language::English => "english",
			Language::Portuguese => "portuguese",
			Language::Spanish => "spanish",
			Language::French => "french",
		}
	}
}

#[derive(Debug, Clone)]
pub struct PickListWrapper<D: DisplayTranslated> {
	pub lang: Language,
	pub item: D,
}

pub trait DisplayTranslated {
	fn to_str_tr(&self) -> &str;
}

impl<D: DisplayTranslated> std::fmt::Display for PickListWrapper<D> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&tr(&self.lang, self.item.to_str_tr()))
	}
}

impl<D: DisplayTranslated + std::cmp::PartialEq> PartialEq for PickListWrapper<D> {
	fn eq(&self, other: &Self) -> bool {
		self.item == other.item
	}
}

impl<D: DisplayTranslated + std::cmp::PartialEq> Eq for PickListWrapper<D> {}

impl PickListWrapper<Language> {
	pub fn get_langs(lang: Language) -> Vec<PickListWrapper<Language>> {
		let mut themes_wrapper = Vec::new();
		for item in Language::ALL {
			themes_wrapper.push(PickListWrapper::<Language> { lang, item });
		}
		themes_wrapper
	}

	pub fn new_lang(lang: Language, item: Language) -> Self {
		Self { lang, item }
	}
}
