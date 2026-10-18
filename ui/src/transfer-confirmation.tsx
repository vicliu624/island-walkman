import React, { useState } from 'react';
import { Button, Checkbox, Modal, Switch } from 'animal-island-ui';
import { tr, useLanguage } from './i18n';

export function TransferConfirmation({ count, root, locked, connected, writeEnabled, covers, onCovers, onClose, onTransfer }: {
    count: number; root: string; locked: boolean; connected: boolean; writeEnabled: boolean; covers: boolean;
    onCovers: (value: boolean) => void; onClose: () => void; onTransfer: (confirmed: boolean) => void;
}) {
    useLanguage();
    const [confirmed, setConfirmed] = useState(false);
    return <Modal open title={tr('让音乐出发吧')} variant="game" typewriter={false} onClose={onClose}
        footer={<div className="row"><Button onClick={onClose}>{tr('再整理一下')}</Button>
            <Button type="primary" disabled={locked || !connected || count === 0 || (!writeEnabled && !confirmed)}
                onClick={() => onTransfer(confirmed)}>{tr('备份并开始传输')}</Button></div>}>
        <p>{tr('将 ')}{count}{tr(' 首歌曲传到 ')}{root}{tr('。应用会保留旧曲库，校验新文件后再提交。')}</p>
        {!writeEnabled && <div className="experimental-transfer" role="group" aria-label={tr('Experimental transfer')}>
            <p>{tr('Full A918 transfer validation is incomplete. The application backs up the existing library before writing. If an I/O error occurs, stop and inspect the connection before trying again.')}</p>
            <Checkbox options={[{ value: 'confirmed', label: tr('I understand and allow this experimental transfer.') }]}
                value={confirmed ? ['confirmed'] : []} disabled={locked}
                onChange={values => setConfirmed(values.includes('confirmed'))}/>
        </div>}
        <div className="setting-switch"><span>{tr('传输专辑封面')}</span><Switch checked={covers} disabled={locked} onChange={onCovers}/></div>
        <p className="muted">{tr('传输过程中保持设备连接。完成后请安全弹出，再验证播放器上的显示与播放。')}</p>
    </Modal>;
}
