import { Container, Stack, Text, Title } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { appInfo } from "@/api/commands";

export function HomePage() {
  const { t } = useTranslation();
  const info = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });

  return (
    <Container py="xl">
      <Stack>
        <Title order={1}>{t("app.title")}</Title>
        <Text>{t("home.empty")}</Text>
        {info.data && (
          <Text size="sm" c="dimmed">
            {t("app.version", { version: info.data.version })}
          </Text>
        )}
      </Stack>
    </Container>
  );
}
