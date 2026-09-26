export function suggestRecordNumber(existingNumbers:string[],prefix:string):string{
 const escaped=prefix.replace(/[.*+?^${}()|[\]\\]/g,"\\$&");
 const pattern=new RegExp(`^${escaped}-(\\d+)$`,"i");
 let highest=0,width=3;
 const occupied=new Set(existingNumbers.map(value=>value.trim().toLocaleLowerCase()));
 for(const value of existingNumbers){const match=value.trim().match(pattern);if(!match)continue;highest=Math.max(highest,Number(match[1]));width=Math.max(width,match[1].length)}
 let candidate:string;
 do{highest+=1;candidate=`${prefix}-${String(highest).padStart(width,"0")}`}while(occupied.has(candidate.toLocaleLowerCase()));
 return candidate;
}
