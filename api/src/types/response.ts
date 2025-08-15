import { Status } from "./queue";
import { ProcessOptions } from "./request";

export interface SuccessResponse<T> {
  success: true;
  message: string;
  data: T;
}

export interface ErrorResponse<RequestData> {
  success: false;
  message: string;
  data?: RequestData;
}

export type CustomResponse<T> = SuccessResponse<T> | ErrorResponse<T>;

export interface UploadResponse {
  id: string;
  filename: string;
  path: string;
  size: number;
}

export interface ProcessResponse {
  id: string;
  file: string;
  message: string;
  options?: ProcessOptions;
  progress: number;
  status?: Status;
  queuedAt?: Date;
  totalEntries?: number;
  estimatedTimeMs?: number;
  estimatedTime?: string;
}

export interface ProgressResponse {
  id: string;
  status: "queued" | "processing" | "completed" | "failed";
  progress: number;
  message?: string;
  totalEntries?: number;
  estimatedTime?: string;
  estimatedTimeMs?: number;
  estimatedTimeRemaining?: string;
  estimatedTimeRemainingMs?: number;
}

export interface DeleteResponse {
  message: string;
}

export interface CsvColumnsResponse {
  headers: string[];
}

// File-related types
export interface UploadedFile {
  filename: string;
  id: string;
  size: number;
  uploadedAt: Date;
  modifiedAt: Date;
}

export interface FileListResponse {
  files: UploadedFile[];
}

// Report-related types
export interface ReportFile {
  filename: string;
  fileId: string;
  size: number;
  createdAt: Date;
  modifiedAt: Date;
}

export interface ReportListResponse {
  reports: ReportFile[];
}

export interface ProcessingStage {
  Starting?: void;
  ParsingCsv?: void;
  GroupingProducts?: void;
  GroupingProductsCompleted?: {
    total_parents: number;
    total_products: number;
  };
  ProcessingProducts?: {
    sku: string;
    current: number;
    total: number;
  };
  ProcessingVariations?: {
    sku: string;
    current: number;
    total: number;
  };
  Finalizing?: void;
  Completed?: void;
  Failed?: string;
}

export interface RustTimestamp {
  secs_since_epoch: number;
  nanos_since_epoch: number;
}

export interface ReportDetailsResponse {
  report: ProcessingProgress;
}

// Standard API response wrapper
export interface ApiResponse<T = any> {
  success: boolean;
  data: T;
  message?: string;
}

// Replace the existing interfaces in your TypeScript file

export interface FailedRowDetail {
  row_number: number;
  reason: string;
}

export interface FailedProductDetail {
  sku: string;
  reason: string;
}

export interface ProcessingProgress {
  file_id: string;
  percent: number;
  stage: ProcessingStage | string;
  total_rows: number;
  processed_rows: number;
  successful_rows: number;
  failed_rows: number;
  failed_row_details: FailedRowDetail[];        // NEW
  failed_product_details: FailedProductDetail[]; // NEW
  start_time: RustTimestamp;
  last_updated: RustTimestamp;
}

// You can use these types like this:
// export type UploadPDFResponse = CustomResponse<UploadResponse>;
// export type ProcessPDFResponse = CustomResponse<ProcessResponse>;
