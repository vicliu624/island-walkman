import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const result=spawnSync(process.platform==='win32'?'python':'python3',[fileURLToPath(new URL('./version.py',import.meta.url)),'--check'],{stdio:'inherit'});
if(result.error)throw result.error;
process.exit(result.status??1);
