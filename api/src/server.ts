import express, { Express, Request, Response, Application } from "express";
import dotenv from "dotenv";
import bodyParser from "body-parser";
import {
  upload,
  uploadExists,
  processedExists,
  getProcessedFilePath,
  getUploadFilePath,
  deleteReport,
  deleteUploadedFile,
  getAllReports,
  getAllUploadedFiles,
  getReportContent,
} from "@/helpers/uploadhelper";
import { ResponseHelper } from "@/helpers/response";
import mqConnection, { Queue } from "@/lib/rabbitmq";
import {
  ProcessResponse,
  UploadResponse,
  ProgressResponse,
  CsvColumnsResponse,
  DeleteResponse,
  FileListResponse,
  ReportDetailsResponse,
  ReportListResponse,
} from "@/types/response";
import { ProcessOptions, WordPressFieldMapping } from "@/types/request";
import {
  getFileProgress,
  isFileInProcessing,
  startFileProcess,
  fileProcessingService,
} from "@/lib/redis";
import fs from "fs";
import path from "path";
import csv from "csv-parser";
import { cleanFieldMapping } from "./helpers/helper";
import { ProgressService } from "@/lib/redis/ProgressService";

dotenv.config();

const app: Application = express();
const port = process.env.API_PORT || 3000;

// Constants for time estimation
const AVERAGE_ROW_PROCESS_TIME_MS = 150; // Milliseconds per row (adjust based on your Rust program's performance)

// Initialize connections before starting server
async function initializeConnections() {
  try {
    await mqConnection.connect();
    console.log("Connected to RabbitMQ");
    return true;
  } catch (error) {
    console.error("Failed to connect to RabbitMQ:", error);
    return false;
  }
}

// Helper function to count total rows in CSV
async function countCsvRows(filePath: string): Promise<number> {
  return new Promise((resolve, reject) => {
    let rowCount = 0;
    fs.createReadStream(filePath)
      .pipe(csv())
      .on("data", () => {
        rowCount++;
      })
      .on("error", (error) => {
        reject(error);
      })
      .on("end", () => {
        resolve(rowCount);
      });
  });
}

// Helper function to calculate estimated processing time
function calculateEstimatedTime(totalRows: number): {
  estimatedTimeMs: number;
  estimatedTimeFormatted: string;
} {
  const estimatedTimeMs = totalRows * AVERAGE_ROW_PROCESS_TIME_MS;

  // Format the time in a human-readable format
  let formattedTime = "";
  if (estimatedTimeMs < 1000) {
    formattedTime = `${estimatedTimeMs}ms`;
  } else if (estimatedTimeMs < 60000) {
    formattedTime = `${Math.round(estimatedTimeMs / 1000)}s`;
  } else if (estimatedTimeMs < 3600000) {
    formattedTime = `${Math.round(estimatedTimeMs / 60000)}m ${Math.round(
      (estimatedTimeMs % 60000) / 1000
    )}s`;
  } else {
    const hours = Math.floor(estimatedTimeMs / 3600000);
    const minutes = Math.round((estimatedTimeMs % 3600000) / 60000);
    formattedTime = `${hours}h ${minutes}m`;
  }

  return {
    estimatedTimeMs,
    estimatedTimeFormatted: formattedTime,
  };
}

const progressService = new ProgressService();
// Middleware
app.use(bodyParser.json());
app.use(bodyParser.urlencoded({ extended: true }));
app.use(express.json());
app.use((req, res, next) => {
  ResponseHelper.registerExpressResponse(req, res);
  next();
});

app.use("/", express.static(path.join(__dirname, "../public")));
app.use("/css", express.static(path.join(__dirname, "../public/css")));
app.use("/js", express.static(path.join(__dirname, "../public/js")));

