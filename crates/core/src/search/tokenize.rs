//! 中文 n-gram 与标识符拆词（计划 D10）。
//!
//! 中文按“单字 + 双字”生成 token；拉丁标识符额外按 camelCase / snake_case 拆词。
//! 全部 token 存入普通 FTS5（`unicode61`），不把 trigram 宣称为通用中文分词器。

use std::collections::BTreeSet;

/// 是否为中日韩统一表意文字（常用范围）。
pub fn is_cjk(ch: char) -> bool {
    let code = ch as u32;
    (0x4E00..=0x9FFF).contains(&code)
        || (0x3400..=0x4DBF).contains(&code)
        || (0xF900..=0xFAFF).contains(&code)
        || (0x20000..=0x2A6DF).contains(&code)
}

/// 分词：返回去重后的 token 集合（保持稳定顺序）。
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens: BTreeSet<String> = BTreeSet::new();
    let mut cjk_run: Vec<char> = Vec::new();
    let mut word: String = String::new();

    let flush_word = |word: &mut String, tokens: &mut BTreeSet<String>| {
        if word.is_empty() {
            return;
        }
        tokens.insert(word.clone());
        tokens.insert(word.to_lowercase());
        for part in split_identifier(word) {
            if part != *word {
                tokens.insert(part.clone());
                tokens.insert(part.to_lowercase());
            }
        }
        word.clear();
    };

    let flush_cjk = |run: &mut Vec<char>, tokens: &mut BTreeSet<String>| {
        for ch in run.iter() {
            tokens.insert(ch.to_string());
        }
        for window in run.windows(2) {
            let bigram: String = window.iter().collect();
            tokens.insert(bigram);
        }
        run.clear();
    };

    for ch in text.chars() {
        if is_cjk(ch) {
            flush_word(&mut word, &mut tokens);
            cjk_run.push(ch);
            continue;
        }
        flush_cjk(&mut cjk_run, &mut tokens);
        if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            word.push(ch);
        } else {
            flush_word(&mut word, &mut tokens);
        }
    }
    flush_word(&mut word, &mut tokens);
    flush_cjk(&mut cjk_run, &mut tokens);

    tokens.into_iter().collect()
}

/// 空格分隔的 token 串（写入 `search_documents.tokens`）。
pub fn token_string(text: &str) -> String {
    tokenize(text).join(" ")
}

/// camelCase / snake_case / kebab-case 拆词。
pub fn split_identifier(word: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = word.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '_' || *ch == '-' {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            continue;
        }
        let previous = if index == 0 {
            None
        } else {
            Some(chars[index - 1])
        };
        let next = chars.get(index + 1).copied();
        let boundary = match previous {
            Some(prev) => {
                (ch.is_uppercase() && prev.is_lowercase())
                    || (ch.is_uppercase() && prev.is_numeric())
                    || (ch.is_numeric() && !prev.is_numeric() && prev.is_alphanumeric())
                    || (ch.is_uppercase()
                        && next.map(|n| n.is_lowercase()).unwrap_or(false)
                        && prev.is_uppercase())
            }
            None => false,
        };
        if boundary && !current.is_empty() {
            parts.push(std::mem::take(&mut current));
        }
        current.push(*ch);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}
