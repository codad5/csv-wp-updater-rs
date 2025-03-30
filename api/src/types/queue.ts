    export type NewCSVUploadQueue = {
        file: string; // relative part to shared_storage
        start_row: number; // page number to start processing
        row_count: number; // number of pages to process use 0 for all
        piority?: 0 | 1 | 2; // 0 - low, 1 - medium, 2 - high
        // a new property for field mapping like this 
    }


    export type Status = 'queued' | 'processing' | 'completed' | 'failed'


