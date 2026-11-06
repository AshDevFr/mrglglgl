//! A mechanical check of a written entry against the notes it was written from.
//!
//! Small models invent facts. This catches the worst of it: numbers and capitalised proper names
//! that appear nowhere in the notes the model was given. Feelings, weather and the light are the
//! narrator's to invent and are not checked. The check errs towards letting text through: a
//! capitalised word that opens a sentence is only checked when it starts a run of capitalised
//! words, because ordinary words are capitalised there too.

use std::collections::HashSet;

/// Words always allowed: the narrator's own furniture, days and months, in English and French.
const ALLOWED: &[&str] = &[
    "i",
    "mrgl",
    "mrglglgl",
    "murloc",
    "murlocs",
    "translation",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "lundi",
    "mardi",
    "mercredi",
    "jeudi",
    "vendredi",
    "samedi",
    "dimanche",
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// Words that open a sentence without being part of a name ("Then Prairie Wolves came").
const OPENERS: &[&str] = &[
    "a",
    "an",
    "the",
    "then",
    "after",
    "before",
    "when",
    "while",
    "at",
    "in",
    "on",
    "by",
    "my",
    "our",
    "today",
    "tomorrow",
    "later",
    "so",
    "and",
    "but",
    "even",
    "still",
    "one",
    "two",
    "le",
    "la",
    "les",
    "un",
    "une",
    "des",
    "puis",
    "ensuite",
    "après",
    "avant",
    "quand",
    "au",
    "aux",
    "dans",
    "à",
    "mon",
    "ma",
    "mes",
    "aujourd'hui",
    "demain",
    "et",
    "mais",
];

const UNITS_EN: &[(&str, i64)] = &[
    ("zero", 0),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("sixteen", 16),
    ("seventeen", 17),
    ("eighteen", 18),
    ("nineteen", 19),
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
    ("hundred", 100),
];

const UNITS_FR: &[(&str, i64)] = &[
    ("zéro", 0),
    ("deux", 2),
    ("trois", 3),
    ("quatre", 4),
    ("cinq", 5),
    ("six", 6),
    ("sept", 7),
    ("huit", 8),
    ("neuf", 9),
    ("dix", 10),
    ("onze", 11),
    ("douze", 12),
    ("treize", 13),
    ("quatorze", 14),
    ("quinze", 15),
    ("seize", 16),
    ("vingt", 20),
    ("vingts", 20),
    ("trente", 30),
    ("quarante", 40),
    ("cinquante", 50),
    ("soixante", 60),
    ("cent", 100),
];

/// One is everywhere ("one stubborn swing", "un coup"), so it is never treated as a claim.
const ALWAYS_TRUE: &[i64] = &[0, 1];

fn unit(word: &str) -> Option<i64> {
    UNITS_EN
        .iter()
        .chain(UNITS_FR)
        .find(|(w, _)| *w == word)
        .map(|(_, n)| *n)
}

/// "twenty-three" is 23, "vingt-trois" 23, "quatre-vingt-dix" 90; `None` when not a number.
fn number_word(word: &str) -> Option<i64> {
    let lower = word.to_lowercase();
    let parts: Vec<&str> = lower
        .split('-')
        .filter(|p| !p.is_empty() && *p != "et")
        .collect();
    let values: Vec<i64> = parts.iter().map(|p| unit(p)).collect::<Option<_>>()?;
    let mut total = 0;
    let mut i = 0;
    while i < values.len() {
        // quatre-vingt is four twenties, and cent can be multiplied the same way.
        if i + 1 < values.len() && values[i] < 10 && (values[i + 1] == 20 || values[i + 1] == 100) {
            total += values[i] * values[i + 1];
            i += 2;
        } else {
            total += values[i];
            i += 1;
        }
    }
    Some(total)
}

/// Spellings a word may have in the notes: as written, and without an English or French
/// plural or possessive ending.
fn forms(word: &str) -> Vec<String> {
    let w = word
        .to_lowercase()
        .trim_end_matches("'s")
        .trim_end_matches("’s")
        .to_string();
    let mut out = vec![w.clone()];
    let strip = |suffix: &str, with: &str| {
        w.strip_suffix(suffix)
            .filter(|stem| stem.chars().count() >= 2)
            .map(|stem| format!("{stem}{with}"))
    };
    for form in [
        strip("ies", "y"),
        strip("ves", "f"),
        strip("ves", "fe"),
        strip("men", "man"),
        strip("es", ""),
        strip("s", ""),
        strip("aux", "al"),
        strip("x", ""),
    ]
    .into_iter()
    .flatten()
    {
        out.push(form);
    }
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’' || c == '-'
}

/// Words and numbers of a text, with whether each word opens a sentence.
fn tokens(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut opens = true;
    let mut current = String::new();
    let flush = |current: &mut String, opens: &mut bool, out: &mut Vec<(String, bool)>| {
        let word = current.trim_matches(|c| c == '\'' || c == '’' || c == '-');
        if !word.is_empty() {
            out.push((word.to_string(), *opens));
            *opens = false;
        }
        current.clear();
    };
    for c in text.chars() {
        if is_word_char(c) {
            current.push(c);
            continue;
        }
        flush(&mut current, &mut opens, &mut out);
        if matches!(
            c,
            '.' | '!' | '?' | ':' | '\n' | '"' | '“' | '”' | '«' | '»' | '*' | '('
        ) {
            opens = true;
        }
    }
    flush(&mut current, &mut opens, &mut out);
    out
}

/// What an entry may mention.
#[derive(Debug, Clone, Default)]
pub struct FactSet {
    words: HashSet<String>,
    numbers: HashSet<i64>,
}

impl FactSet {
    /// Everything in `notes` (the user part of the prompt: beats, quests, memory) plus `extra`
    /// words such as the character's faction.
    pub fn from_notes(notes: &str, extra: &[&str]) -> Self {
        let mut facts = Self::default();
        facts.add(notes);
        for word in extra.iter().chain(ALLOWED) {
            facts.add(word);
        }
        facts.numbers.extend(ALWAYS_TRUE);
        facts
    }

    fn add(&mut self, text: &str) {
        for (token, _) in tokens(text) {
            for part in token.split('-') {
                if let Ok(n) = part.parse::<i64>() {
                    self.numbers.insert(n);
                }
                self.words.extend(forms(part));
            }
            self.words.extend(forms(&token));
        }
    }

    fn knows_word(&self, word: &str) -> bool {
        if word.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
        // Murloc-speak gurgles: "Mrrrgl", "Aaaaaughibbrgubugbugrguburgle".
        let lower = word.to_lowercase();
        if lower.starts_with("mrr") || lower.starts_with("mrgl") || lower.starts_with("aaaa") {
            return true;
        }
        forms(word).iter().any(|f| self.words.contains(f))
            || word
                .split('-')
                .all(|part| !part.is_empty() && forms(part).iter().any(|f| self.words.contains(f)))
    }
}

fn capitalised(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase)
}

