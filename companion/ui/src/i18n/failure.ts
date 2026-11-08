import type { TFunction } from "i18next";
import type { Failure } from "@/bindings/Failure";
import { type Problem, problem } from "./problems";

/**
 * A command's error as a failure the window can explain. Commands that report a kind send it as
 * an object; anything else (a plain string, a crash in the bridge) is a failure nobody planned for.
 */
export function asFailure(error: unknown): Failure {
  if (
    typeof error === "object" &&
    error !== null &&
    "kind" in error &&
    typeof error.kind === "string"
  ) {
    return error as Failure;
  }
  return { kind: "other", message: String(error) };
}

/**
 * The failure said inside its context ("The download failed: ..."), followed by the step that
 * fits its kind. A failure nobody planned for keeps the backend's words and the context's step.
 */
export function failureProblem(
  t: TFunction,
  context: Problem,
  error: unknown,
): string {
  const failure = asFailure(error);
  if (failure.kind === "other" || failure.kind === "cancelled") {
    return problem(t, context, {
      message: failure.kind === "other" ? failure.message : failure.kind,
    });
  }
  const values: Record<string, unknown> = { ...failure };
  const message = t(context, { message: t(`failure.${failure.kind}`, values) });
  const stop = /[.!?]$/.test(message) ? "" : ".";
  return `${message}${stop} ${t(`failure.${failure.kind}Next`, values)}`;
}
