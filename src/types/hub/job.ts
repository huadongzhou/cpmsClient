export interface JobProgress {
  jobId: string;
  stage: string;
  message?: string;
  percent?: number;
}

export interface JobListParams {
  pageNumber: number;
  pageSize: number;
  type: 1 | 2 | 3 | number;
  title?: string;
  searchTime?: "now" | "history" | "" | string;
}
