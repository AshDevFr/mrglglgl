import { Container, Stack, Text, Title } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { appInfo, listCharacters } from "@/api/commands";

export function HomePage() {
  const { t } = useTranslation();
  const info = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const empty = characters.isSuccess && characters.data.length === 0;

  return (
    <Container py="xl" className="reading">
      <Stack>
        <Title order={1}>{t("app.title")}</Title>
        {characters.isPending && <Text>{t("loading")}</Text>}
        {empty && (
          <>
            <Text>{t("home.empty")}</Text>
            <Text>{t("home.emptyHelp")}</Text>
          </>
        )}
        {characters.isSuccess && !empty && <Text>{t("home.pick")}</Text>}
        <Text size="sm" c="dimmed">
          {t("sync.explain")}
        </Text>
        {info.data && (
          <Text size="sm" c="dimmed">
            {t("app.version", { version: info.data.version })}
          </Text>
        )}
      </Stack>
    </Container>
  );
}
