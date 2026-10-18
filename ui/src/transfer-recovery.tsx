import React from 'react';
import { Button, Card, Progress, Title } from 'animal-island-ui';
import { tr, useLanguage, diagnostic } from './i18n';

export type RecoveryJob = {
    running: boolean; percent: number; transfer_phase?: string; resume_available?: boolean;
    needs_review?: boolean; files_done?: number; files_total?: number;
    can_discard?: boolean; commit_in_progress?: boolean;
    message?: string;
};
const phases: Record<string,string> = {
    preparing:'Preparing transfer', copying:'Copying and verifying files', verifying:'Checking saved progress', recovering:'Restoring the connection',
    waiting_device:'Waiting for your WALKMAN', paused:'Transfer paused', needs_review:'Transfer needs attention',
    committing:'Finishing safely', completed:'Transfer complete',
};
export function TransferRecovery({ job, busy, onPause, onDiscard }: {
    job: RecoveryJob; busy: boolean; onPause: () => void; onDiscard: () => void;
}) {
    useLanguage();
    if (!job.transfer_phase && !job.resume_available) return null;
    const phase=job.transfer_phase || 'paused';
    return <Card className="transfer-recovery" role={job.needs_review ? 'alert' : 'status'}>
        <div className="row"><Title size="small" color={job.needs_review ? 'app-orange' : 'app-teal'}>{tr(phases[phase] || 'Transfer paused')}</Title>
            {job.running && <Button size="small" disabled={busy || job.commit_in_progress} onClick={onPause}>{tr('Pause transfer')}</Button>}
            {!job.running && job.resume_available && job.can_discard && <Button size="small" type="text" disabled={busy} onClick={onDiscard}>{tr('Discard task, keep files')}</Button>}
        </div>
        {job.running && <Progress percent={job.percent} aria-label={tr('任务进度')}/>}
        {!!job.files_total && <p>{tr('Files verified: {0} / {1}', job.files_done || 0, job.files_total)}</p>}
        {phase === 'recovering' && <p>{tr('The connection was interrupted. Verified progress is being restored automatically.')}</p>}
        {phase === 'waiting_device' && <p>{tr('Reconnect the same WALKMAN. Saved progress will resume automatically when it is available.')}</p>}
        {phase === 'paused' && <p>{tr('Progress is saved. Resume continues the saved selection; later queue changes are kept for the next transfer.')}</p>}
        {job.needs_review && <p>{tr('Progress and backups are saved. Check the connection and the task details before choosing Retry after checking.')}</p>}
        {job.needs_review && job.message && <p className="file-path">{diagnostic(job.message)}</p>}
        {job.commit_in_progress && <p>{tr('Keep the device connected while the saved library transaction finishes.')}</p>}
        {job.running && !job.commit_in_progress && <p className="muted">{tr('Pausing takes effect after the current file finishes.')}</p>}
        {phase === 'completed' && <p>{tr('Write complete. Safely eject the WALKMAN and verify playback.')}</p>}
    </Card>;
}
