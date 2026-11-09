//! Writes one entry: generate, check it against the notes, rewrite once if it invents things,
//! and give up on prose (the day then shows its plain facts) if the rewrite still does.

use std::time::Duration;

use mrglglgl_llm::{
    CancellationToken, GenerateOutcome, GenerateRequest, Provider, ProviderError, generate_text,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::factcheck::{self, FactSet};
use crate::prompt::{self, Prompt};

/// What the person watching an entry being written sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Chunk(String),
    /// The text so far was set aside and writing starts again (the fact check asked for a
    /// rewrite).
    Restart,
}

/// How an entry fared against the fact check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Passed,
    /// The first draft failed and the rewrite passed.
    Rewritten,
    /// The rewrite failed too; the day shows its plain facts.
    Failed,
    /// Written or changed by the player by hand: the fact check does not vouch for it.
    Edited,
}

impl CheckStatus {
    pub fn key(self) -> &'static str {
        match self {
            CheckStatus::Passed => "passed",
            CheckStatus::Rewritten => "rewritten",
            CheckStatus::Failed => "failed",
            CheckStatus::Edited => "edited",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [Self::Passed, Self::Rewritten, Self::Failed, Self::Edited]
            .into_iter()
            .find(|s| s.key() == key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteResult {
    Written {
        /// The entry, with real names back in place of any stand-ins.
        text: String,
        check: CheckStatus,
        /// Why the entry was set aside, when it was.
        reason: Option<String>,
    },
    Cancelled,
    TimedOut,
}

/// How to ask the provider.
#[derive(Debug, Clone)]
pub struct WriteOptions {
    pub language: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: Duration,
    /// Words the entry may use besides the notes, such as the character's faction.
    pub allowed: Vec<String>,
}

/// Strips what small models add around an entry: reasoning blocks and surrounding blank space.
fn clean(text: &str) -> String {
    let mut out = text.to_string();
    while let (Some(start), Some(end)) = (out.find("<think>"), out.find("</think>")) {
        if end < start {
            break;
        }
        out.replace_range(start..end + "</think>".len(), "");
    }
    // Small models sometimes copy the worked example's layout: a "Notes:" paragraph restating
    // the notes, then an "Entry:" label. Only what follows the label is the entry.
    let is_label = |line: &str, word: &str| {
        let l = line.trim().to_lowercase();
        l.strip_prefix(word)
            .is_some_and(|rest| rest.trim_start().starts_with(':'))
    };
    let lines: Vec<&str> = out.lines().collect();
    if let Some(at) = lines
        .iter()
        .position(|l| is_label(l, "entry") || is_label(l, "entrée"))
    {
        let label = lines[at];
        let after = label.split_once(':').map_or("", |(_, rest)| rest);
        out = std::iter::once(after)
            .chain(lines[at + 1..].iter().copied())
            .collect::<Vec<_>>()
            .join("\n");
    } else if lines.first().is_some_and(|l| is_label(l, "notes")) {
        let rest = out.split_once("\n\n").map_or("", |(_, rest)| rest);
        out = rest.to_string();
    }
    // Asked to rewrite, a small model may copy the request after the entry: nothing from the
    // first such phrase on is the entry.
    if let Some(at) = ECHOED_INSTRUCTIONS
        .iter()
        .filter_map(|phrase| out.find(phrase))
        .min()
    {
        out.truncate(at);
    }
    drop_repeated_sentences(out.trim())
}

/// How the rewrite request begins its parts, as `prompt::rewrite_prompt` words them.
const ECHOED_INSTRUCTIONS: &[&str] = &[
    "A first draft was written:",
    "It mentions things that are not in the notes",
    "Rewrite the entry so that every event",
];

/// Sentences shorter than this may repeat: "Mrgl!" said twice is a voice, not a slip.
const REPEAT_MIN_WORDS: usize = 4;

/// Removes later copies of a sentence already said, word for word, which small models produce
/// as a refrain. Only a line that loses a sentence is rebuilt: paragraphs, line breaks and the
/// rest of the text stay as written.
fn drop_repeated_sentences(text: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut lines = Vec::new();
    for line in text.split('\n') {
        let all = sentences(line);
        let kept: Vec<&str> = all
            .iter()
            .copied()
            .filter(|sentence| {
                let key = sentence
                    .split_whitespace()
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
                    .join(" ");
                key.split(' ').count() < REPEAT_MIN_WORDS || seen.insert(key)
            })
            .collect();
        if kept.len() == all.len() {
            lines.push(Some(line.to_string()));
        } else if kept.is_empty() {
            // The line was nothing but a repeat: drop it, keeping blank lines between paragraphs.
            lines.push(None);
        } else {
            lines.push(Some(kept.join(" ")));
        }
    }
    let joined = lines.into_iter().flatten().collect::<Vec<_>>().join("\n");
    // A dropped line can leave three line breaks in a row where a paragraph break was.
    let mut tidy = joined;
    while tidy.contains("\n\n\n") {
        tidy = tidy.replace("\n\n\n", "\n\n");
    }
    tidy.trim().to_string()
}

/// A line's sentences, each ending with its own punctuation and trimmed.
fn sentences(paragraph: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = paragraph.char_indices().collect();
    for (i, &(at, c)) in chars.iter().enumerate() {
        let next = chars.get(i + 1).map(|&(_, n)| n);
        if matches!(c, '.' | '!' | '?') && next.is_none_or(char::is_whitespace) {
            let end = at + c.len_utf8();
            let sentence = paragraph[start..end].trim();
            if !sentence.is_empty() {
                out.push(sentence);
            }
            start = end;
        }
    }
    let rest = paragraph[start..].trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

async fn attempt(
    provider: &dyn Provider,
    prompt: &Prompt,
    options: &WriteOptions,
    cancel: &CancellationToken,
    sink: Option<&mpsc::Sender<StreamEvent>>,
) -> Result<Result<String, WriteResult>, ProviderError> {
    let (tx, mut rx) = mpsc::channel::<String>(64);
    let forward = async {
        while let Some(chunk) = rx.recv().await {
            if let Some(sink) = sink {
                let _ = sink.send(StreamEvent::Chunk(chunk)).await;
            }
        }
    };
    let request = GenerateRequest {
        system: prompt.system.clone(),
        user: prompt.user.clone(),
        max_tokens: options.max_tokens,
        temperature: options.temperature,
        language: options.language.clone(),
    };
    let (result, ()) = tokio::join!(
        generate_text(provider, request, options.timeout, cancel.clone(), Some(tx)),
        forward
    );
    let (outcome, text) = result?;
    Ok(match outcome {
        GenerateOutcome::Completed => {
            let cleaned = clean(&text);
            tracing::debug!(
                "model answered {} words, {} after cleaning",
                text.split_whitespace().count(),
                cleaned.split_whitespace().count()
            );
            tracing::trace!("model answer:\n{text}");
            Ok(cleaned)
        }
        GenerateOutcome::Cancelled => Err(WriteResult::Cancelled),
        GenerateOutcome::TimedOut => Err(WriteResult::TimedOut),
    })
}

/// Writes an entry from `prompt`, checking it against the prompt's notes.
pub async fn write(
    provider: &dyn Provider,
    prompt: &Prompt,
    options: &WriteOptions,
    cancel: &CancellationToken,
    sink: Option<&mpsc::Sender<StreamEvent>>,
) -> Result<WriteResult, ProviderError> {
    let allowed: Vec<&str> = options.allowed.iter().map(String::as_str).collect();
    let facts = FactSet::from_notes(&prompt.facts, &allowed);

    let first = match attempt(provider, prompt, options, cancel, sink).await? {
        Ok(text) => text,
        Err(stopped) => return Ok(stopped),
    };
    let claims = factcheck::check(&first, &facts);
    if claims.is_empty() {
        return Ok(WriteResult::Written {
            text: prompt::restore_names(&first, &prompt.aliases),
            check: CheckStatus::Passed,
            reason: None,
        });
    }
    tracing::info!(
        "entry asked again, it got facts wrong: {}",
        factcheck::reason(&claims)
    );

    if let Some(sink) = sink {
        let _ = sink.send(StreamEvent::Restart).await;
    }
    let again = prompt::rewrite_prompt(prompt, &first, &claims);
    let second = match attempt(provider, &again, options, cancel, sink).await? {
        Ok(text) => text,
        Err(stopped) => return Ok(stopped),
    };
    let claims = factcheck::check(&second, &facts);
    if claims.is_empty() {
        return Ok(WriteResult::Written {
            text: prompt::restore_names(&second, &prompt.aliases),
            check: CheckStatus::Rewritten,
            reason: None,
        });
    }
    let reason = factcheck::reason(&claims);
    tracing::warn!("entry set aside: {reason}");
    Ok(WriteResult::Written {
        text: prompt::restore_names(&second, &prompt.aliases),
        check: CheckStatus::Failed,
        reason: Some(reason),
    })
}
