//! 语言检测：先按文字系统判断中日韩，其余交给 whatlang。

/// 支持显示和翻译的语言：(代码, 界面显示名, 提示词里用的英文名)
const LANGS: &[(&str, &str, &str)] = &[
    ("zh", "中文", "Simplified Chinese (简体中文)"),
    ("en", "English", "English"),
    ("ja", "日本語", "Japanese"),
    ("ko", "한국어", "Korean"),
    ("fr", "Français", "French"),
    ("de", "Deutsch", "German"),
    ("es", "Español", "Spanish"),
    ("pt", "Português", "Portuguese"),
    ("it", "Italiano", "Italian"),
    ("ru", "Русский", "Russian"),
    ("ar", "العربية", "Arabic"),
    ("vi", "Tiếng Việt", "Vietnamese"),
    ("th", "ไทย", "Thai"),
];

fn find(code: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    LANGS.iter().find(|(c, _, _)| *c == code)
}

/// 界面显示名，未知代码原样返回
pub fn display_name(code: &str) -> String {
    find(code).map_or_else(|| code.to_string(), |l| l.1.to_string())
}

/// 提示词里用的英文名，未知代码原样返回
pub fn prompt_name(code: &str) -> String {
    find(code).map_or_else(|| code.to_string(), |l| l.2.to_string())
}

/// 检测原文语言，返回语言代码；判断不了返回 None
pub fn detect(text: &str) -> Option<&'static str> {
    let (mut han, mut kana, mut hangul, mut latin_words, mut other) = (0usize, 0, 0, 0, 0);
    let mut in_latin_word = false;
    for c in text.chars() {
        let is_latin = c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(&c);
        if is_latin && !in_latin_word {
            latin_words += 1;
        }
        in_latin_word = is_latin;
        match c {
            '\u{3040}'..='\u{30FF}' | '\u{31F0}'..='\u{31FF}' => kana += 1,
            '\u{AC00}'..='\u{D7AF}' | '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' => hangul += 1,
            '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}' => han += 1,
            c if c.is_alphabetic() && !is_latin => other += 1,
            _ => {}
        }
    }

    if kana > 0 {
        return Some("ja");
    }
    if hangul > 0 && hangul >= han {
        return Some("ko");
    }
    // 中英混排：一个汉字按半个单词算，汉字多就当中文
    if han > 0 && han * 2 >= latin_words {
        return Some("zh");
    }
    if latin_words == 0 && other == 0 {
        return None;
    }

    let info = whatlang::detect(text)?;
    let code = match info.lang() {
        whatlang::Lang::Eng => "en",
        whatlang::Lang::Fra => "fr",
        whatlang::Lang::Deu => "de",
        whatlang::Lang::Spa => "es",
        whatlang::Lang::Por => "pt",
        whatlang::Lang::Ita => "it",
        whatlang::Lang::Rus => "ru",
        whatlang::Lang::Ara => "ar",
        whatlang::Lang::Vie => "vi",
        whatlang::Lang::Tha => "th",
        whatlang::Lang::Cmn => "zh",
        whatlang::Lang::Jpn => "ja",
        whatlang::Lang::Kor => "ko",
        _ => return if other == 0 { Some("en") } else { None },
    };
    // 拉丁字母的短文本 whatlang 经常猜错，不可靠时按英文处理
    if other == 0 && code != "en" && latin_words < 5 && !info.is_reliable() {
        return Some("en");
    }
    Some(code)
}

/// 自动方向：原文是母语就译成第二语言，否则译成母语
pub fn auto_target<'a>(from: Option<&str>, primary: &'a str, secondary: &'a str) -> &'a str {
    if from == Some(primary) { secondary } else { primary }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts() {
        assert_eq!(detect("你好，世界"), Some("zh"));
        assert_eq!(detect("这个 API 怎么用"), Some("zh"));
        assert_eq!(detect("こんにちは世界"), Some("ja"));
        assert_eq!(detect("안녕하세요"), Some("ko"));
        assert_eq!(detect("hello"), Some("en"));
        assert_eq!(detect("Use the 缓存 layer for better performance"), Some("en"));
        assert_eq!(detect("The quick brown fox jumps over the lazy dog"), Some("en"));
        assert_eq!(detect("Le chat est sur la table et il dort tranquillement"), Some("fr"));
        assert_eq!(detect("Привет, как дела? Сегодня хорошая погода"), Some("ru"));
        assert_eq!(detect("123 + 456"), None);
    }

    #[test]
    fn direction() {
        assert_eq!(auto_target(Some("zh"), "zh", "en"), "en");
        assert_eq!(auto_target(Some("ja"), "zh", "en"), "zh");
        assert_eq!(auto_target(None, "zh", "en"), "zh");
    }
}
