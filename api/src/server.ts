import express, { Express, Request, Response, Application } from 'express';
import dotenv from 'dotenv';
import bodyParser from 'body-parser';
import { upload, uploadExists, processedExists, getProcessedFilePath, getUploadFilePath } from '@/helpers/uploadhelper';
import { ResponseHelper } from '@/helpers/response';
import mqConnection, { Queue } from '@/lib/rabbitmq';
import { ProcessResponse, UploadResponse, ProgressResponse } from '@/types/response';
import { ProcessOptions, WordPressFieldMapping } from '@/types/request';
import {
    getFileProgress,
    isFileInProcessing,
    startFileProcess,
    fileProcessingService
} from '@/lib/redis';
import fs from 'fs';
import path from 'path';
import csv from 'csv-parser';

dotenv.config();

const app: Application = express();
const port = process.env.API_PORT || 3000;

// Constants for time estimation
const AVERAGE_ROW_PROCESS_TIME_MS = 150; // Milliseconds per row (adjust based on your Rust program's performance)

// Initialize connections before starting server
async function initializeConnections() {
    try {
        await mqConnection.connect();
        console.log('Connected to RabbitMQ');
        return true;
    } catch (error) {
        console.error('Failed to connect to RabbitMQ:', error);
        return false;
    }
}

