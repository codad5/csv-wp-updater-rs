import multer from "multer";
import path from "path";
import fs from "fs";
import { UploadedFile, ReportFile, ProcessingProgress } from "@/types/response";
import { FileExistsReturn, GetPathReturn, GetAllUploadedFilesReturn, DeleteFileReturn, GetAllReportsReturn, GetReportContentReturn, DeleteReportReturn } from "@/types/uploadHelper";

// Define paths for upload and processed directories
const csvUploadPath = `${process.env.SHARED_STORAGE_PATH}/upload/csv`;
const processedPath = `${process.env.SHARED_STORAGE_PATH}/processed`;

// Ensure directories exist
if (!fs.existsSync(csvUploadPath))
  fs.mkdirSync(csvUploadPath, { recursive: true });
if (!fs.existsSync(processedPath))
  fs.mkdirSync(processedPath, { recursive: true });

const allowedExtensions = [".csv"]; // Define allowable extensions
const allowedMimes = [
  "text/csv",
  "application/csv",
  "application/vnd.ms-excel",
]; // Common CSV MIME types

const storage = multer.diskStorage({
  destination: (req, file, cb) => {
    cb(null, csvUploadPath); // Directory where files will be saved
  },
  filename: (req, file, cb) => {
    cb(null, Date.now() + path.extname(file.originalname)); // Rename file with timestamp
  },
});

export const upload = multer({
  storage: storage,
  fileFilter: (req, file, cb) => {
    const fileExtension = path.extname(file.originalname).toLowerCase(); // Get the file extension
    const fileMime = file.mimetype; // Get the MIME type
    console.log(fileExtension, fileMime);

    // Validate extension and MIME type
    if (!allowedExtensions.includes(fileExtension)) {
      return cb(
        new Error(
          `Invalid file extension. Only ${allowedExtensions.join(
            ", "
          )} files are allowed.`
        )
      );
    }

    // Some browsers/clients may send CSV files with various MIME types
    // Being a bit more flexible with MIME types for CSVs
    if (
      !allowedMimes.includes(fileMime) &&
      fileMime !== "application/octet-stream"
    ) {
      return cb(
        new Error(
          `Invalid MIME type. Only ${allowedMimes.join(
            ", "
          )} MIME types are allowed.`
        )
      );
    }

    cb(null, true); // Accept the file
  },
});
export const uploadExists = (filename: string): FileExistsReturn => {
  return fs.existsSync(path.join(csvUploadPath, filename));
};

export const processedExists = (filename: string): FileExistsReturn => {
  return fs.existsSync(path.join(processedPath, filename));
};

export const getProcessedFilePath = (filename: string): GetPathReturn => {
  return path.join(processedPath, filename);
};

export const getUploadFilePath = (filename: string): GetPathReturn => {
  return path.join(csvUploadPath, filename);
};

export const getAllUploadedFiles = (): GetAllUploadedFilesReturn => {
  try {
    const files = fs.readdirSync(csvUploadPath);
    return files
      .filter((file: string) => file.endsWith(".csv"))
      .map((file: string): UploadedFile => {
        const filePath = path.join(csvUploadPath, file);
        const stats = fs.statSync(filePath);
        return {
          filename: file,
          id: file.split(".").slice(0, -1).join("."),
          size: stats.size,
          uploadedAt: stats.birthtime,
          modifiedAt: stats.mtime,
        };
      })
      .sort(
        (a: UploadedFile, b: UploadedFile) =>
          b.uploadedAt.getTime() - a.uploadedAt.getTime()
      );
  } catch (error) {
    console.error("Error reading upload directory:", error);
    return [];
  }
};

export const deleteUploadedFile = (filename: string): DeleteFileReturn => {
  try {
    const filePath = path.join(csvUploadPath, filename);
    if (fs.existsSync(filePath)) {
      fs.unlinkSync(filePath);
      return true;
    }
    return false;
  } catch (error) {
    console.error("Error deleting file:", error);
    return false;
  }
};

export const getReportsPath = (): GetPathReturn => {
  const reportsPath = `${process.env.SHARED_STORAGE_PATH}/processed/history`;
  if (!fs.existsSync(reportsPath)) {
    fs.mkdirSync(reportsPath, { recursive: true });
  }
  return reportsPath;
};

export const getAllReports = (): GetAllReportsReturn => {
  try {
    const reportsPath = getReportsPath();
    const files = fs.readdirSync(reportsPath);
    return files
      .filter((file: string) => file.endsWith("_processing_result.json"))
      .map((file: string): ReportFile => {
        const filePath = path.join(reportsPath, file);
        const stats = fs.statSync(filePath);
        const fileId = file.replace("_processing_result.json", "");
        return {
          filename: file,
          fileId,
          size: stats.size,
          createdAt: stats.birthtime,
          modifiedAt: stats.mtime,
        };
      })
      .sort(
        (a: ReportFile, b: ReportFile) =>
          b.createdAt.getTime() - a.createdAt.getTime()
      );
  } catch (error) {
    console.error("Error reading reports directory:", error);
    return [];
  }
};

export const getReportContent = (fileId: string): GetReportContentReturn => {
  try {
    const reportsPath = getReportsPath();
    const reportFile = `${fileId}_processing_result.json`;
    const filePath = path.join(reportsPath, reportFile);

    if (fs.existsSync(filePath)) {
      const content = fs.readFileSync(filePath, "utf8");
      return JSON.parse(content) as ProcessingProgress;
    }
    return null;
  } catch (error) {
    console.error("Error reading report:", error);
    return null;
  }
};

export const deleteReport = (fileId: string): DeleteReportReturn => {
  try {
    const reportsPath = getReportsPath();
    const reportFile = `${fileId}_processing_result.json`;
    const filePath = path.join(reportsPath, reportFile);

    if (fs.existsSync(filePath)) {
      fs.unlinkSync(filePath);
      return true;
    }
    return false;
  } catch (error) {
    console.error("Error deleting report:", error);
    return false;
  }
};
