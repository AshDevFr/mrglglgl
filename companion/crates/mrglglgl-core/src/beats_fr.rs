//! The day's beats in French, for a narrator writing French prose. Game names (creatures, items,
//! places, quests, factions) stay exactly as the client recorded them; only the wording around
//! them is French. Diary style, no subject, passé composé: "Pêché 12 poissons".

use crate::beats::Beat;

fn et_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} et {last}", rest.join(", ")),
    }
}

/// `items`, then "1 autre …" or "N autres …" when `others` is not zero.
fn with_others(items: &[String], others: usize, one: &str, many: &str) -> Vec<String> {
    let mut list = items.to_vec();
    match others {
        0 => {}
        1 => list.push(format!("1 {one}")),
        n => list.push(format!("{n} {many}")),
    }
    list
}

fn counted(items: &[(String, i64)]) -> Vec<String> {
    items
        .iter()
        .map(|(name, count)| format!("{name} ×{count}"))
        .collect()
}

/// Copper as the French client shows it: "3 po 20 pa 5 pc".
pub fn or(copper: i64) -> String {
    let (g, s, c) = (copper / 10_000, copper / 100 % 100, copper % 100);
    let mut parts = Vec::new();
    if g > 0 {
        parts.push(format!("{g} po"));
    }
    if s > 0 {
        parts.push(format!("{s} pa"));
    }
    if c > 0 || parts.is_empty() {
        parts.push(format!("{c} pc"));
    }
    parts.join(" ")
}

fn money_place(source: &Option<String>, earned: bool) -> Option<&'static str> {
    Some(match (source.as_deref()?, earned) {
        ("loot", _) => "grâce au butin",
        ("quest", true) => "grâce aux quêtes",
        ("quest", false) => "pour les quêtes",
        ("vendor", true) => "en vendant aux marchands",
        ("vendor", false) => "chez les marchands",
        ("repair", _) => "en réparations",
        ("trainer", _) => "chez les maîtres",
        ("travel", _) => "en vols",
        ("mail", _) => "par courrier",
        ("auction", _) => "à l'hôtel des ventes",
        ("trade", _) => "en échanges",
        _ => return None,
    })
}

fn quest_list(titles: &[String], others: i64) -> String {
    let counted_quests = |n: i64, other: bool| match (n, other) {
        (1, true) => "1 autre quête".to_string(),
        (n, true) => format!("{n} autres quêtes"),
        (1, false) => "1 quête".to_string(),
        (n, false) => format!("{n} quêtes"),
    };
    if titles.is_empty() {
        return counted_quests(others, false);
    }
    let quoted: Vec<String> = titles.iter().map(|t| format!("\"{t}\"")).collect();
    let named = if titles.len() == 1 {
        format!("la quête {}", quoted[0])
    } else {
        format!("les quêtes {}", et_list(&quoted))
    };
    if others > 0 {
        format!("{named}, et {}", counted_quests(others, true))
    } else {
        named
    }
}

fn capitalised(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => text,
    }
}