// Helper function to count total rows in CSV
async function countCsvRows(filePath: string): Promise<number> {
    return new Promise((resolve, reject) => {
        let rowCount = 0;
        fs.createReadStream(filePath)
            .pipe(csv())
            .on('data', () => {
                rowCount++;
            })
            .on('error', (error) => {
                reject(error);
            })
            .on('end', () => {
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
    let formattedTime = '';
    if (estimatedTimeMs < 1000) {
        formattedTime = `${estimatedTimeMs}ms`;
    } else if (estimatedTimeMs < 60000) {
        formattedTime = `${Math.round(estimatedTimeMs / 1000)}s`;
    } else if (estimatedTimeMs < 3600000) {
        formattedTime = `${Math.round(estimatedTimeMs / 60000)}m ${Math.round((estimatedTimeMs % 60000) / 1000)}s`;
    } else {
        const hours = Math.floor(estimatedTimeMs / 3600000);
        const minutes = Math.round((estimatedTimeMs % 3600000) / 60000);
        formattedTime = `${hours}h ${minutes}m`;
    }
    
    return {
        estimatedTimeMs,
        estimatedTimeFormatted: formattedTime
    };
}

// Middleware
app.use(bodyParser.json());
app.use(bodyParser.urlencoded({ extended: true }));
app.use(express.json());
app.use((req, res, next) => {
    ResponseHelper.registerExpressResponse(req, res);
    next();
});

app.use('/', express.static(path.join(__dirname, '../public')));

// Get CSV Headers
app.get('/columns/:id', async (req: Request, res: Response) => {
    try {
        const { id } = req.params;
        const fileName = `${id}.csv`;
        
        if (!uploadExists(fileName)) {
            throw new Error('File not found');
        }
        
        const filePath = getUploadFilePath(fileName);
        const headers: string[] = [];
        
        fs.createReadStream(filePath)
            .pipe(csv())
            .on('headers', (headerList) => {
                headers.push(...headerList);
                ResponseHelper.success({ headers });
            })
            .on('error', (error) => {
                ResponseHelper.error('Failed to read CSV headers', { message: error.message });
            })
            .on('end', () => {
                if (headers.length === 0) {
                    ResponseHelper.error('No headers found in CSV', { message: 'CSV file has no headers' });
                }
            });
            
    } catch (error) {
        ResponseHelper.error(
            (error as Error).message ?? 'Failed to read CSV headers',
            { message: (error as Error).message ?? 'Failed to read CSV headers' }
        );
    }
});

app.post('/upload', upload.single('csv'), async (req: Request, res: Response) => {
    try {
        if (!req.file) {
            throw new Error('File is missing');
        }
        ResponseHelper.success<UploadResponse>({
            id: req.file.filename.split('.').slice(0, -1).join('.'),
            filename: req.file.filename,
            path: req.file.path,
            size: req.file.size
        });
    } catch (error) {
        ResponseHelper.error(
            (error as Error).message ?? 'File upload failed',
            { message: (error as Error).message ?? 'File upload failed' }
        );
    }
});

app.post('/process/:id', async (req: Request, res: Response) => {
    try {
        const { id } = req.params;
        const { 
            priority = 1, 
            startRow = 0, 
            rowCount = 99999,
            wordpress_field_mapping 
        } = req.body as ProcessOptions;

        const fileName = `${id}.csv`;
        if (!uploadExists(fileName)) {
            throw new Error('File not found');
        }

        if (!wordpress_field_mapping || Object.keys(wordpress_field_mapping).length === 0) {
            throw new Error('Field mapping is required');
        }

        // Get the actual CSV file path
        const filePath = getUploadFilePath(fileName);
        
        // Count total rows in the CSV
        const totalEntries = await countCsvRows(filePath);
        
        // Calculate estimated processing time based on row count
        const { estimatedTimeMs, estimatedTimeFormatted } = calculateEstimatedTime(
            Math.min(totalEntries, rowCount)
        );

        if (await fileProcessingService.isFileInProcessing(id)) {
            console.log('File is already in processing');
            const progress = await fileProcessingService.getFileProgress(id) ?? 0;
            
            ResponseHelper.success<ProcessResponse>({
                id,
                file: fileName,
                message: 'File is already in processing',
                options: { priority, wordpress_field_mapping },
                status: 'processing',
                progress,
                totalEntries,
                estimatedTimeMs,
                estimatedTime: estimatedTimeFormatted
            });
            return;
        }
        
        const d = await mqConnection.sendToQueue(Queue.CSV_UPLOAD, {
            file: fileName,
            start_row: startRow,
            row_count: rowCount,
            wordpress_field_mapping: wordpress_field_mapping,
        });

        if (!d) {
            throw new Error('Failed to send file to queue');
        }

        await fileProcessingService.startFileProcess(id);

        ResponseHelper.success<ProcessResponse>({
            id,
            file: fileName,
            message: 'File processing started',
            options: { priority, wordpress_field_mapping },
            status: 'queued',
            progress: 0,
            queuedAt: new Date(),
            totalEntries,
            estimatedTimeMs,
            estimatedTime: estimatedTimeFormatted
        });
    } catch (error) {
        ResponseHelper.error(
            (error as Error).message ?? 'File processing failed',
            { message: (error as Error).message ?? 'File processing failed' }
        );
    }
});

app.get('/progress/:id', async (req: Request, res: Response) => {
    try {
        const { id } = req.params;
        const fileName = `${id}.csv`;

        if (!uploadExists(fileName)) {
            throw new Error('File not found');
        }

        const progress = await fileProcessingService.getFileProgress(id);
        const status = await fileProcessingService.isFileInProcessing(id) ? 'processing' : 'completed';
        
        // Get the actual CSV file path
        const filePath = getUploadFilePath(fileName);
        
        // Count total rows in the CSV if needed
        const totalEntries = await countCsvRows(filePath);
        
        // Calculate remaining time based on progress
        const completedPercentage = progress ? progress / 100 : 0;
        const remainingEntries = totalEntries * (1 - completedPercentage);
        const { estimatedTimeMs, estimatedTimeFormatted } = calculateEstimatedTime(remainingEntries);

        ResponseHelper.success<ProgressResponse>({
            id,
            progress: progress ?? 0,
            status,
            message: 'Progress retrieved successfully',
            totalEntries,
            estimatedTimeRemaining: estimatedTimeFormatted,
            estimatedTimeRemainingMs: estimatedTimeMs
        });
    } catch (error) {
        ResponseHelper.error(
            (error as Error).message ?? 'Failed to retrieve progress',
            { message: (error as Error).message ?? 'Failed to retrieve progress' }
        );
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
        console.error('Failed to initialize required connections. Exiting...');
        process.exit(1);
    }
}

startServer();