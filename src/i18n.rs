use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use colored::Colorize;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub code: &'static str,
    pub name: &'static str,
    pub native_name: &'static str,
}

pub const SUPPORTED_LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo { code: "english", name: "English", native_name: "English" },
    LanguageInfo { code: "chinese_simplified", name: "Chinese (Simplified)", native_name: "简体中文" },
    LanguageInfo { code: "chinese_traditional", name: "Chinese (Traditional)", native_name: "繁體中文" },
    LanguageInfo { code: "japanese", name: "Japanese", native_name: "日本語" },
    LanguageInfo { code: "korean", name: "Korean", native_name: "한국어" },
    LanguageInfo { code: "spanish", name: "Spanish", native_name: "Español" },
    LanguageInfo { code: "french", name: "French", native_name: "Français" },
    LanguageInfo { code: "deutsch", name: "German", native_name: "Deutsch" },
    LanguageInfo { code: "russian", name: "Russian", native_name: "Русский" },
    LanguageInfo { code: "portuguese", name: "Portuguese", native_name: "Português" },
    LanguageInfo { code: "italian", name: "Italian", native_name: "Italiano" },
];

const DEFAULT_LANGUAGE: &str = "english";

fn embedded_language(code: &str) -> Option<&'static str> {
    match code {
        "english" => Some(include_str!("../assets/languages/en.json")),
        "chinese_simplified" => Some(include_str!("../assets/languages/zh.json")),
        "chinese_traditional" => Some(include_str!("../assets/languages/zh_tw.json")),
        "japanese" => Some(include_str!("../assets/languages/ja.json")),
        "korean" => Some(include_str!("../assets/languages/ko.json")),
        "spanish" => Some(include_str!("../assets/languages/es.json")),
        "french" => Some(include_str!("../assets/languages/fr.json")),
        "deutsch" => Some(include_str!("../assets/languages/de.json")),
        "russian" => Some(include_str!("../assets/languages/ru.json")),
        "portuguese" => Some(include_str!("../assets/languages/pt.json")),
        "italian" => Some(include_str!("../assets/languages/it.json")),
        _ => None,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Config {
    language: String,
}

struct I18nState {
    language: String,
    translations: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

static I18N: OnceLock<Mutex<Option<I18nState>>> = OnceLock::new();

fn i18n_mutex() -> &'static Mutex<Option<I18nState>> {
    I18N.get_or_init(|| Mutex::new(None))
}

fn get_config_dir() -> Option<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "execrooted", "filebyte")?;
    Some(proj_dirs.config_dir().to_path_buf())
}

fn config_path() -> Option<PathBuf> {
    get_config_dir().map(|dir| dir.join("config.json"))
}

fn load_config() -> Option<Config> {
    let path = config_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Config>(&content).ok()
}

fn save_config(config: &Config) -> bool {
    let path = match config_path() {
        Some(p) => p,
        None => return false,
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let content = serde_json::to_string_pretty(config).unwrap_or_default();
    std::fs::write(&path, content).is_ok()
}

fn parse_translations(json_str: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(value) = serde_json::from_str::<Value>(json_str) {
        if let Value::Object(obj) = value {
            for (key, val) in obj {
                if let Some(s) = val.as_str() {
                    map.insert(key.clone(), s.to_string());
                }
            }
        }
    }
    map
}

pub fn tr(key: &str) -> String {
    if let Ok(guard) = i18n_mutex().lock() {
        if let Some(state) = &*guard {
            if let Some(val) = state.translations.get(key) {
                return val.clone();
            }
            if let Some(val) = state.fallback.get(key) {
                return val.clone();
            }
        }
    }
    let en = include_str!("../assets/languages/en.json");
    if let Ok(value) = serde_json::from_str::<Value>(en) {
        if let Value::Object(obj) = value {
            if let Some(val) = obj.get(key) {
                if let Some(s) = val.as_str() {
                    return s.to_string();
                }
            }
        }
    }
    key.to_string()
}

pub fn tr_format(key: &str, args: &[&str]) -> String {
    let template = tr(key);
    if args.is_empty() {
        return template;
    }
    let mut result = template;
    for arg in args {
        result = result.replacen("{}", arg, 1);
    }
    result
}

pub fn list_languages() -> &'static [LanguageInfo] {
    SUPPORTED_LANGUAGES
}

pub fn get_language_code(name: &str) -> Option<&'static str> {
    for lang in SUPPORTED_LANGUAGES {
        if lang.code == name || lang.name == name || lang.native_name == name {
            return Some(lang.code);
        }
    }
    None
}