/// The claims in `entry` that `facts` do not support: names (runs of capitalised words) and
/// numbers, as written, each once, in order.
pub fn check(entry: &str, facts: &FactSet) -> Vec<String> {
    let words = tokens(entry);
    let mut claims: Vec<String> = Vec::new();
    let mut push = |claim: String| {
        if !claims.contains(&claim) {
            claims.push(claim);
        }
    };

    let mut i = 0;
    while i < words.len() {
        let (word, opens) = &words[i];
        if let Some(n) = word.parse::<i64>().ok().or_else(|| number_word(word)) {
            if !facts.numbers.contains(&n) {
                push(word.clone());
            }
            i += 1;
            continue;
        }
        if !capitalised(word) {
            i += 1;
            continue;
        }
        // A run of capitalised words is one name.
        let mut end = i + 1;
        while end < words.len() && capitalised(&words[end].0) && !words[end].1 {
            end += 1;
        }
        let mut run: Vec<&str> = words[i..end].iter().map(|(w, _)| w.as_str()).collect();
        if *opens {
            if run.len() == 1 {
                i = end;
                continue;
            }
            if OPENERS.contains(&run[0].to_lowercase().as_str()) {
                run.remove(0);
            }
        }
        if run.iter().any(|w| !facts.knows_word(w)) {
            push(run.join(" "));
        }
        i = end;
    }
    claims
}

/// The reason stored with a story whose entry could not be made to agree with the notes.
pub fn reason(claims: &[String]) -> String {
    format!("mentions {} not in the day's notes", claims.join(", "))
}
