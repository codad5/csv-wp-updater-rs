import multer from "multer";
import path from "path";
import fs from "fs";

// Define paths for upload and processed directories
const csvUploadPath = `${process.env.SHARED_STORAGE_PATH}/upload/csv`;
const processedPath = `${process.env.SHARED_STORAGE_PATH}/processed`;

// Ensure directories exist
if (!fs.existsSync(csvUploadPath)) fs.mkdirSync(csvUploadPath, { recursive: true });
if (!fs.existsSync(processedPath)) fs.mkdirSync(processedPath, { recursive: true });

const allowedExtensions = ['.csv']; // Define allowable extensions
const allowedMimes = ['text/csv', 'application/csv', 'application/vnd.ms-excel']; // Common CSV MIME types

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
          `Invalid file extension. Only ${allowedExtensions.join(', ')} files are allowed.`
        )
      );
    }
    
    // Some browsers/clients may send CSV files with various MIME types
    // Being a bit more flexible with MIME types for CSVs
    if (!allowedMimes.includes(fileMime) && fileMime !== 'application/octet-stream') {
      return cb(
        new Error(
          `Invalid MIME type. Only ${allowedMimes.join(', ')} MIME types are allowed.`
        )
      );
    }
    
    cb(null, true); // Accept the file
  },
});

export const uploadExists = (filename: string) => {
  return fs.existsSync(path.join(csvUploadPath, filename));
};

export const processedExists = (filename: string) => {
  return fs.existsSync(path.join(processedPath, filename));
};

export const getProcessedFilePath = (filename: string) => {
  return path.join(processedPath, filename);
};

// Helper function to get the upload path
export const getUploadFilePath = (filename: string) => {
  return path.join(csvUploadPath, filename);
};