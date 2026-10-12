use crate::knowledge::Command;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::{collections::HashMap, sync::LazyLock};

const STOPWORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "to",
    "how",
    "do",
    "i",
    "can",
    "for",
    "of",
    "in",
    "on",
    "by",
    "with",
    "and",
    "my",
    "please",
    "want",
    "need",
    "me",
    "is",
    "it",
    "its",
    "this",
    "that",
    "way",
    "command",
    "what",
    "whats",
    "which",
    "should",
    "using",
    "use",
    "am",
    "are",
    "does",
    "there",
    "some",
    "any",
    "too",
    "just",
    "much",
    "many",
    "one",
    "few",
    "be",
    "was",
    "been",
    "has",
    "have",
    "so",
    "then",
    "we",
    "our",
    "your",
    "you",
    "currently",
    "again",
    "also",
];

// Canonical word first; every other word in the group is rewritten to it.
const SYNONYMS: &[&[&str]] = &[
    &["directory", "folder", "dir"],
    &[
        "delete", "remove", "erase", "wipe", "del", "rm", "nuke", "destroy",
    ],
    &["list", "show", "display", "view", "see"],
    &["create", "make", "new"],
    &["stop", "terminate", "halt"],
    &["restart", "reboot", "bounce"],
    &["large", "big", "bigger", "biggest", "largest", "huge"],
    &["image", "img"],
    &["repository", "repo"],
    &["environment", "env"],
    &["variable", "var"],
    &["process", "program", "app", "application"],
    &["memory", "ram", "mem"],
    &["config", "configuration", "settings"],
    &["namespace", "ns"],
    &["deployment", "deploy"],
    &["copy", "cp", "duplicate"],
    &["move", "mv"],
    &[
        "admin",
        "root",
        "administrator",
        "superuser",
        "elevated",
        "sudo",
    ],
    &["undo", "revert", "rollback"],
    &["exit", "quit"],
    &["executable", "runnable"],
    &["from", "out"],
    &["error", "stderr"],
];

static STOPWORD_STEMS: LazyLock<Vec<String>> =
    LazyLock::new(|| STOPWORDS.iter().map(|word| stem(word)).collect());
static SYNONYM_STEMS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    SYNONYMS
        .iter()
        .flat_map(|group| group.iter().map(move |word| (stem(word), stem(group[0]))))
        .collect()
});

const FUZZY_WEIGHT: f32 = 0.5;
// skim awards ~28/char for a perfect match, including its bonuses.
const FUZZY_SCORE_PER_CHAR: f32 = 28.0;
const COMMAND_TEXT_DISCOUNT: f32 = 0.8;
const TAG_MATCH_SCORE: f32 = 0.5;
// Pooled matching finds candidates; phrase agreement picks between them.
const POOLED_SHARE: f32 = 0.5;

// Crude but symmetric: both sides go through it, so "deleting", "deleted" and "delete" all become "delet".
fn stem(word: &str) -> String {
    let mut stem = word.to_lowercase();
    if stem.len() >= 5 && stem.ends_with("ies") {
        stem.truncate(stem.len() - 3);
        stem.push('y');
        return stem;
    }
    for suffix in ["ing", "ed"] {
        if stem.len() >= suffix.len() + 3 && stem.ends_with(suffix) {
            stem.truncate(stem.len() - suffix.len());
            break;
        }
    }
    if stem.len() >= 4 && stem.ends_with('s') && !stem.ends_with("ss") {
        stem.pop();
    }
    if stem.len() >= 4 && stem.ends_with('e') {
        stem.pop();
    }
    let bytes = stem.as_bytes();
    if stem.len() >= 4
        && bytes[bytes.len() - 1] == bytes[bytes.len() - 2]
        && !b"aeiou".contains(&bytes[bytes.len() - 1])
    {
        stem.pop();
    }
    stem
}

pub fn normalize(word: &str) -> Option<String> {
    let stemmed = stem(word);
    let canonical = SYNONYM_STEMS.get(&stemmed).cloned().unwrap_or(stemmed);
    (!canonical.is_empty() && !STOPWORD_STEMS.contains(&canonical)).then_some(canonical)
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(normalize)
        .collect();
    tokens.dedup();
    tokens
}

pub struct IndexedCommand {
    pub command: Command,
    phrases: Vec<Vec<String>>,
    intent_words: Vec<String>,
    command_words: Vec<String>,
    tags: Vec<String>,
    intent_text: String,
}

