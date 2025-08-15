import { WordPressFieldMapping } from "./request";

export type NewCSVUploadQueue = {
  file: string; // relative part to shared_storage
  start_row: number; // page number to start processing
  row_count: number; // number of pages to process use 0 for all
  piority?: 0 | 1 | 2; // 0 - low, 1 - medium, 2 - high
  is_new_upload?: boolean; // if true, it will be a new upload and not an update
  wordpress_field_mapping: WordPressFieldMapping;
  site_details: {
    key: string;
    secret: string;
    url: string;
    name: string;
  };
  batch_size: number;
  batch_delay_minutes: number; // delay in minutes between batchesdry_run?: boolean; // if true, it will not make any API calls, just log the actions
  dry_run: boolean; // if true, it will not make any API calls, just log the actions
};

export type Status = "queued" | "processing" | "completed" | "failed";
