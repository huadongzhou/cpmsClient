import type { ServerData } from "./server";

export interface UserData {
  userId?: string;
  userName?: string;
  userAccount?: string;
  token?: string;
  refreshToken?: string;
  expireTime?: string;
}

export interface AuthPersistState {
  user: UserData;
  server: ServerData;
  productType: number;
  systemInitData?: unknown;
}