impl IndexedCommand {
    pub fn new(command: Command) -> Self {
        let phrases = std::iter::once(&command.trigger)
            .chain(&command.aliases)
            .map(|phrase| tokenize(phrase))
            .collect();
        let intent_text = command.intent_text().to_lowercase();
        Self {
            intent_words: tokenize(&intent_text),
            command_words: tokenize(&command.command),
            tags: command
                .context_tags
                .iter()
                .filter_map(|tag| normalize(tag))
                .collect(),
            phrases,
            intent_text,
            command,
        }
    }

    pub fn has_tag(&self, token: &str) -> bool {
        self.tags.iter().any(|tag| tag == token)
    }
}

pub struct SearchEngine {
    matcher: SkimMatcherV2,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn match_score(&self, indexed: &IndexedCommand, tokens: &[String]) -> Option<f32> {
        if tokens.is_empty() {
            return Some(0.0);
        }
        let pooled = tokens
            .iter()
            .map(|token| self.token_score(indexed, token))
            .sum::<f32>()
            / tokens.len() as f32;
        if pooled <= 0.0 {
            return None;
        }
        let phrase = indexed
            .phrases
            .iter()
            .map(|words| phrase_agreement(tokens, words))
            .fold(0.0, f32::max);
        Some(POOLED_SHARE * pooled + (1.0 - POOLED_SHARE) * phrase)
    }

    fn token_score(&self, indexed: &IndexedCommand, token: &str) -> f32 {
        let from_intent = best_word_score(token, &indexed.intent_words)
            .or_else(|| self.fuzzy_score(token, &indexed.intent_text));
        let from_command = best_word_score(token, &indexed.command_words)
            .map(|score| score * COMMAND_TEXT_DISCOUNT);
        let from_tags = indexed.has_tag(token).then_some(TAG_MATCH_SCORE);
        [from_intent, from_command, from_tags]
            .into_iter()
            .flatten()
            .fold(0.0, f32::max)
    }

    fn fuzzy_score(&self, token: &str, text: &str) -> Option<f32> {
        let length = token.chars().count();
        if length < 4 {
            return None;
        }
        let raw = self.matcher.fuzzy_match(text, token)?;
        Some((raw as f32 / (length as f32 * FUZZY_SCORE_PER_CHAR)).clamp(0.0, 1.0) * FUZZY_WEIGHT)
    }
}

// F1 of "how much of the query this phrase covers" and "how much of the phrase the query covers".
fn phrase_agreement(tokens: &[String], words: &[String]) -> f32 {
    if words.is_empty() {
        return 0.0;
    }
    let query_coverage = tokens
        .iter()
        .filter_map(|token| best_word_score(token, words))
        .sum::<f32>()
        / tokens.len() as f32;
    let phrase_coverage = words
        .iter()
        .filter_map(|word| {
            tokens
                .iter()
                .filter_map(|token| word_score(token, word))
                .reduce(f32::max)
        })
        .sum::<f32>()
        / words.len() as f32;
    let total = query_coverage + phrase_coverage;
    if total == 0.0 {
        0.0
    } else {
        2.0 * query_coverage * phrase_coverage / total
    }
}

fn best_word_score(token: &str, words: &[String]) -> Option<f32> {
    words
        .iter()
        .filter_map(|word| word_score(token, word))
        .reduce(f32::max)
}

fn word_score(token: &str, word: &str) -> Option<f32> {
    if word == token {
        return Some(1.0);
    }
    if token.len() >= 3 && word.starts_with(token) {
        return Some(0.9);
    }
    if word.len() >= 4 && token.starts_with(word) {
        return Some(0.8);
    }
    // Short tokens ("ip", "ls") would match inside unrelated words ("wipe", "else").
    (token.len() >= 4 && word.contains(token)).then_some(0.6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_drops_stopwords_and_normalizes() {
        assert_eq!(
            tokenize("How to delete a Directory"),
            ["delet", "directory"]
        );
        assert_eq!(tokenize("remove folders"), tokenize("deleting directories"));
    }

    #[test]
    fn stemming_is_consistent_across_inflections() {
        for group in [
            ["change", "changes", "changed"],
            ["stop", "stopped", "stopping"],
            ["file", "files", "filed"],
        ] {
            assert!(
                group.iter().all(|word| stem(word) == stem(group[0])),
                "{group:?}"
            );
        }
    }

    #[test]
    fn short_tokens_do_not_match_inside_words() {
        assert_eq!(word_score("ip", "wipe"), None);
        assert_eq!(word_score("delet", "delet"), Some(1.0));
    }
}
