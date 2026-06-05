export interface JobProgress {
  jobId: string;
  stage: string;
  message?: string;
  percent?: number;
}
