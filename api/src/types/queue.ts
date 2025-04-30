import { WordPressFieldMapping } from "./request";

export type NewCSVUploadQueue = {
    file: string; // relative part to shared_storage
    start_row: number; // page number to start processing
    row_count: number; // number of pages to process use 0 for all
    piority?: 0 | 1 | 2; // 0 - low, 1 - medium, 2 - high
    is_new_upload?: boolean; // if true, it will be a new upload and not an update
    wordpress_field_mapping: WordPressFieldMapping;
    siteDetails: {
        key: string;
        secret: string;
        url: string;
        name: string;
    }
}

export type Status = 'queued' | 'processing' | 'completed' | 'failed';