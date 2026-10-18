import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import ts from '../ui/node_modules/typescript/lib/typescript.js';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const module=ts.transpileModule(fs.readFileSync(path.join(root,'ui/src/messages.ts'),'utf8'),{compilerOptions:{module:ts.ModuleKind.ESNext}}).outputText;
const {messages}=await import('data:text/javascript;base64,'+Buffer.from(module).toString('base64'));
const slots=text=>[...text.matchAll(/\{\d+\}/g)].map(match=>match[0]).sort();
let count=0;
for(const [key,values] of Object.entries(messages)) {
    assert(values.length===2||values.length===3,key);
    assert(values.every(value=>typeof value==='string'&&value.trim()),key);
    assert.deepEqual(slots(values[0]),slots(key),`English placeholders: ${key}`);
    assert.deepEqual(slots(values[1]),slots(key),`Japanese placeholders: ${key}`);
    if(values[2]) assert.deepEqual(slots(values[2]),slots(key),`Chinese placeholders: ${key}`);
    assert(!/\p{Script=Han}/u.test(values[0]),`Chinese text in English value: ${key}`);
    count++;
}
let source;
function visit(node){
    if(ts.isCallExpression(node)&&node.expression.getText(source)==='tr'&&ts.isStringLiteral(node.arguments[0])) assert(messages[node.arguments[0].text],`Missing translation: ${node.arguments[0].text}`);
    if(ts.isJsxText(node)&&node.parent.openingElement?.tagName.getText(source)!=='option') assert(!/\p{Script=Han}/u.test(node.text),`Untranslated JSX text: ${node.text}`);
    ts.forEachChild(node,visit);
}
for (const file of ['main.tsx', 'updates.tsx']) { source=ts.createSourceFile(file, fs.readFileSync(path.join(root,'ui/src',file),'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX); visit(source); }
assert(fs.readFileSync(path.join(root,'ui/index.html'),'utf8').includes('lang="en"'));
const diagnostics=JSON.parse(fs.readFileSync(path.join(root,'ui/src/diagnostics.json'),'utf8'));
for(const row of diagnostics){
    assert.equal(row.length,3);
    const placeholders=text=>(text.match(/\{[^}]*\}/g)||[]).sort();
    assert.deepEqual(placeholders(row[0]),placeholders(row[1]));
    assert.deepEqual(placeholders(row[0]),placeholders(row[2]));
}
let diagnosticSource=fs.readFileSync(path.join(root,'ui/src/diagnostic-text.ts'),'utf8').replace("import diagnostics from './diagnostics.json';",'const diagnostics='+JSON.stringify(diagnostics)+';');
diagnosticSource=ts.transpileModule(diagnosticSource,{compilerOptions:{module:ts.ModuleKind.ESNext}}).outputText;
const {translateDiagnostic}=await import('data:text/javascript;base64,'+Buffer.from(diagnosticSource).toString('base64'));
assert.equal(translateDiagnostic('ja','Completed'),'完了');
assert.equal(translateDiagnostic('zh-CN','Cannot copy file: F:/Music/test.mp3'),'复制文件失败：F:/Music/test.mp3');
assert.equal(translateDiagnostic('en','已完成'),'Completed');
assert.equal(translateDiagnostic('ja','Operation failed: Transfer queue is empty'),'操作に失敗しました：転送キューは空です');
assert.equal(translateDiagnostic('ja','Low-space commit failed; original library restored: injected I/O error'),'省容量の確定に失敗しました。旧曲庫は復元されました: injected I/O error');
console.log(`Translation checks passed: ${count} complete English/Chinese/Japanese messages.`);
