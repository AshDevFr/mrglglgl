import { Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { appInfo, listCharacters } from "@/api/commands";
import { EmptyState } from "@/components/EmptyState";

export function HomePage() {
  const { t } = useTranslation();
  const info = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const empty = characters.isSuccess && characters.data.length === 0;

  return (
    <div className="desk" style={{ padding: "2rem 2.5rem" }}>
      <Stack maw="40rem">
        <h1 className="character-name">{t("app.title")}</h1>
        {characters.isPending && <EmptyState title={t("loading")} />}
        {empty && (
          <EmptyState title={t("home.empty")}>{t("home.emptyHelp")}</EmptyState>
        )}
        {characters.isSuccess && !empty && <Text>{t("home.pick")}</Text>}
        {!empty && (
          <Text size="sm" c="dimmed">
            {t("sync.explain")}
          </Text>
        )}
        {info.data && (
          <Text size="sm" c="dimmed">
            {t("app.version", { version: info.data.version })}
          </Text>
        )}
      </Stack>
    </div>
  );
}
