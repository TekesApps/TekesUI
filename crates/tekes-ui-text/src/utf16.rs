//! UTF-16 偏移与 UTF-8 字节偏移的对应。

/// 把 UTF-16 偏移换成字节偏移的表。偏移落在代理对中间时，取该字符的起点。
#[derive(Clone, Debug)]
pub struct Utf16Map {
    /// `bytes[i]` 是第 `i` 个 UTF-16 单元所在字符的起始字节偏移；末尾多一项（总字节数）。
    bytes: Vec<usize>,
}

impl Utf16Map {
    pub fn new(text: &str) -> Utf16Map {
        let mut bytes = Vec::with_capacity(text.len() + 1);
        for (byte_index, ch) in text.char_indices() {
            for _ in 0..ch.len_utf16() {
                bytes.push(byte_index);
            }
        }
        bytes.push(text.len());
        Utf16Map { bytes }
    }

    /// 文本的 UTF-16 长度。
    pub fn utf16_len(&self) -> usize {
        self.bytes.len() - 1
    }

    pub fn byte_of(&self, utf16: usize) -> usize {
        self.bytes[utf16.min(self.utf16_len())]
    }

    pub fn byte_range(&self, utf16: std::ops::Range<usize>) -> std::ops::Range<usize> {
        self.byte_of(utf16.start)..self.byte_of(utf16.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ascii_cjk_and_astral() {
        let s = "a中😀b";
        let m = Utf16Map::new(s);
        assert_eq!(m.utf16_len(), 5);
        assert_eq!(m.byte_of(0), 0);
        assert_eq!(m.byte_of(1), 1);
        assert_eq!(m.byte_of(2), 4);
        assert_eq!(m.byte_of(3), 4); // 代理对的后半，取字符起点
        assert_eq!(m.byte_of(4), 8);
        assert_eq!(m.byte_of(5), 9);
        assert_eq!(&s[m.byte_range(2..4)], "😀");
    }
}
