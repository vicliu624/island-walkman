import React, { useState } from 'react';
import { Button, Card, Title } from 'animal-island-ui';
import { open } from '@tauri-apps/plugin-dialog';
import { isTauri } from '@tauri-apps/api/core';
import { tr, useLanguage } from './i18n';

type Probe = { candidates: {vendor_id: string; product_id: string; product: string | null; serial: string | null; bus: string; ports: number[]; speed: string | null}[] };
type Package = {sha256: string; bytes: number; stock_103_exact_match: boolean; components: {index: number; bytes: number; sha256: string}[]};

export function Firmware({ locked, api, run }: {
    locked: boolean;
    api: (path: string, body: unknown) => Promise<unknown>;
    run: (fn: () => Promise<void>) => Promise<void>;
}) {
    useLanguage();
    const [probe, setProbe] = useState<Probe | null>(null);
    const [pkg, setPackage] = useState<Package | null>(null);
    const [name, setName] = useState('');
    async function inspect() {
        const path = await open({multiple: false, filters: [{name: 'Sony UPG', extensions: ['upg', 'UPG']}]});
        if (typeof path !== 'string') return;
        setPackage(null); setName(path);
        setPackage(await api('firmware/inspect', {path}) as Package);
    }
    return <Card><Title size="small" color="app-teal">{tr('设备固件')}</Title>
        <h3>{tr('只读研究预览')}</h3>
        <p>{tr('检查 USB 身份与本地 UPG 文件。此版本不会刷写、重启播放器或运行 Sony 更新器。')}</p>
        <div className="row">
            <Button disabled={locked} onClick={() => void run(async () => {setProbe(null); setProbe(await api('firmware/probe', {}) as Probe);})}>{tr('只读检测 USB')}</Button>
            <Button disabled={locked || !isTauri()} onClick={() => void run(inspect)}>{tr('检查本地固件')}</Button>
        </div>
        {probe && <div role="status"><p>{tr('发现 {0} 台 A91x 系列候选设备', probe.candidates.length)}</p>
            {probe.candidates.map((d, i) => <div key={i}>
                <p>{d.product || tr('未知型号')} · {d.vendor_id}:{d.product_id} · {d.speed || tr('未知速度')}</p>
                <p>{tr('序列号：{0}', d.serial || tr('未提供'))}</p>
                <p>{tr('USB 总线：{0}；端口：{1}', d.bus, d.ports.join('.'))}</p>
            </div>)}
            <p>{tr('USB 标识由 A916/A918/A919 共用，尚未验证具体型号。电量与已安装固件版本尚不可读。')}</p>
        </div>}
        {pkg && <div role="status">
            <p style={{overflowWrap: 'anywhere'}}>{name}</p>
            <p>{tr('容器校验通过：{0} 字节，{1} 个组件', pkg.bytes, pkg.components.length)}</p>
            <p>{pkg.stock_103_exact_match ? tr('与已记录的 Sony 原版 1.03 完全一致') : tr('非已记录的原版包；可能是修改版或未知来源')}</p>
            <p style={{overflowWrap: 'anywhere'}}>SHA-256: {pkg.sha256}</p>
            <p>{tr('MD5 与解密结构校验不等于发布者认证、设备兼容性或刷写许可。')}</p>
        </div>}
    </Card>;
}
