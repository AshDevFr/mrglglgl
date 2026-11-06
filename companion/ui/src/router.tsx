import {
  createRootRoute,
  createRoute,
  createRouter,
  useNavigate,
} from "@tanstack/react-router";
import { Layout } from "@/Layout";
import { CharacterPage } from "@/pages/CharacterPage";
import { HomePage } from "@/pages/HomePage";
import { MilestonesPage } from "@/pages/MilestonesPage";
import { NarratorPage } from "@/pages/NarratorPage";
import { ReviewPage } from "@/pages/ReviewPage";
import { SettingsPage } from "@/pages/SettingsPage";

// Code-based routing keeps the app self-contained (no router codegen plugin).
const rootRoute = createRootRoute({ component: Layout });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: function Home() {
    const navigate = useNavigate();
    return <HomePage onOpenSettings={() => navigate({ to: "/settings" })} />;
  },
});

const characterRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/characters/$characterId",
  validateSearch: (search: Record<string, unknown>): { day?: string } => ({
    day: typeof search.day === "string" ? search.day : undefined,
  }),
  component: function Character() {
    const { characterId } = characterRoute.useParams();
    const { day } = characterRoute.useSearch();
    return <CharacterPage characterId={Number(characterId)} day={day} />;
  },
});

const milestonesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/characters/$characterId/milestones",
  component: function Milestones() {
    const { characterId } = milestonesRoute.useParams();
    return <MilestonesPage characterId={Number(characterId)} />;
  },
});

const reviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/characters/$characterId/review",
  component: function Review() {
    const { characterId } = reviewRoute.useParams();
    return <ReviewPage characterId={Number(characterId)} />;
  },
});

const narratorRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/narrators/$narratorId",
  component: function Narrator() {
    const { narratorId } = narratorRoute.useParams();
    return <NarratorPage narratorId={narratorId} />;
  },
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  characterRoute,
  milestonesRoute,
  reviewRoute,
  narratorRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
