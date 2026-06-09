import { unwrapCommand } from "./client";
import type { AuthPersistState } from "@/types/hub/auth";
import type { JobListParams } from "@/types/hub/job";
import type { AuthDirectDeviceData } from "@/types/hub/printer";
import type { ServerData } from "@/types/hub/server";
import type { StartupState } from "@/types/hub/startup";

/** 读取客户端本地启动态：隐私协议、登录用户、服务器、产品类型和系统能力状态。 */
export function getStartupState() {
  return unwrapCommand<StartupState>("get_startup_state");
}

/** 标记隐私协议已同意。 */
export function savePolicyAgreed() {
  return unwrapCommand<boolean>("save_policy_agreed");
}

/** 保存登录成功后的用户、服务器和产品类型。 */
export function saveAuthState(state: AuthPersistState) {
  return unwrapCommand<StartupState>("save_auth_state", { state });
}

/** 清空登录态并保留非认证类设置。 */
export function clearAuthState() {
  return unwrapCommand<StartupState>("clear_auth_state");
}

/** 保存 iframe / Web 登录后推送的 token。 */
export function saveAuthToken(token: string) {
  return unwrapCommand<StartupState>("save_auth_token", { token });
}

/** 保存最近一次使用的 CPMS 服务器信息。 */
export function saveServerInfo(server: ServerData) {
  return unwrapCommand<ServerData>("save_server_info", { server });
}

/** 保存用户选择的直连打印机。 */
export function saveDirectDevice(device: AuthDirectDeviceData) {
  return unwrapCommand<AuthDirectDeviceData>("save_direct_device", { device });
}

/** 获取 CPMS 作业列表。 */
export function getJobList(params: JobListParams) {
  return unwrapCommand<unknown>("get_job_list", {
    pageNumber: params.pageNumber,
    pageSize: params.pageSize,
    jobType: params.type,
    title: params.title ?? "",
    searchTime: params.searchTime ?? "",
  });
}

/** 获取当前用户可用直连打印设备列表。 */
export function getAvailableDevices() {
  return unwrapCommand<unknown>("get_available_devices");
}

/** 更新服务端选择机器，并在客户端本地持久化。 */
export function selectDirectDevice(device: AuthDirectDeviceData) {
  return unwrapCommand<unknown>("select_direct_device", { device });
}

/** 初始化客户端系统能力，包括打印、USB、Socket 和后台任务。 */
export function systemInit() {
  return unwrapCommand<StartupState>("system_init");
}

/** 销毁客户端系统能力，用于退出或关闭前释放资源。 */
export function systemDestroy() {
  return unwrapCommand<boolean>("system_destroy");
}

/** 启动后台任务。 */
export function startBackgroundTasks() {
  return unwrapCommand<boolean>("start_background_tasks");
}

/** 停止后台任务。 */
export function stopBackgroundTasks() {
  return unwrapCommand<boolean>("stop_background_tasks");
}
