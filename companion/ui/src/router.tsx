import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  useNavigate,
} from "@tanstack/react-router";
import { ErrorPage } from "@/components/ErrorPage";
import { Layout } from "@/Layout";
import { CharacterPage } from "@/pages/CharacterPage";
import { HomePage } from "@/pages/HomePage";
import { MilestonesPage } from "@/pages/MilestonesPage";
import { NarratorPage } from "@/pages/NarratorPage";
import { PrintPage } from "@/pages/PrintPage";
import { ReviewPage } from "@/pages/ReviewPage";
import { SettingsPage } from "@/pages/SettingsPage";

// Code-based routing keeps the app self-contained (no router codegen plugin). Every page sits
// in the app's frame except the print page, which is the journal alone, as it goes on paper.
const rootRoute = createRootRoute({ component: Outlet });
const appRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "app",
  component: Layout,
});

const printRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/print/$characterId",
  component: function Print() {
    const { characterId } = printRoute.useParams();
    return <PrintPage characterId={Number(characterId)} />;
  },
});

const indexRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/",
  component: function Home() {
    const navigate = useNavigate();
    return <HomePage onOpenSettings={() => navigate({ to: "/settings" })} />;
  },
});

const characterRoute = createRoute({
  getParentRoute: () => appRoute,
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
  getParentRoute: () => appRoute,
  path: "/characters/$characterId/milestones",
  component: function Milestones() {
    const { characterId } = milestonesRoute.useParams();
    return <MilestonesPage characterId={Number(characterId)} />;
  },
});

const reviewRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/characters/$characterId/review",
  component: function Review() {
    const { characterId } = reviewRoute.useParams();
    return <ReviewPage characterId={Number(characterId)} />;
  },
});

const reviewAllRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/review",
  component: () => <ReviewPage />,
});

const narratorRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/narrators/$narratorId",
  component: function Narrator() {
    const { narratorId } = narratorRoute.useParams();
    return <NarratorPage narratorId={narratorId} />;
  },
});

const settingsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/settings",
  component: SettingsPage,
});

const routeTree = rootRoute.addChildren([
  appRoute.addChildren([
    indexRoute,
    characterRoute,
    milestonesRoute,
    reviewRoute,
    reviewAllRoute,
    narratorRoute,
    settingsRoute,
  ]),
  printRoute,
]);

// A page that breaks shows what broke and a way back, inside the frame, instead of the
// router's bare message that only a restart got past.
export const router = createRouter({
  routeTree,
  defaultErrorComponent: ErrorPage,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
