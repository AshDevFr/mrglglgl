import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";

// Every user-facing string lives in a locale catalogue. English is the only
// one shipped; a missing key or language falls back to English, so a new
// catalogue (French first) can be added without touching components.
export const resources = { en: { translation: en } } as const;

export function initI18n(language: string = navigator.language) {
  return i18n.use(initReactI18next).init({
    resources,
    lng: language.split("-")[0],
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });
}

export default i18n;
