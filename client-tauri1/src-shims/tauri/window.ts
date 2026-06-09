import {
  availableMonitors as tauri1AvailableMonitors,
  currentMonitor as tauri1CurrentMonitor,
  getAll,
  getCurrent,
  primaryMonitor as tauri1PrimaryMonitor,
  type Monitor as Tauri1Monitor,
} from "../../node_modules/@tauri-apps/api/window.js";

type WorkAreaMonitor = Tauri1Monitor & {
  workArea: {
    position: Tauri1Monitor["position"];
    size: Tauri1Monitor["size"];
  };
};

function withWorkArea(monitor: Tauri1Monitor | null): WorkAreaMonitor | null {
  if (!monitor) {
    return null;
  }

  return {
    ...monitor,
    workArea: {
      position: monitor.position,
      size: monitor.size,
    },
  };
}

async function currentMonitor() {
  return withWorkArea(await tauri1CurrentMonitor());
}

async function primaryMonitor() {
  return withWorkArea(await tauri1PrimaryMonitor());
}

async function availableMonitors() {
  return (await tauri1AvailableMonitors()).map((monitor) => withWorkArea(monitor)!);
}

function getCurrentWindow() {
  return getCurrent();
}

export { availableMonitors, currentMonitor, getAll, getCurrent, getCurrentWindow, primaryMonitor };
export type { WorkAreaMonitor as Monitor };
