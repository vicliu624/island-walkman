import diagnostics from './diagnostics.json';

const escape = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const patterns = diagnostics.flatMap(row => [0, 2].map(column => ({row,
    pattern: new RegExp('^' + row[column].split(/\{[^}]*\}/).map(escape).join('([\\s\\S]*?)') + '$')
}))).sort((a,b) => b.row[0].replace(/\{[^}]*\}/g,'').length-a.row[0].replace(/\{[^}]*\}/g,'').length);

export function translateDiagnostic(locale: string, text: string, depth = 0): string {
    for (const {row, pattern} of patterns) {
        const match=pattern.exec(text);
        if (!match) continue;
        let index=1;
        return row[locale==='ja'?1:locale==='zh-CN'?2:0].replace(/\{[^}]*\}/g,()=> {
            const value=match[index++] ?? '';
            return depth<3 ? translateDiagnostic(locale,value,depth+1) : value;
        });
    }
    return text;
}
