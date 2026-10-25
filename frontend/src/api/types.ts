export type LoaderKind = 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge';

export const LOADER_NAMES: Record<LoaderKind, string> = {
  vanilla: '原版',
  fabric: 'Fabric',
  quilt: 'Quilt',
  forge: 'Forge',
  neoforge: 'NeoForge',
};

export interface LoaderVersion {
  id: string;
  version: string;
  stable: boolean;
  recommended: boolean;
}

export interface Instance {
  id: string;
  name: string;
  versionId: string;
  loader: LoaderKind;
  loaderVersion?: string | null;
  profileId?: string | null;
  javaPath?: string | null;
  memoryMb?: number | null;
  jvmArgs?: string | null;
  gameArgs?: string | null;
  icon?: string | null;
  installed: boolean;
  lastPlayed?: number | null;
  createdAt: number;
  playSeconds?: number;
  launches?: number;
  running?: boolean;
}

export type InstanceInput = Pick<Instance, 'name' | 'javaPath' | 'memoryMb' | 'jvmArgs' | 'gameArgs' | 'icon'> & {
  versionId?: string;
  loader?: LoaderKind;
  loaderVersion?: string | null;
};

export interface Account {
  id: string;
  kind: 'microsoft' | 'offline';
  username: string;
  uuid: string;
  expiresAt?: number | null;
  skinUrl?: string | null;
  isDefault: boolean;
  createdAt: number;
}

export interface JavaInstall {
  path: string;
  version: string;
  major: number;
}

export interface SystemInfo {
  backendVersion: string;
  coreVersion: string;
  os: { os: string; arch: string };
  memory: { totalMb: number; availableMb: number };
  recommendedMemoryMb: number;
  javas: JavaInstall[];
  sidecarJava: string;
  dataDir: string;
  minecraftDir: string;
  instancesDir: string;
  running: string[];
}

export interface VersionEntry {
  id: string;
  type: 'release' | 'snapshot' | 'old_beta' | 'old_alpha';
  url: string;
  releaseTime: string;
}

export interface VersionList {
  latest: { release: string; snapshot: string };
  versions: VersionEntry[];
}

export interface BatchStatus {
  id: string;
  label: string;
  state: 'running' | 'done' | 'failed' | 'cancelled' | string;
  total: number;
  completed: number;
  failed: number;
  bytesDone: number;
  bytesTotal: number;
  errors: string[];
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  path: string;
  enabled: boolean;
  error?: string | null;
}

export type Settings = {
  mirror?: 'bmclapi' | 'official';
  downloadConcurrency?: number;
  javaPath?: string;
  memoryMb?: number | null;
  windowWidth?: number;
  windowHeight?: number;
  msClientId?: string;
  [key: string]: unknown;
};

export interface DeviceCode {
  userCode: string;
  verificationUri: string;
  expiresIn: number;
  message?: string;
}

export interface ServerEvent {
  kind: string;
  data: any;
}
