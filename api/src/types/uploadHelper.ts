import { UploadedFile, ReportFile, ProcessingProgress } from "./response";

// Function return types for upload helper
export type GetAllUploadedFilesReturn = UploadedFile[];
export type GetAllReportsReturn = ReportFile[];
export type GetReportContentReturn = ProcessingProgress | null;
export type DeleteFileReturn = boolean;
export type DeleteReportReturn = boolean;
export type FileExistsReturn = boolean;
export type GetPathReturn = string;