impl Beat {
    /// The beat in French.
    pub fn text_fr(&self) -> String {
        match self {
            Beat::Travelled { zones, others: 0 } if zones.len() == 1 => {
                format!("Passé la journée à {}", zones[0])
            }
            Beat::Travelled { zones, others } => format!(
                "Traversé {}",
                et_list(&with_others(zones, *others, "autre zone", "autres zones"))
            ),
            Beat::FirstVisit { places, others } => format!(
                "Visité pour la première fois : {}",
                et_list(&with_others(places, *others, "autre lieu", "autres lieux"))
            ),
            Beat::Kills { lines, others } => {
                let mut parts: Vec<String> = lines
                    .iter()
                    .map(|(name, count)| format!("{name} ×{count}"))
                    .collect();
                match others {
                    0 => {}
                    1 => parts.push("1 autre créature".to_string()),
                    n => parts.push(format!("{n} autres créatures")),
                }
                format!("Tué {}", et_list(&parts))
            }
            Beat::Notable {
                name,
                classification,
                count,
            } => {
                let kind = match classification.as_str() {
                    "elite" => "élite",
                    "rare" => "rare",
                    "rareelite" => "élite rare",
                    _ => "boss mondial",
                };
                if *count == 1 {
                    format!("Tué {name}, un {kind}")
                } else {
                    format!("Tué {name} ×{count}, {kind}")
                }
            }
            Beat::Fishing { fish, junk, streak } => {
                let mut text = format!("Allé pêcher, {fish} poissons pris");
                if !junk.is_empty() {
                    text = format!("{text}, plus {}", et_list(&counted(junk)));
                }
                if let Some(days) = streak {
                    text = format!("{text} ({days}e jour de pêche d'affilée)");
                }
                text
            }
            Beat::Boot { item, count } if *count == 1 => {
                format!("Pêché {item} et gardé")
            }
            Beat::Boot { item, count } => format!("Pêché {item} ×{count} et tout gardé"),
            Beat::Loot { items } => format!("Ramassé {}", et_list(items)),
            Beat::SeenLoot { items } => format!("Vu tomber {}", et_list(items)),
            Beat::Death {
                killer,
                environment,
                place,
                count,
            } => {
                let how = match (killer, environment.as_deref()) {
                    (_, Some("Drowning")) => "Noyé".to_string(),
                    (_, Some("Falling")) => "Mort d'une chute".to_string(),
                    (_, Some("Fatigue")) => "Mort d'épuisement en eau profonde".to_string(),
                    (_, Some("Fire")) => "Mort dans le feu".to_string(),
                    (_, Some("Lava")) => "Mort dans la lave".to_string(),
                    (_, Some("Slime")) => "Mort dans la vase".to_string(),
                    (_, Some(other)) => format!("Mort ({other})"),
                    (Some(k), None) => format!("Tué par {k}"),
                    (None, None) => "Mort".to_string(),
                };
                let times = if *count > 1 {
                    format!(" ({count} fois)")
                } else {
                    String::new()
                };
                match place {
                    Some(p) => format!("{how} à {p}{times}"),
                    None => format!("{how}{times}"),
                }
            }
            Beat::LevelUp { from: Some(f), to } if to - f >= 2 => {
                format!("Passé du niveau {f} au niveau {to}")
            }
            Beat::LevelUp { to, .. } => format!("Atteint le niveau {to}"),
            Beat::Companions {
                names,
                new,
                minutes,
            } => {
                let parts: Vec<String> = names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| {
                        let mut part = n.clone();
                        if new.contains(n) {
                            part.push_str(" (pour la première fois)");
                        }
                        if let Some(m) = minutes.get(i) {
                            part.push_str(&format!(" pendant {}", super::beats::duration_fr(*m)));
                        }
                        part
                    })
                    .collect();
                format!("En groupe avec {}", et_list(&parts))
            }
            Beat::Bosses { names } => format!("Vaincu {}", et_list(names)),
            Beat::Milestones { names, challenges } => {
                let told = |list: &[String], one: &str, many: &str| {
                    let quoted: Vec<String> = list.iter().map(|n| format!("\"{n}\"")).collect();
                    let kind = if list.len() == 1 { one } else { many };
                    format!("{kind} {}", et_list(&quoted))
                };
                let mut parts = Vec::new();
                if !names.is_empty() {
                    let named: Vec<String> = names
                        .iter()
                        .map(|id| crate::milestones::name_in(id, "fr").to_string())
                        .collect();
                    parts.push(told(&named, "obtenu le jalon", "obtenu les jalons"));
                }
                if !challenges.is_empty() {
                    parts.push(told(
                        challenges,
                        "accompli le défi Legacy",
                        "accompli les défis Legacy",
                    ));
                }
                capitalised(parts.join(" et "))
            }
            Beat::Profession { name, from, to } => format!("{name} est passé de {from} à {to}"),
            Beat::Money {
                earned,
                spent,
                earned_mostly,
                spent_mostly,
            } => {
                let side = |verb: &str, copper: i64, mostly: Option<&str>| {
                    let mut part = format!("{verb} {}", or(copper));
                    if let Some(where_) = mostly {
                        part.push_str(&format!(", surtout {where_}"));
                    }
                    part
                };
                let mut parts = Vec::new();
                if *earned > 0 {
                    parts.push(side("Gagné", *earned, money_place(earned_mostly, true)));
                }
                if *spent > 0 {
                    let verb = if parts.is_empty() {
                        "Dépensé"
                    } else {
                        "dépensé"
                    };
                    parts.push(side(verb, *spent, money_place(spent_mostly, false)));
                }
                parts.join(" ; ")
            }
            Beat::Learned { spells, others } => format!(
                "Appris {}",
                et_list(&with_others(spells, *others, "autre sort", "autres sorts"))
            ),
            Beat::Collected { names } => format!("Ajouté {} à la collection", et_list(names)),
            Beat::Reputation { lines } => {
                let parts: Vec<String> = lines
                    .iter()
                    .map(|(faction, standing)| format!("{standing} auprès de {faction}"))
                    .collect();
                format!("Devenu {}", et_list(&parts))
            }
            Beat::Gear { items, others } => format!(
                "Équipé un meilleur {}",
                et_list(&with_others(items, *others, "autre pièce", "autres pièces"))
            ),
            Beat::Travel {
                bound,
                flight_paths,
            } => {
                let mut parts = Vec::new();
                if let Some(place) = bound {
                    parts.push(format!("lié la pierre de foyer à {place}"));
                }
                if !flight_paths.is_empty() {
                    let kind = if flight_paths.len() == 1 {
                        "le trajet aérien de"
                    } else {
                        "les trajets aériens de"
                    };
                    parts.push(format!("découvert {kind} {}", et_list(flight_paths)));
                }
                capitalised(parts.join(" et "))
            }
            Beat::Duels { won, lost } => {
                let count = |n: i64| {
                    if n == 1 {
                        "un duel".to_string()
                    } else {
                        format!("{n} duels")
                    }
                };
                match (won, lost) {
                    (0, l) => format!("Perdu {}", count(*l)),
                    (w, 0) => format!("Gagné {}", count(*w)),
                    (w, l) => format!("Gagné {} et perdu {}", count(*w), count(*l)),
                }
            }
            Beat::Instances { runs } => {
                let parts: Vec<String> = runs
                    .iter()
                    .map(|(name, minutes)| match minutes {
                        Some(m) => format!("{name} ({m} minutes)"),
                        None => name.clone(),
                    })
                    .collect();
                format!("Exploré {}", et_list(&parts))
            }
            Beat::Farewell => {
                "Mort pour de bon : pour ce personnage hardcore, c'était le dernier jour"
                    .to_string()
            }
            Beat::Spoke { lines } => {
                let told: Vec<String> = lines
                    .iter()
                    .map(|(channel, text)| match channel.as_str() {
                        "emote" => format!("*{text}*"),
                        _ => format!("\"{text}\""),
                    })
                    .collect();
                format!("Dans le rôle : {}", told.join(" "))
            }
            Beat::Auctions { sold, won } => {
                let mut parts = Vec::new();
                if !sold.is_empty() {
                    parts.push(format!("vendu {}", et_list(sold)));
                }
                if !won.is_empty() {
                    parts.push(format!("remporté {}", et_list(won)));
                }
                capitalised(format!("{} à l'hôtel des ventes", parts.join(" et ")))
            }
            Beat::Crafted { items } => format!("Fabriqué {}", et_list(&counted(items))),
            Beat::Gathered { via, items } => match via {
                Some(skill) => format!("Récolté {} ({skill})", et_list(&counted(items))),
                None => format!("Récolté {}", et_list(&counted(items))),
            },
            Beat::QuestsDone { groups, elsewhere } => {
                let mut parts: Vec<String> = groups
                    .iter()
                    .map(|g| match &g.zone {
                        Some(zone) => format!("à {zone}, {}", quest_list(&g.titles, g.others)),
                        None => quest_list(&g.titles, g.others),
                    })
                    .collect();
                match elsewhere {
                    0 => {}
                    1 => parts.push("et 1 autre quête ailleurs".to_string()),
                    n => parts.push(format!("et {n} autres quêtes ailleurs")),
                }
                let text = parts.join(" ; ");
                match text.strip_prefix("à ") {
                    Some(_) => format!("Terminé, {text}"),
                    None => format!("Terminé {text}"),
                }
            }
            Beat::QuestsTaken { titles, others } => {
                format!("Accepté {}", quest_list(titles, *others))
            }
            Beat::BigQuests { quests } => {
                let kind = |k: &str| match k {
                    "elite" => "élite".to_string(),
                    "dungeon" => "donjon".to_string(),
                    "group" => "groupe".to_string(),
                    other => other.to_string(),
                };
                match quests.as_slice() {
                    [(title, k)] => format!("Terminé la quête {} \"{title}\"", kind(k)),
                    _ => {
                        let parts: Vec<String> = quests
                            .iter()
                            .map(|(title, k)| format!("\"{title}\" ({})", kind(k)))
                            .collect();
                        format!("Terminé {}", et_list(&parts))
                    }
                }
            }
            Beat::Practice { skills } => {
                let parts: Vec<String> = skills
                    .iter()
                    .map(|(name, from, to)| format!("{name} de {from} à {to}"))
                    .collect();
                format!("Entraîné {}", et_list(&parts))
            }
        }
    }
}
