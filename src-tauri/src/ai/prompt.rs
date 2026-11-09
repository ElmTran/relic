//! 提示词渲染和译文后处理。

/// 渲染系统提示词，替换 {from} {to}
pub fn render(template: &str, from: &str, to: &str) -> String {
    template.replace("{from}", from).replace("{to}", to)
}

/// 去掉模型自己加的首尾空白、整体包裹的代码块标记和引号（原文本身带的不去）
pub fn clean(output: &str, source: &str) -> String {
    let mut s = output.trim();
    let src = source.trim();

    if s.starts_with("```") && s.ends_with("```") && s.len() >= 6 && !src.starts_with("```") {
        let inner = &s[3..s.len() - 3];
        // 去掉 ``` 后面紧跟的语言标记
        let inner = match inner.split_once('\n') {
            Some((lang, rest)) if !lang.trim().contains(' ') => rest,
            _ => inner,
        };
        s = inner.trim();
    }

    const QUOTES: &[(char, char)] = &[('"', '"'), ('“', '”'), ('「', '」'), ('『', '』'), ('\'', '\'')];
    for &(open, close) in QUOTES {
        if s.starts_with(open)
            && s.ends_with(close)
            && s.chars().count() >= 2
            && !src.starts_with(open)
            // 内部还有同样的引号，说明不是整体包裹，不动
            && !s[open.len_utf8()..s.len() - close.len_utf8()].contains([open, close])
        {
            s = s[open.len_utf8()..s.len() - close.len_utf8()].trim();
            break;
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans() {
        assert_eq!(clean("  \"Hello\"  ", "你好"), "Hello");
        assert_eq!(clean("“你好”", "Hello"), "你好");
        assert_eq!(clean("\"Hello\"", "\"你好\""), "\"Hello\"");
        assert_eq!(clean("\"a\" and \"b\"", "x"), "\"a\" and \"b\"");
        assert_eq!(clean("```text\nHello\n```", "你好"), "Hello");
        assert_eq!(clean("```\nlet a = 1;\n```", "```\nlet a = 1;\n```"), "```\nlet a = 1;\n```");
    }

    #[test]
    fn renders() {
        assert_eq!(render("from {from} to {to}", "English", "Chinese"), "from English to Chinese");
    }
}
