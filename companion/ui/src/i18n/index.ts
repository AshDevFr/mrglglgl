import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

// Every user-facing string lives in a locale catalogue: English and French. A
// missing key or language falls back to English, so a new catalogue can be
// added without touching components.
export const resources = {
  en: { translation: en },
  fr: { translation: fr },
} as const;

function baseLanguage(language: string): string {
  return language.split("-")[0];
}

export function initI18n(language: string = navigator.language) {
  return i18n.use(initReactI18next).init({
    resources,
    lng: baseLanguage(language),
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });
}

/** Switches to the player's chosen language, or back to the system's when none is chosen. */
export function applyLanguage(language: string | null) {
  return i18n.changeLanguage(baseLanguage(language ?? navigator.language));
}

export default i18n;
