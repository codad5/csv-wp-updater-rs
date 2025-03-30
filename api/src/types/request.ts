export interface ProcessOptions {
    rowCount?: number;
    startRow?: number;
    priority?: 0 | 1 | 2; // 0 = low, 1 = normal, 2 = high
}