import {
  Alert,
  Button,
  Group,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  copySiteTheme,
  getSettings,
  openSite,
  saveSiteSettings,
  showSiteFolder,
} from "@/api/commands";
import type { SiteSettings } from "@/bindings/SiteSettings";
import { problem } from "@/i18n/problems";
import { formatTime } from "@/lib/format";
import {
  canBuild,
  rebuildSite,
  type SiteBuild,
  useLastBuild,
} from "@/lib/siteBuild";
import { FolderField } from "./FolderField";

/** The journal website: where it is written, whose journals it shows, how it looks, and its
 * last build. */
export function WebsitePanel({ site }: { site: SiteSettings }) {
  const { t, i18n } = useTranslation();
  const [draft, setDraft] = useState<SiteSettings>(site);
  const [saved, setSaved] = useState(false);
  const [building, setBuilding] = useState(false);
  const [copied, setCopied] = useState<number | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [mine, setMine] = useState<SiteBuild | null>(null);
  const stored = useLastBuild();
  // The newer of this panel's own build and one the app ran by itself.
  const last = mine && (!stored || mine.at >= stored.at) ? mine : stored;
  const set = (patch: Partial<SiteSettings>) => {
    setDraft((d) => ({ ...d, ...patch }));
    setSaved(false);
  };
  const path = (text: string) => (text.trim() === "" ? null : text.trim());

  // The characters on the site are chosen under Settings, Characters: this tab saves its own
  // fields and keeps that list as stored, so it never undoes a choice made there.
  async function save(): Promise<SiteSettings> {
    setFailure(null);
    try {
      const site = {
        ...draft,
        characters: (await getSettings()).settings.site.characters,
      };
      await saveSiteSettings(site);
      setDraft(site);
      setSaved(true);
      return site;
    } catch (e) {
      setFailure(String(e));
      return draft;
    }
  }

  async function buildNow() {
    setBuilding(true);
    setMine(await rebuildSite(await save()));
    setBuilding(false);
  }

  async function copyTheme() {
    if (!draft.themeFolder) return;
    setFailure(null);
    try {
      setCopied((await copySiteTheme(draft.themeFolder)).length);
    } catch (e) {
      setFailure(String(e));
    }
  }

  return (
    <Stack>
      <Title order={2} size="h3">
        {t("site.title")}
      </Title>
      <Text size="sm" c="dimmed">
        {t("site.help")}
      </Text>
      <Switch
        label={t("site.enabled")}
        description={t("site.enabledHelp")}
        checked={draft.enabled}
        onChange={(e) => set({ enabled: e.currentTarget.checked })}
      />
      <FolderField
        label={t("site.folder")}
        description={t("site.folderHelp")}
        value={draft.folder ?? ""}
        onChange={(v) => set({ folder: path(v) })}
      />
      <TextInput
        label={t("site.siteTitle")}
        placeholder={t("site.defaultTitle")}
        value={draft.title}
        onChange={(e) => set({ title: e.currentTarget.value })}
      />
      <TextInput
        label={t("site.address")}
        description={t("site.addressHelp")}
        placeholder="https://my-journal.pages.dev/"
        value={draft.baseUrl ?? ""}
        onChange={(e) => set({ baseUrl: path(e.currentTarget.value) })}
      />
      <Text size="sm">
        {t("site.charactersMoved", {
          count: draft.characters.length,
        })}
      </Text>
      <Switch
        label={t("site.standInNames")}
        description={t("site.standInNamesHelp")}
        checked={draft.standInNames}
        onChange={(e) => set({ standInNames: e.currentTarget.checked })}
      />
      <Switch
        label={t("site.hideRealm")}
        checked={draft.hideRealm}
        onChange={(e) => set({ hideRealm: e.currentTarget.checked })}
      />
      <FolderField
        label={t("site.themeFolder")}
        description={t("site.themeFolderHelp")}
        value={draft.themeFolder ?? ""}
        onChange={(v) => set({ themeFolder: path(v) })}
      />
      <Group gap="xs">
        <Button
          variant="default"
          disabled={!draft.themeFolder}
          onClick={() => void copyTheme()}
        >
          {t("site.copyTheme")}
        </Button>
        {/* A disabled button says what it is waiting for. */}
        {!draft.themeFolder && (
          <Text size="xs" c="dimmed">
            {t("site.copyThemeNeedsFolder")}
          </Text>
        )}
      </Group>
      {copied !== null && (
        <Alert color="green">{t("site.themeCopied", { count: copied })}</Alert>
      )}
      {failure && (
        <Alert color="red">
          {problem(t, "site.buildFailed", { message: failure })}
        </Alert>
      )}
      <Group>
        <Button onClick={() => void save()}>{t("site.save")}</Button>
        <Button
          variant="light"
          loading={building}
          disabled={!canBuild(draft)}
          onClick={() => void buildNow()}
        >
          {t("site.buildNow")}
        </Button>
        {saved && <Text size="sm">{t("site.saved")}</Text>}
      </Group>
      {/* With the website kept up to date, nothing being built is a warning; otherwise it only
          says what Build now waits for. */}
      {!canBuild(draft) &&
        (draft.enabled ? (
          <Alert color="yellow">{t("site.cannotBuild")}</Alert>
        ) : (
          <Text size="xs" c="dimmed">
            {t("site.cannotBuild")}
          </Text>
        ))}
      {last?.report && (
        <Stack gap="xs">
          <Text size="sm" role="status">
            {t("site.built", {
              when: formatTime(Math.floor(last.at / 1000), i18n.language),
              written: last.report.written,
              unchanged: last.report.unchanged,
              removed: last.report.removed,
            })}
          </Text>
          <Group gap="xs">
            <Button size="xs" variant="light" onClick={() => void openSite()}>
              {t("site.open")}
            </Button>
            <Button
              size="xs"
              variant="default"
              onClick={() => void showSiteFolder()}
            >
              {t("site.showFolder")}
            </Button>
          </Group>
        </Stack>
      )}
      {last?.error && (
        <Alert color="red">
          {problem(t, "site.buildFailed", { message: last.error })}
        </Alert>
      )}
    </Stack>
  );
}
