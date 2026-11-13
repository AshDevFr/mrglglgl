//! The optional first pass before a day's entry: the model reads the day's notes, numbered, and
//! picks the two to four moments that make the day, in the order to tell them, with one line on
//! what ties them together. The entry is then written with those moments as the heart of the day.
//!
//! Choosing is the step small models do worst while also writing; asked alone, with an answer
//! that can be checked (the numbers must name notes, the line may name nothing the notes do not),
//! it is a small task. Anything that does not parse is no plan, and the entry is written as
//! without one.

use crate::factcheck::{self, FactSet};
use crate::prompt;

/// The most moments a plan keeps.
pub const MOMENTS_MAX: usize = 4;
/// Room for the answer, and for a reasoning model's thinking.
pub const PLAN_TOKENS: u32 = 400;
/// A thread longer than this is not one line.
const ARC_WORDS: usize = 30;

/// The moments chosen, as indexes into the day's notes in the order to tell them, and the
/// thread between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub moments: Vec<usize>,
    pub arc: Option<String>,
}

/// The planning request: the system part and the user part.
pub fn request(notes: &[String], language: &str) -> (String, String) {
    let system = format!(
        "You plan an entry in a World of Warcraft adventurer's journal. The notes list what \
         happened in one day, numbered. Choose the two to four notes that make the day a story, \
         in the order to tell them; the rest will be mentioned in passing at most. Prefer what \
         changed the adventurer (a death, a new level, a new place, a rare find, a hard fight) \
         over routine. Then write one line, in {language}, on what ties the chosen moments \
         together, naming only what the notes name. Answer exactly in this form and nothing \
         else:\nMoments: 3, 1, 5\nArc: <one line>",
        language = prompt::language_name(language),
    );
    let user = notes
        .iter()
        .enumerate()
        .map(|(i, n)| format!("{}. {n}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    (system, user)
}

/// The plan in `answer`, for a day of `notes` notes; `None` when no moment names a note.
pub fn parse(answer: &str, notes: usize) -> Option<Plan> {
    let answer = crate::writer::clean(answer);
    let field = |name: &str| {
        answer.lines().find_map(|line| {
            let line = line.trim().trim_start_matches(['*', '-', ' ']);
            let (key, value) = line.split_once(':')?;
            (key.trim_matches(['*', ' ']).eq_ignore_ascii_case(name))
                .then(|| value.trim().trim_matches('*').trim().to_string())
        })
    };
    let mut moments: Vec<usize> = Vec::new();
    for part in field("moments")?.split([',', ' ']) {
        let Ok(n) = part.trim().trim_end_matches('.').parse::<usize>() else {
            continue;
        };
        if (1..=notes).contains(&n) && !moments.contains(&(n - 1)) {
            moments.push(n - 1);
        }
    }
    moments.truncate(MOMENTS_MAX);
    if moments.is_empty() {
        return None;
    }
    let arc = field("arc")
        .map(|a| a.trim_matches(['"', '“', '”']).to_string())
        .filter(|a| !a.is_empty() && a.split_whitespace().count() <= ARC_WORDS);
    Some(Plan { moments, arc })
}

/// `plan` with its thread dropped when the thread names what `facts` (the day's notes) do not.
pub fn checked(mut plan: Plan, facts: &str, allowed: &[&str]) -> Plan {
    if let Some(arc) = &plan.arc {
        let claims = factcheck::check(arc, &FactSet::from_notes(facts, allowed));
        if !claims.is_empty() {
            tracing::debug!(
                "plan's thread left out, it names {}",
                factcheck::reason(&claims)
            );
            plan.arc = None;
        }
    }
    plan
}
