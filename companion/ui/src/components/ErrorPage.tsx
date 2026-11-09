import { Alert, Button, Code, Group, Stack } from "@mantine/core";
import { useQueryClient } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { problem } from "@/i18n/problems";
import { log } from "@/lib/log";

interface Props {
  error: unknown;
  reset: () => void;
}

/**
 * What a page shows when it breaks, in place of the page: what broke and what to do, a way to
 * try again or go back that needs no restart, and the details for a report. The sidebar stays,
 * since the error is caught inside the window's frame.
 */
export function ErrorPage({ error, reset }: Props) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const message = error instanceof Error ? error.message : String(error);
  const details = error instanceof Error && error.stack ? error.stack : message;
  useEffect(() => {
    log("error", `page crashed at ${window.location.pathname}: ${details}`);
  }, [details]);

  async function again() {
    // A page often breaks on data it read: read everything again before drawing it.
    await queryClient.invalidateQueries();
    reset();
    await router.invalidate();
  }

  async function home() {
    reset();
    await router.navigate({ to: "/" });
  }

  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <Stack gap="md" maw="48rem">
        <Alert color="red" role="alert">
          {problem(t, "errors.page", { message })}
        </Alert>
        <Group gap="xs">
          <Button onClick={() => void again()}>
            {t("errorPage.tryAgain")}
          </Button>
          <Button variant="default" onClick={() => void home()}>
            {t("errorPage.home")}
          </Button>
        </Group>
        <details>
          <summary>{t("errorPage.details")}</summary>
          <Code block>{details}</Code>
        </details>
      </Stack>
    </div>
  );
}