// Get CSV Headers
app.get("/columns/:id", async (req: Request, res: Response) => {
  try {
    const { id } = req.params;
    const fileName = `${id}.csv`;

    if (!uploadExists(fileName)) {
      throw new Error("File not found");
    }

    const filePath = getUploadFilePath(fileName);
    const readHeaders = (): Promise<string[]> => {
      return new Promise((resolve, reject) => {
        const headers: string[] = [];

        fs.createReadStream(filePath)
          .pipe(csv())
          .on("headers", (headerList) => {
            headers.push(...headerList);
            resolve(headers);
          })
          .on("error", (error) => {
            reject(error);
          })
          .on("end", () => {
            if (headers.length === 0) {
              reject(new Error("No headers found in CSV"));
            } else {
              resolve(headers);
            }
          });
      });
    };

    let headers = await readHeaders();
    if (headers.length === 0) {
      throw new Error("No headers found in CSV");
    }
    ResponseHelper.success({ headers });
  } catch (error) {
    ResponseHelper.error(
      (error as Error).message ?? "Failed to read CSV headers",
      { message: (error as Error).message ?? "Failed to read CSV headers" }
    );
  }
});

app.post(
  "/upload",
  upload.single("csv"),
  async (req: Request, res: Response) => {
    try {
      if (!req.file) {
        throw new Error("File is missing");
      }
      ResponseHelper.success<UploadResponse>({
        id: req.file.filename.split(".").slice(0, -1).join("."),
        filename: req.file.filename,
        path: req.file.path,
        size: req.file.size,
      });
    } catch (error) {
      ResponseHelper.error((error as Error).message ?? "File upload failed", {
        message: (error as Error).message ?? "File upload failed",
      });
    }
  }
);

// Usage in your endpoint:
app.post("/process/:id", async (req: Request, res: Response) => {
  try {
    const { id } = req.params;
    const {
      siteDetails,
      priority = 1,
      startRow = 0,
      rowCount = 99999,
      is_new_upload = false, // Default to false if not provided
      wordpress_field_mapping,
    } = req.body as ProcessOptions;

    const fileName = `${id}.csv`;
    if (!uploadExists(fileName)) {
      throw new Error("File not found");
    }

    if (
      !wordpress_field_mapping ||
      Object.keys(wordpress_field_mapping).length === 0
    ) {
      throw new Error("Field mapping is required");
    }

    // Clean the field mapping to remove BOM characters
    const cleanedMapping = cleanFieldMapping(wordpress_field_mapping);

    // Get the actual CSV file path
    const filePath = getUploadFilePath(fileName);

    // Count total rows in the CSV
    const totalEntries = await countCsvRows(filePath);

    // Calculate estimated processing time based on row count
    const { estimatedTimeMs, estimatedTimeFormatted } = calculateEstimatedTime(
      Math.min(totalEntries, rowCount)
    );

    if (await fileProcessingService.isFileInProcessing(id)) {
      console.log("File is already in processing");
      const progress = (await fileProcessingService.getFileProgress(id)) ?? 0;

      ResponseHelper.success<ProcessResponse>({
        id,
        file: fileName,
        message: "File is already in processing",
        options: {
          priority,
          wordpress_field_mapping: cleanedMapping,
          siteDetails: { ...siteDetails, secret: "***" },
        },
        status: "processing",
        progress,
        totalEntries,
        estimatedTimeMs,
        estimatedTime: estimatedTimeFormatted,
      });
      return;
    }

    const d = await mqConnection.sendToQueue(Queue.CSV_UPLOAD, {
      site_details: siteDetails,
      file: fileName,
      start_row: startRow,
      row_count: rowCount,
      wordpress_field_mapping: cleanedMapping,
      is_new_upload,
    });

    if (!d) {
      throw new Error("Failed to send file to queue");
    }

    await fileProcessingService.startFileProcess(id);

    ResponseHelper.success<ProcessResponse>({
      id,
      file: fileName,
      message: "File processing started",
      options: {
        priority,
        wordpress_field_mapping: cleanedMapping,
        siteDetails: { ...siteDetails, secret: "***" },
      },
      status: "queued",
      progress: 0,
      queuedAt: new Date(),
      totalEntries,
      estimatedTimeMs,
      estimatedTime: estimatedTimeFormatted,
    });
  } catch (error) {
    ResponseHelper.error((error as Error).message ?? "File processing failed", {
      message: (error as Error).message ?? "File processing failed",
    });
  }
});

