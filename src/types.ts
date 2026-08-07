export interface Artifact {
  id: string;
  title: string;
  source_url: string;
  text: string;
  html: string;
  hash: string;
  created_at: string;
  tags: string[];
}

export type RepoProvider = "git" | "gitee";

// 注意：token 永远不会出现在该类型里（凭据隔离）
export interface RepoConfig {
  id: string;
  provider: RepoProvider;
  name: string;
  remote_url: string;
  branch: string;
  username: string;
}

export interface SyncPreview {
  job_id: string;
  repo_id: string;
  repo_name: string;
  artifact_count: number;
  artifact_titles: string[];
  remote_url: string;
}

export type SyncStatus =
  | "pending"
  | "confirmed"
  | "running"
  | "success"
  | "failed";

export interface SyncJob {
  id: string;
  repo_id: string;
  status: SyncStatus;
  artifact_ids: string[];
  created_at: string;
  finished_at?: string;
  error?: string;
}

export interface AuditEntry {
  at: string;
  action: string;
  detail: string;
}

// 工作区目录树
export interface WorkspaceTree {
  nodes: DomainNode[];
}
export interface DomainNode {
  host: string;
  items: DomainItem[];
}
export interface DomainItem {
  id: string;
  title: string;
  created_at: string;
  tags: string[];
}

// 本地文件浏览器
export interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}
