// src/lib/redis/index.ts
import { FileProcessingService } from './FileProcessingService';

// Create singleton instances
export const fileProcessingService = new FileProcessingService();

// Re-export types
export { FileStatus } from './FileProcessingService';

// Export all methods from both services
export const {
    isFileInProcessing,
    isProcessDone,
    startFileProcess,
    markAsDone: markFileAsDone,
    markAsFailed: markFileAsFailed,
    markProgress: markFileProgress,
    getFileProgress,
} = fileProcessingService;