fn current_language_code() -> String {
    if let Ok(guard) = i18n_mutex().lock() {
        if let Some(state) = &*guard {
            return state.language.clone();
        }
    }
    DEFAULT_LANGUAGE.to_string()
}

pub fn set_language(lang_code: &str) {
    let json_str = match embedded_language(lang_code) {
        Some(s) => s,
        None => {
            eprintln!("Error: Language '{}' not found, using English", lang_code);
            include_str!("../assets/languages/en.json")
        }
    };

    let translations = parse_translations(json_str);
    let fallback = parse_translations(include_str!("../assets/languages/en.json"));

    let state = I18nState {
        language: lang_code.to_string(),
        translations,
        fallback,
    };

    if let Ok(mut guard) = i18n_mutex().lock() {
        *guard = Some(state);
    }
}

pub fn save_language(lang_code: &str) -> bool {
    let config = Config {
        language: lang_code.to_string(),
    };
    save_config(&config)
}

pub fn get_language_name(code: &str) -> &'static str {
    for lang in SUPPORTED_LANGUAGES {
        if lang.code == code {
            return lang.name;
        }
    }
    "Unknown"
}

pub fn get_language_native_name(code: &str) -> &'static str {
    for lang in SUPPORTED_LANGUAGES {
        if lang.code == code {
            return lang.native_name;
        }
    }
    "Unknown"
}

pub fn config_dir_display() -> String {
    get_config_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn prompt_language_selection(color: bool, first_run: bool) -> Option<String> {
    let languages = list_languages();
    let cfg = load_config();
    let default_lang = cfg.as_ref().map(|c| c.language.as_str()).unwrap_or(DEFAULT_LANGUAGE);

    if first_run {
        println!();
        if color {
            println!("{}", tr("language_selection_first_run").green().bold());
        } else {
            println!("{}", tr("language_selection_first_run"));
        }
    } else {
        if color {
            println!("{}", tr("language_selection_prompt").cyan().bold());
        } else {
            println!("{}", tr("language_selection_prompt"));
        }
    }
    println!();

    for (i, lang) in languages.iter().enumerate() {
        let marker = if lang.code == default_lang {
            " (current)"
        } else {
            ""
        };
         if color {
            println!(
                "  {}  {}: {} [{}]{}",
                (i + 1).to_string().yellow().bold(),
                lang.code,
                lang.name,
                lang.native_name,
                marker
            );
        } else {
            println!(
                "  {}  {}: {} [{}]{}",
                i + 1,
                lang.code,
                lang.name,
                lang.native_name,
                marker
            );
        }
    }
    println!();

    let prompt = if first_run {
        let default_msg = tr("first_run_default");
        format!("{} [1]: ", default_msg)
    } else {
        let current = current_language_code();
        let current_name = get_language_name(&current);
        format!("Select [{}]: ", current_name)
    };

    print!("{}", prompt);
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }
    let input = input.trim();

    if input.is_empty() {
        return if first_run {
            Some("english".to_string())
        } else {
            Some(default_lang.to_string())
        };
    }

    if let Ok(idx) = input.parse::<usize>() {
        if idx > 0 && idx <= languages.len() {
            return Some(languages[idx - 1].code.to_string());
        }
    }

    for lang in languages {
        if lang.code.eq_ignore_ascii_case(input) {
            return Some(lang.code.to_string());
        }
    }

    if first_run {
        Some("english".to_string())
    } else {
        Some(default_lang.to_string())
    }
}

pub fn init(color: bool) -> String {
    let lang_code = if let Some(cfg) = load_config() {
        cfg.language.clone()
    } else {
        let selected = prompt_language_selection(color, true);
        let code = selected.unwrap_or_else(|| DEFAULT_LANGUAGE.to_string());
        save_language(&code);
        if color {
            println!("{}", tr_format("language_saved", &[&code]).green().bold());
        } else {
            println!("{}", tr_format("language_saved", &[&code]));
        }
        code
    };

    set_language(&lang_code);
    lang_code
}

pub fn handle_language_flag(color: bool) {
    let selected = prompt_language_selection(color, false);
    if let Some(code) = selected {
        set_language(&code);
        if save_language(&code) {
            if color {
                println!("{}", tr_format("language_saved", &[&code]).green().bold());
            } else {
                println!("{}", tr_format("language_saved", &[&code]));
            }
            println!("{}: {}", get_language_name(&code), get_language_native_name(&code));
            println!("Config: {}/config.json", config_dir_display());
        } else {
            eprintln!("Warning: Could not save language preference (config directory not writable)");
            eprintln!("Language will reset to default on next run.");
        }
    }
}
