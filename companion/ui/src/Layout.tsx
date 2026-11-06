import {
  ActionIcon,
  AppShell,
  Group,
  NavLink,
  ScrollArea,
  Text,
  Title,
  UnstyledButton,
  useMantineColorScheme,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Outlet, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import {
  listCharacters,
  onJournalUpdated,
  onScreenshotsUpdated,
  onWriting,
  syncNow,
} from "@/api/commands";
import { CharacterTree } from "@/components/CharacterTree";
import { scheduleRebuild } from "@/lib/siteBuild";
import { applyZoom, savedZoom, stepZoom } from "@/lib/zoom";

export function Layout() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { toggleColorScheme } = useMantineColorScheme();
  const params = useParams({ strict: false });
  const selectedId = params.characterId ? Number(params.characterId) : null;
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });

  // ⌘/Ctrl with +, − and 0 size the interface, as in a browser.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const direction =
        e.key === "=" || e.key === "+"
          ? 1
          : e.key === "-" || e.key === "_"
            ? -1
            : e.key === "0"
              ? 0
              : null;
      if (direction === null) return;
      e.preventDefault();
      void applyZoom(stepZoom(savedZoom(), direction));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // The website follows the journal: a new day's facts, an entry written or accepted.
  useEffect(() => {
    const journals = onJournalUpdated(() => scheduleRebuild());
    const writing = onWriting((event) => {
      if (event.type === "finished") scheduleRebuild();
    });
    return () => {
      journals.then((stop) => stop());
      writing.then((stop) => stop());
    };
  }, []);

  // The game saved a journal or a screenshot: refresh what is on screen.
  useEffect(() => {
    const journals = onJournalUpdated(() => queryClient.invalidateQueries());
    const shots = onScreenshotsUpdated(() =>
      queryClient.invalidateQueries({ queryKey: ["screenshots"] }),
    );
    return () => {
      journals.then((stop) => stop());
      shots.then((stop) => stop());
    };
  }, [queryClient]);

  return (
    <AppShell navbar={{ width: 280, breakpoint: "sm" }} padding={0}>
      <a href="#journal" className="skip-link">
        {t("nav.skip")}
      </a>
      <AppShell.Navbar p="sm">
        <AppShell.Section>
          <Group justify="space-between" mb="sm">
            <UnstyledButton onClick={() => navigate({ to: "/" })}>
              <Title order={3} className="reading">
                {t("app.title")}
              </Title>
            </UnstyledButton>
            <ActionIcon
              variant="subtle"
              onClick={toggleColorScheme}
              aria-label={t("nav.theme")}
            >
              ◐
            </ActionIcon>
          </Group>
          <Text size="sm" c="dimmed" fw={600} mb={4}>
            {t("nav.characters")}
          </Text>
        </AppShell.Section>
        <AppShell.Section grow component={ScrollArea}>
          {characters.data && (
            <CharacterTree
              characters={characters.data}
              selectedId={selectedId}
              onSelect={(id) =>
                navigate({
                  to: "/characters/$characterId",
                  params: { characterId: String(id) },
                })
              }
            />
          )}
        </AppShell.Section>
        <AppShell.Section>
          <NavLink
            component="button"
            label={t("nav.syncNow")}
            onClick={async () => {
              await syncNow();
              await queryClient.invalidateQueries();
            }}
          />
          <NavLink
            component="button"
            label={t("nav.settings")}
            onClick={() => navigate({ to: "/settings" })}
          />
        </AppShell.Section>
      </AppShell.Navbar>
      <AppShell.Main h="100vh" id="journal" tabIndex={-1}>
        <Outlet />
      </AppShell.Main>
    </AppShell>
  );
}
