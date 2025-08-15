// src/lib/redis/ProgressService.ts
import { BaseRedisService } from "./BaseRedisService";

export interface ProcessingStage {
  stage:
    | "Starting"
    | "ParsingCsv"
    | "GroupingProducts"
    | "GroupingProductsCompleted"
    | "ProcessingProducts"
    | "ProcessingVariations"
    | "Finalizing"
    | "Completed"
    | "Failed";
  message: string;
  details?: {
    sku?: string;
    current?: number;
    total?: number;
    total_parents?: number;
    total_products?: number;
    error?: string;
  };
}

export interface ProcessingProgress {
  file_id: string;
  percent: number;
  stage: ProcessingStage;
  stage_message: string;
  total_rows: number;
  processed_rows: number;
  successful_rows: number;
  failed_rows: number;
  start_time: string;
  last_updated: string;
}

export interface ProgressResponse {
  id: string;
  progress: number;
  status: "processing" | "completed" | "failed" | "not_found";
  message: string;
  stage?: ProcessingStage;
  stage_message: string;
  totalEntries?: number;
  processedEntries?: number;
  successfulEntries?: number;
  failedEntries?: number;
  estimatedTimeRemaining?: string;
  estimatedTimeRemainingMs?: number;
  startTime?: string;
  lastUpdated?: string;
}

export class ProgressService extends BaseRedisService {
  constructor() {
    super("progress");
  }

  async getDetailedProgress(
    fileId: string
  ): Promise<ProcessingProgress | null> {
    try {
      const progressJson = await this.redis.get(`${this.prefix}:${fileId}`);
      if (!progressJson) {
        return null;
      }
      return JSON.parse(progressJson) as ProcessingProgress;
    } catch (error) {
      console.error("Error parsing progress JSON:", error);
      return null;
    }
  }

  async getProgressResponse(fileId: string): Promise<ProgressResponse> {
    const detailedProgress = await this.getDetailedProgress(fileId);

    if (!detailedProgress) {
      return {
        id: fileId,
        progress: 0,
        status: "not_found",
        stage_message: "Not found",
        message: "File processing not found",
      };
    }

    // Determine status based on stage
    let status: "processing" | "completed" | "failed" = "processing";
    if (detailedProgress.stage.stage === "Completed") {
      status = "completed";
    } else if (detailedProgress.stage.stage === "Failed") {
      status = "failed";
    }

    // Calculate estimated time remaining
    const { estimatedTimeRemaining, estimatedTimeRemainingMs } =
      this.calculateEstimatedTime(
        detailedProgress.total_rows,
        detailedProgress.processed_rows,
        detailedProgress.start_time
      );

    // if progress is 100% clear the cache key
    if (
      detailedProgress.percent >= 100 ||
      status === "completed" ||
      status === "failed"
    ) {
      await this.redis.del(`${this.prefix}:${fileId}`);
    }

    return {
      id: fileId,
      progress: Math.round(detailedProgress.percent * 100) / 100, // Round to 2 decimal places
      status,
      message: detailedProgress.stage.message,
      stage: detailedProgress.stage,
      stage_message: detailedProgress.stage_message ?? "Processing",
      totalEntries: detailedProgress.total_rows,
      processedEntries: detailedProgress.processed_rows,
      successfulEntries: detailedProgress.successful_rows,
      failedEntries: detailedProgress.failed_rows,
      estimatedTimeRemaining,
      estimatedTimeRemainingMs,
      startTime: detailedProgress.start_time,
      lastUpdated: detailedProgress.last_updated,
    };
  }

  private calculateEstimatedTime(
    totalRows: number,
    processedRows: number,
    startTime: string
  ): {
    estimatedTimeRemaining: string;
    estimatedTimeRemainingMs: number;
  } {
    const AVERAGE_ROW_PROCESS_TIME_MS = 150; // Milliseconds per row

    const remainingRows = totalRows - processedRows;
    const estimatedTimeRemainingMs =
      remainingRows * AVERAGE_ROW_PROCESS_TIME_MS;

    // If we have actual processing data, use it for better estimation
    const startTimestamp = new Date(startTime).getTime();
    const currentTime = Date.now();
    const elapsedMs = currentTime - startTimestamp;

    let actualEstimatedMs = estimatedTimeRemainingMs;

    if (processedRows > 0 && elapsedMs > 0) {
      const actualTimePerRow = elapsedMs / processedRows;
      actualEstimatedMs = remainingRows * actualTimePerRow;
    }

    // Format the time in a human-readable format
    let formattedTime = "";
    if (actualEstimatedMs < 1000) {
      formattedTime = `${Math.round(actualEstimatedMs)}ms`;
    } else if (actualEstimatedMs < 60000) {
      formattedTime = `${Math.round(actualEstimatedMs / 1000)}s`;
    } else if (actualEstimatedMs < 3600000) {
      const minutes = Math.floor(actualEstimatedMs / 60000);
      const seconds = Math.round((actualEstimatedMs % 60000) / 1000);
      formattedTime = `${minutes}m ${seconds}s`;
    } else {
      const hours = Math.floor(actualEstimatedMs / 3600000);
      const minutes = Math.round((actualEstimatedMs % 3600000) / 60000);
      formattedTime = `${hours}h ${minutes}m`;
    }

    return {
      estimatedTimeRemaining: formattedTime,
      estimatedTimeRemainingMs: Math.round(actualEstimatedMs),
    };
  }

  // Quick method to get just the percentage
  async getProgressPercent(fileId: string): Promise<number> {
    try {
      const percent = await this.redis.get(`${this.prefix}:percent:${fileId}`);
      return percent ? parseFloat(percent) : 0;
    } catch (error) {
      console.error("Error getting progress percent:", error);
      return 0;
    }
  }

  // Check if file is currently being processed
  async isProcessing(fileId: string): Promise<boolean> {
    const progress = await this.getDetailedProgress(fileId);
    if (!progress) return false;

    return !["Completed", "Failed"].includes(progress.stage.stage);
  }
}
