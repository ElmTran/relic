//! 增量 SSE 解析。只关心 data 字段，按空行切分事件。

#[derive(Default)]
pub struct SseParser {
    buf: Vec<u8>,
    data: String,
}

impl SseParser {
    /// 喂入一段字节，把解析出的完整事件的 data 追加到 out
    pub fn push(&mut self, chunk: &[u8], out: &mut Vec<String>) {
        self.buf.extend_from_slice(chunk);
        let mut start = 0;
        while let Some(i) = self.buf[start..].iter().position(|&b| b == b'\n') {
            let end = start + i;
            let line = &self.buf[start..end];
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            // 只解码完整的行，所以 UTF-8 字符被切在两个 chunk 之间也没关系
            let line = String::from_utf8_lossy(line).into_owned();
            self.line(&line, out);
            start = end + 1;
        }
        self.buf.drain(..start);
    }

    /// 流结束：处理最后一行和没有以空行结尾的事件
    pub fn finish(&mut self, out: &mut Vec<String>) {
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&std::mem::take(&mut self.buf)).into_owned();
            self.line(line.trim_end_matches('\r'), out);
        }
        self.line("", out);
    }

    fn line(&mut self, line: &str, out: &mut Vec<String>) {
        if line.is_empty() {
            if !self.data.is_empty() {
                out.push(std::mem::take(&mut self.data));
            }
            return;
        }
        if line.starts_with(':') {
            return; // 注释 / 心跳
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        if field == "data" {
            if !self.data.is_empty() {
                self.data.push('\n');
            }
            self.data.push_str(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_chunks(chunks: &[&[u8]]) -> Vec<String> {
        let mut p = SseParser::default();
        let mut out = vec![];
        for c in chunks {
            p.push(c, &mut out);
        }
        p.finish(&mut out);
        out
    }

    #[test]
    fn basic_and_split() {
        let s = "data: {\"a\":1}\n\n: keep-alive\n\ndata: [DONE]\n\n".as_bytes();
        assert_eq!(parse_chunks(&[s]), vec!["{\"a\":1}", "[DONE]"]);
        let (a, b) = s.split_at(9);
        assert_eq!(parse_chunks(&[a, b]), vec!["{\"a\":1}", "[DONE]"]);
    }

    #[test]
    fn crlf_multiline_and_utf8_split() {
        let s = "data: 你好\r\ndata: 世界\r\n\r\ndata: tail".as_bytes();
        let (a, b) = s.split_at(7); // 切在"你"的中间
        assert_eq!(parse_chunks(&[a, b]), vec!["你好\n世界", "tail"]);
    }
}
