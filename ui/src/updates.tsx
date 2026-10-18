import React, { useEffect, useState } from 'react';
import { Button, Card, Progress, Title } from 'animal-island-ui';
import { getVersion } from '@tauri-apps/api/app';
import { isTauri } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { tr, useLanguage } from './i18n';

export function Updates({ locked, onWorking }: { locked: boolean; onWorking: (value: boolean) => void }) {
    useLanguage();
    const [version, setVersion] = useState('0.2.1');
    const [update, setUpdate] = useState<Update | null>(null);
    const [phase, setPhase] = useState<'idle' | 'checking' | 'current' | 'available' | 'downloading' | 'installing' | 'error'>('idle');
    const [percent, setPercent] = useState(0);
    const [error, setError] = useState('');
    useEffect(() => { if (isTauri()) getVersion().then(setVersion).catch(() => {}); }, []);
    useEffect(() => () => { if (update) void update.close(); }, [update]);
    async function checkUpdate() {
        setPhase('checking'); setError('');
        try { const next = await check(); setUpdate(next); setPhase(next ? 'available' : 'current'); }
        catch (e) { setError(String(e)); setPhase('error'); }
    }
    async function install() {
        if (!update || locked) return;
        setPhase('downloading'); setPercent(0);
        let total = 0, downloaded = 0;
        try {
            await update.downloadAndInstall(event => {
                if (event.event === 'Started') total = event.data.contentLength || 0;
                if (event.event === 'Progress') { downloaded += event.data.chunkLength; if (total) setPercent(Math.min(100, Math.round(downloaded / total * 100))); }
                if (event.event === 'Finished') { setPercent(100); setPhase('installing'); }
            });
            await relaunch();
        } catch (e) { setError(String(e)); setPhase('error'); }
    }
    const working = ['checking', 'downloading', 'installing'].includes(phase);
    useEffect(() => { onWorking(working); return () => onWorking(false); }, [working, onWorking]);
    return <Card><Title size="small" color="app-teal">{tr('软件更新')}</Title>
        <h3>{tr('当前版本 {0}', version)}</h3>
        <p>{tr('更新来自本项目的 GitHub Releases，安装前会验证数字签名。')}</p>
        {phase === 'checking' && <p role="status">{tr('正在检查更新')}</p>}
        {phase === 'current' && <p role="status">{tr('已是最新版本')}</p>}
        {update && <><p>{tr('发现新版本 {0}', update.version)}</p><p className="release-notes">{update.body}</p></>}
        {phase === 'downloading' && <><p role="status">{tr('正在下载更新 {0}%', percent)}</p><Progress percent={percent}/></>}
        {phase === 'installing' && <p role="status">{tr('正在安装更新')}</p>}
        {phase === 'error' && <p role="alert">{tr('更新失败：{0}', error)}</p>}
        <div className="row"><Button disabled={locked || working || !isTauri()} onClick={checkUpdate}>{tr('检查更新')}</Button>
        {update && <Button type="primary" disabled={locked || working} onClick={install}>{tr('下载并安装更新')}</Button>}</div>
        <p>{tr('安装后将重新启动应用。请先完成传输。')}</p>
    </Card>;
}
