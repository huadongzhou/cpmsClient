import { isTauri } from "@tauri-apps/api/core";
import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { AppNotification } from "@/types/app/notification";
import {
  type DesktopNotificationAckPayload,
  DESKTOP_NOTIFICATION_ACK_EVENT,
  DESKTOP_NOTIFICATION_PUSH_EVENT,
  DESKTOP_NOTIFICATION_WINDOW,
  ensureNotificationWindow,
  positionNotificationWindow,
} from "@/api/tauri/notification";

export type DesktopNotificationDiagnosticStatus = "ok" | "failed" | "skipped" | "timeout";

export interface DesktopNotificationDiagnosticStep {
  name: string;
  status: DesktopNotificationDiagnosticStatus;
  durationMs: number;
  detail?: string;
}

export interface DesktopNotificationDiagnostic {
  ok: boolean;
  steps: DesktopNotificationDiagnosticStep[];
  ack?: DesktopNotificationAckPayload;
}

interface NotificationAckWaitResult {
  received?: DesktopNotificationAckPayload;
  final?: DesktopNotificationAckPayload;
}

interface NotificationAckWaiter {
  promise: Promise<NotificationAckWaitResult>;
  cancel: () => void;
}

const DEFAULT_ACK_TIMEOUT_MS = 2500;

/** 分步诊断桌面通知链路（运行时检测/调试视图使用）。 */
export async function diagnoseDesktopNotification(
  notification: AppNotification,
  options: { ackTimeoutMs?: number } = {},
): Promise<DesktopNotificationDiagnostic> {
  const steps: DesktopNotificationDiagnosticStep[] = [];
  const ackTimeoutMs = options.ackTimeoutMs ?? DEFAULT_ACK_TIMEOUT_MS;

  const tauriStartedAt = now();
  if (!isTauri()) {
    pushStep(steps, "tauri-runtime", "skipped", tauriStartedAt, "not running inside Tauri");
    return { ok: false, steps };
  }
  pushStep(steps, "tauri-runtime", "ok", tauriStartedAt);

  let notificationWindow: WebviewWindow;
  const windowStartedAt = now();
  try {
    notificationWindow = await ensureNotificationWindow();
    pushStep(steps, "ensure-window", "ok", windowStartedAt);
  } catch (error) {
    pushStep(steps, "ensure-window", "failed", windowStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  const positionStartedAt = now();
  try {
    await positionNotificationWindow(notificationWindow);
    pushStep(steps, "position-window", "ok", positionStartedAt);
  } catch (error) {
    pushStep(steps, "position-window", "failed", positionStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  let ackWaiter: NotificationAckWaiter;
  const ackListenerStartedAt = now();
  try {
    ackWaiter = await createNotificationAckWaiter(notification.id, ackTimeoutMs);
    pushStep(steps, "ack-listener", "ok", ackListenerStartedAt);
  } catch (error) {
    pushStep(steps, "ack-listener", "failed", ackListenerStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  const emitStartedAt = now();
  try {
    await emitTo(DESKTOP_NOTIFICATION_WINDOW, DESKTOP_NOTIFICATION_PUSH_EVENT, notification);
    pushStep(steps, "emit-push-event", "ok", emitStartedAt);
  } catch (error) {
    ackWaiter.cancel();
    pushStep(steps, "emit-push-event", "failed", emitStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  const ackStartedAt = now();
  const ackResult = await ackWaiter.promise;
  if (!ackResult.received) {
    pushStep(
      steps,
      "notification-window-received",
      "timeout",
      ackStartedAt,
      "notification window did not acknowledge the push event",
    );
    pushStep(steps, "notification-window-show", "skipped", ackStartedAt);
    return { ok: false, steps };
  }

  pushStep(steps, "notification-window-received", "ok", ackStartedAt);

  if (ackResult.final?.stage === "shown") {
    pushStep(steps, "notification-window-show", "ok", ackStartedAt);
    return { ok: true, steps, ack: ackResult.final };
  }

  if (ackResult.final?.stage === "show-error") {
    pushStep(
      steps,
      "notification-window-show",
      "failed",
      ackStartedAt,
      ackResult.final.error ?? "window.show() failed",
    );
    return { ok: false, steps, ack: ackResult.final };
  }

  pushStep(
    steps,
    "notification-window-show",
    "timeout",
    ackStartedAt,
    "notification window received the event but did not confirm show()",
  );
  return { ok: false, steps, ack: ackResult.received };
}

async function createNotificationAckWaiter(
  notificationId: string,
  timeoutMs: number,
): Promise<NotificationAckWaiter> {
  let unlisten: UnlistenFn | undefined;
  let received: DesktopNotificationAckPayload | undefined;
  let final: DesktopNotificationAckPayload | undefined;
  let done = false;
  let timer: number | undefined;
  let resolvePromise: (result: NotificationAckWaitResult) => void = () => undefined;
  const promise = new Promise<NotificationAckWaitResult>((resolve) => {
    resolvePromise = resolve;
  });

  function finish() {
    if (done) {
      return;
    }

    done = true;
    if (timer !== undefined) {
      window.clearTimeout(timer);
    }

    unlisten?.();
    resolvePromise({ received, final });
  }

  unlisten = await listen<DesktopNotificationAckPayload>(DESKTOP_NOTIFICATION_ACK_EVENT, (event) => {
    const payload = event.payload;

    if (payload.id !== notificationId) {
      return;
    }

    if (payload.stage === "received") {
      received = payload;
      return;
    }

    final = payload;
    received ??= payload;
    finish();
  });

  if (done) {
    unlisten();
  } else {
    timer = window.setTimeout(() => finish(), timeoutMs);
  }

  return { promise, cancel: finish };
}

function pushStep(
  steps: DesktopNotificationDiagnosticStep[],
  name: string,
  status: DesktopNotificationDiagnosticStatus,
  startedAt: number,
  detail?: string,
) {
  const step: DesktopNotificationDiagnosticStep = {
    name,
    status,
    durationMs: Math.max(0, Math.round(now() - startedAt)),
  };

  if (detail) {
    step.detail = detail;
  }

  steps.push(step);
}

function now() {
  return typeof performance === "undefined" ? Date.now() : performance.now();
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
