import express, { Express, Request, Response, Application } from 'express';
import dotenv from 'dotenv';
import bodyParser from 'body-parser';
import { upload, uploadExists, processedExists, getProcessedFilePath, getUploadFilePath } from '@/helpers/uploadhelper';
import { ResponseHelper } from '@/helpers/response';
import mqConnection, { Queue } from '@/lib/rabbitmq';
import { ProcessResponse, UploadResponse, ProgressResponse } from '@/types/response';
import { ProcessOptions } from '@/types/request';
import {
    getFileProgress,
    isFileInProcessing,
    startFileProcess,
    fileProcessingService
} from '@/lib/redis';
import fs from 'fs';
import path from 'path';

dotenv.config();

const app: Application = express();
const port = process.env.API_PORT || 3000;

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

// Middleware
app.use(bodyParser.json());
app.use(bodyParser.urlencoded({ extended: true }));
app.use(express.json());
app.use((req, res, next) => {
    ResponseHelper.registerExpressResponse(req, res);
    next();
});

app.use('/', express.static(path.join(__dirname, '../public')));


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
        const { priority = 1, startRow = 0, rowCount = 99999 } = req.body as ProcessOptions;

        if (!uploadExists(`${id}.csv`)) {
            throw new Error('File not found');
        }

        if (await fileProcessingService.isFileInProcessing(id)) {
            console.log('File is already in processing');
            const progress = await fileProcessingService.getFileProgress(id) ?? 0;
            
            ResponseHelper.success<ProcessResponse>({
                id,
                file: `${id}.csv`,
                message: 'File is already in processing',
                options: { priority },
                status: 'processing',
                progress
            });
            return;
        }
        
        const d = await mqConnection.sendToQueue(Queue.CSV_UPLOAD, {
            file: `${id}.csv`,
            start_row: startRow,
            row_count: rowCount,
        });

        if (!d) {
            throw new Error('Failed to send file to queue');
        }

        await fileProcessingService.startFileProcess(id);

        ResponseHelper.success<ProcessResponse>({
            id,
            file: `${id}.csv`,
            message: 'File processing started',
            options: {  priority },
            status: 'queued',
            progress: 0,
            queuedAt: new Date()
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

        if (!uploadExists(`${id}.csv`)) {
            throw new Error('File not found');
        }

        const progress = await fileProcessingService.getFileProgress(id);
        const status = await fileProcessingService.isFileInProcessing(id) ? 'processing' : 'completed';

        ResponseHelper.success<ProgressResponse>({
            id,
            progress: progress ?? 0,
            status,
            message: 'Progress retrieved successfully'
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