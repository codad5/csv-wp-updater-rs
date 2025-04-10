import { WordPressFieldMapping } from "@/types/request";

export function cleanFieldMapping(mapping: WordPressFieldMapping): WordPressFieldMapping {
  const cleanedMapping: WordPressFieldMapping = {};
  
  for (const [key, value] of Object.entries(mapping)) {
    if (key === "attributes") {
      if (
        typeof value === "object" &&
        value !== null &&
        !Array.isArray(value)
      ) {
        const cleanedAttributes: { [key: string]: { column: string, variable: boolean } } = {};
        
        for (const [attrKey, attrVal] of Object.entries(value)) {
          if (typeof attrVal === "object" && attrVal !== null) {
            // Type assertion to help TypeScript understand the structure
            const attributeValue = attrVal as { column?: string; variable?: boolean };
            
            if ('column' in attributeValue) {
              cleanedAttributes[attrKey] = {
                column: typeof attributeValue.column === "string" 
                  ? attributeValue.column
                    .replace(/^\uFEFF/, '')
                    .replace(/[\u200B\u200C]/g, '')
                    .replace(/\u00A0/g, ' ')
                    .replace(/[\x00-\x1F]/g, '')
                    .trim()
                  : '',
                variable: Boolean(attributeValue.variable)
              };
            }
          }
        }
        
        cleanedMapping.attributes = cleanedAttributes;
      }
    } else if (typeof value === "string") {
      const cleanValue = value
        .replace(/^\uFEFF/, '')
        .replace(/[\u200B\u200C]/g, '')
        .replace(/\u00A0/g, ' ')
        .replace(/[\x00-\x1F]/g, '')
        .trim();
      
      // Now we cast explicitly to fix TS error only for known safe fields
      (cleanedMapping as any)[key] = cleanValue;
    } else {
      (cleanedMapping as any)[key] = value;
    }
  }
  
  return cleanedMapping;
}