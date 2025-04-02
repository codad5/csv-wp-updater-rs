import { WordPressFieldMapping } from "@/types/request";

/**
 * Removes unwanted characters like BOM (Byte Order Mark) from field mappings
 * @param mapping The original WordPress field mapping object
 * @returns Cleaned WordPress field mapping object
 */
export function cleanFieldMapping(mapping: WordPressFieldMapping): WordPressFieldMapping {
  const cleanedMapping: WordPressFieldMapping = {};
  
  for (const [key, value] of Object.entries(mapping)) {
    if (typeof value === 'string') {
      // Remove BOM and other problematic characters
      let cleanValue = value
        .replace(/^\uFEFF/, '')        // Remove BOM
        .replace(/[\u200B\u200C]/g, '') // Remove zero-width spaces
        .replace(/\u00A0/g, ' ')       // Replace non-breaking spaces with regular spaces
        .replace(/[\x00-\x1F]/g, '')   // Remove control characters
        .trim();                        // Remove leading/trailing whitespace
        
      cleanedMapping[key as keyof WordPressFieldMapping] = cleanValue;
    } else {
      cleanedMapping[key as keyof WordPressFieldMapping] = value;
    }
  }
  
  return cleanedMapping;
}