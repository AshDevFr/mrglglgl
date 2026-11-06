// Turning game names into phrases. The game gives names in the client's language with no
// plural or article, so English gets its own rules; other languages fall back to a neutral
// "6 × name" form until a translator adds theirs here.

const UNCHANGED = /(fish|sheep|deer)$/i;

/** The English plural of a creature or item name ("Prairie Wolf" to "Prairie Wolves"). */
export function pluralEn(name: string): string {
  if (UNCHANGED.test(name)) return name;
  if (/man$/.test(name)) return `${name.slice(0, -3)}men`;
  if (/[^aeiou]y$/i.test(name)) return `${name.slice(0, -1)}ies`;
  if (/(s|x|z|ch|sh)$/i.test(name)) return `${name}es`;
  if (/fe$/i.test(name)) return `${name.slice(0, -2)}ves`;
  if (/(ol|al|ar)f$/i.test(name)) return `${name.slice(0, -1)}ves`;
  return `${name}s`;
}

/** "a" or "an" before an English word, by its first letter. */
export function articleEn(word: string): string {
  return /^[aeiou]/i.test(word) ? "an" : "a";
}

/** One name with a count: "a Prairie Wolf", "23 Prairie Wolves". */
export function countedName(
  language: string,
  name: string,
  count: number,
): string {
  if (language === "en") {
    return count === 1
      ? `${articleEn(name)} ${name}`
      : `${count} ${pluralEn(name)}`;
  }
  return count === 1 ? name : `${count} × ${name}`;
}

/** "a, b and c" in the reader's language. */
export function joinList(language: string, items: string[]): string {
  return new Intl.ListFormat(language, {
    style: "long",
    type: "conjunction",
  }).format(items);
}
