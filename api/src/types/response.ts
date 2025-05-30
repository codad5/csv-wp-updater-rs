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
    options ?: ProcessOptions;
    progress: number;
    status?: Status;
    queuedAt?: Date;
    totalEntries?: number;
    estimatedTimeMs?: number;
    estimatedTime?: string;
}

export interface ProgressResponse {
    id: string;
    progress: number;
    status: Status;
    message?: string;
    totalEntries?: number;
    estimatedTimeRemaining?: string;
    estimatedTimeRemainingMs?: number;
}






// You can use these types like this:
// export type UploadPDFResponse = CustomResponse<UploadResponse>;
// export type ProcessPDFResponse = CustomResponse<ProcessResponse>;