// Replace the existing /progress/:id endpoint with:
app.get("/progress/:id", async (req: Request, res: Response) => {
  try {
    const { id } = req.params;
    const fileName = `${id}.csv`;

    if (!uploadExists(fileName)) {
      throw new Error("File not found");
    }

    const progressResponse = await progressService.getProgressResponse(id);

    ResponseHelper.success(progressResponse);
  } catch (error) {
    ResponseHelper.error(
      (error as Error).message ?? "Failed to retrieve progress",
      { message: (error as Error).message ?? "Failed to retrieve progress" }
    );
  }
});
// Get CSV Headers
app.get("/columns/:id", async (req: Request, res: Response) => {
  try {
    const { id } = req.params;
    const fileName = `${id}.csv`;

    if (!uploadExists(fileName)) {
      throw new Error("File not found");
    }

    const filePath = getUploadFilePath(fileName);
    const readHeaders = (): Promise<string[]> => {
      return new Promise((resolve, reject) => {
        const headers: string[] = [];

        fs.createReadStream(filePath)
          .pipe(csv())
          .on("headers", (headerList: string[]) => {
            headers.push(...headerList);
            resolve(headers);
          })
          .on("error", (error: Error) => {
            reject(error);
          })
          .on("end", () => {
            if (headers.length === 0) {
              reject(new Error("No headers found in CSV"));
            } else {
              resolve(headers);
            }
          });
      });
    };

    const headers = await readHeaders();
    if (headers.length === 0) {
      throw new Error("No headers found in CSV");
    }
    ResponseHelper.success<CsvColumnsResponse>({ headers });
  } catch (error) {
    ResponseHelper.error(
      (error as Error).message ?? "Failed to read CSV headers",
      { message: (error as Error).message ?? "Failed to read CSV headers" }
    );
  }
});

// CSV Management endpoints
app.get("/csv/list", async (req: Request, res: Response) => {
  try {
    const files = getAllUploadedFiles();
    ResponseHelper.success<FileListResponse>({ files });
  } catch (error) {
    ResponseHelper.error("Failed to retrieve CSV files", {
      message: (error as Error).message,
    });
  }
});

app.delete("/csv/:filename", async (req: Request, res: Response) => {
  try {
    const { filename } = req.params;

    if (!filename.endsWith(".csv")) {
      throw new Error("Invalid file format");
    }

    const deleted: boolean = deleteUploadedFile(filename);
    if (deleted) {
      ResponseHelper.success<DeleteResponse>({
        message: "File deleted successfully",
      });
    } else {
      throw new Error("File not found or could not be deleted");
    }
  } catch (error) {
    ResponseHelper.error("Failed to delete CSV file", {
      message: (error as Error).message,
    });
  }
});

// Reports endpoints
app.get("/reports/list", async (req: Request, res: Response) => {
  try {
    const reports = getAllReports();
    ResponseHelper.success<ReportListResponse>({ reports });
  } catch (error) {
    ResponseHelper.error("Failed to retrieve reports", {
      message: (error as Error).message,
    });
  }
});

app.get("/reports/:fileId", async (req: Request, res: Response) => {
  try {
    const { fileId } = req.params;
    const report = getReportContent(fileId);

    if (report) {
      ResponseHelper.success<ReportDetailsResponse>({ report });
    } else {
      throw new Error("Report not found");
    }
  } catch (error) {
    ResponseHelper.error("Failed to retrieve report", {
      message: (error as Error).message,
    });
  }
});

app.delete("/reports/:fileId", async (req: Request, res: Response) => {
  try {
    const { fileId } = req.params;
    const deleted: boolean = deleteReport(fileId);

    if (deleted) {
      ResponseHelper.success<DeleteResponse>({
        message: "Report deleted successfully",
      });
    } else {
      throw new Error("Report not found or could not be deleted");
    }
  } catch (error) {
    ResponseHelper.error("Failed to delete report", {
      message: (error as Error).message,
    });
  }
});

// Start server only after establishing connections
async function startServer() {
  const isConnected = await initializeConnections();

  if (isConnected) {
    app.listen(port, () => {
      console.log(`Server is running at http://localhost:${port}`);
    });
  } else {
    console.error("Failed to initialize required connections. Exiting...");
    process.exit(1);
  }
}

startServer();
