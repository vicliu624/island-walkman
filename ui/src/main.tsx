import { runDesktopSmoke } from './desktop-smoke';
import { isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Updates } from './updates';
import { TransferConfirmation } from './transfer-confirmation';
import { tr, useLanguage, diagnostic, restoreDesktopLanguage } from './i18n';
import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Background, Button, Card, Input, Title, Tag, Progress, Checkbox, Drawer, Modal, Switch, Select } from 'animal-island-ui';
import { MusicIcon, LeafIcon, HeadphonesIcon, FolderIcon, SearchIcon, PlayIcon, PauseIcon, ArrowRightIcon, UploadIcon, GridIcon, UserIcon, ListIcon, SettingsIcon, RefreshIcon, EditIcon, CloseIcon, ImageIcon, PlusIcon, CheckCircleIcon, InfoIcon, ChevronLeftIcon } from 'naive-icons';
import 'animal-island-ui/style';
import './style.css';
type Track = {
    key: string;
    path: string;
    title: string;
    artist: string;
    album: string;
    album_artist: string;
    genre: string;
    year: number;
    track: number;
    track_total: number;
    disc: number;
    disc_total: number;
    duration_ms: number;
    bytes: number;
    bitrate: number;
    sample_rate: number;
    cover: string | null;
    tag_version: string;
    warnings: string[];
    device_id: number | null;
};
type Job = {
    running: boolean;
    percent: number;
    message: string;
    logs: string[];
    backup: string | null;
};
type State = {
    tracks: Track[];
    queue: string[];
    device_root: string;
    device_tracks: Track[];
    job: Job;
};
type Entry = {
    name: string;
    path: string;
    directory: boolean;
};
const blank: State = { tracks: [], queue: [], device_root: '', device_tracks: [], job: { running: false, percent: 0, message: tr("\u51C6\u5907\u597D\uFF0C\u628A\u97F3\u4E50\u5E26\u51FA\u95E8\u3002"), logs: [], backup: null } };
const nav = () => [['library', tr("\u6240\u6709\u6B4C\u66F2"), MusicIcon], ['albums', tr("\u4E13\u8F91\u6536\u85CF"), GridIcon], ['artists', tr("\u827A\u672F\u5BB6"), UserIcon], ['device', tr("\u6211\u7684 WALKMAN"), HeadphonesIcon], ['queue', tr("\u4F20\u8F93\u961F\u5217"), ListIcon], ['settings', tr("\u504F\u597D\u4E0E\u8BF4\u660E"), SettingsIcon]] as const;
const tagVersion = (version: string) => version.replace(/Id3v([234])([0-9])/i, 'ID3 v$1.$2');
const time = (ms: number) => `${Math.floor(ms / 60000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, '0')}`;
const size = (bytes: number) => bytes > 1024 ** 3 ? `${(bytes / 1024 ** 3).toFixed(2)} GB` : `${(bytes / 1024 ** 2).toFixed(1)} MB`;
function Cover({ t, large = false, paused = false }: {
    t?: Track;
    large?: boolean;
    paused?: boolean;
}) { return <div className={`cover ${large ? 'large' : ''}`} title={paused && t?.device_id != null ? tr("任务运行时暂停读取设备封面") : undefined}>{t?.cover && !(paused && t.device_id !== null) ? <img src={`/api/cover/${t.key}?v=${encodeURIComponent(t.cover)}`} alt={tr("{0} \u7684\u5C01\u9762", t.album || t.title)}/> : <MusicIcon size={large ? 48 : 22}/>}</div>; }
function App() {
    const [language, setLanguage] = useLanguage();
    const [updating, setUpdating] = useState(false);
    const [s, setS] = useState<State>(blank), [page, setPage] = useState('library'), [q, setQ] = useState(''), [selected, setSelected] = useState<string[]>([]), [sort, setSort] = useState('title'), [filter, setFilter] = useState(''), [resultPage, setResultPage] = useState(1);
    const [error, setError] = useState(''), [notice, setNotice] = useState(''), [edit, setEdit] = useState<Track | null>(null), [busy, setBusy] = useState(false), [roots, setRoots] = useState<string[]>([]), [devicePath, setDevicePath] = useState(''), [deviceWriteEnabled, setDeviceWriteEnabled] = useState(false);
    const [browse, setBrowse] = useState(false), [browseMode, setBrowseMode] = useState<'music' | 'cover'>('music'), [browsePath, setBrowsePath] = useState(''), [parent, setParent] = useState(''), [entries, setEntries] = useState<Entry[]>([]), [picked, setPicked] = useState<string[]>([]);
    const [confirm, setConfirm] = useState(false), [covers, setCovers] = useState(true), [current, setCurrent] = useState<Track | null>(null), [playing, setPlaying] = useState(false), [position, setPosition] = useState(0);
    const token = useRef(''), audio = useRef<HTMLAudioElement>(null), wantPlayback = useRef(false);
    const groupKey = (t: Track) => page === 'artists' ? (t.artist || tr("\u672A\u77E5\u827A\u672F\u5BB6")) : `${t.album_artist || t.artist}\0${t.album}`;
    async function refresh() { const r = await fetch('/api/state'); if (!r.ok)
        throw Error(tr("\u65E0\u6CD5\u8FDE\u63A5\u672C\u5730\u5E94\u7528")); const d = await r.json(); token.current = d.token; setDeviceWriteEnabled(d.device_write_enabled === true); setS(d.state); }
    async function api(path: string, body: unknown) { const r = await fetch(`/api/${path}`, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Island-Token': token.current }, body: JSON.stringify(body) }); const d = await r.json(); if (!r.ok)
        throw Error(d.error || tr("\u64CD\u4F5C\u5931\u8D25")); return d; }
    async function action(fn: () => Promise<void>) { setError(''); setBusy(true); try {
        await fn();
        await refresh();
    }
    catch (e) {
        setError(String(e instanceof Error ? e.message : e));
    }
    finally {
        setBusy(false);
    } }
    async function devices() { const r = await fetch('/api/devices'); if (!r.ok)
        throw Error(tr("\u8BBE\u5907\u68C0\u6D4B\u5931\u8D25")); const d: string[] = await r.json(); setRoots(d); if (d.length) {
        const path = d.includes(s.device_root) ? s.device_root : d[0];
        setDevicePath(path);
        await api('connect', { path });
    } }
    useEffect(() => { let alive = true; refresh().then(() => devices()).then(() => refresh()).catch(e => { if (alive)
        setError(String(e.message)); }); const timer = setInterval(() => refresh().catch(() => { }), 1500); return () => { alive = false; clearInterval(timer); }; }, []);
    useEffect(() => { if (!notice)
        return; const t = setTimeout(() => setNotice(''), 4500); return () => clearTimeout(t); }, [notice]);
    // Drawer 2.2.1 does not expose a close-label prop. Adapt its fixed label
    // after React commits the panel, including a language change while open.
    useEffect(() => {
        for (const button of Array.from(document.querySelectorAll('[role="dialog"] button[aria-label]'))) {
            if (['关闭', 'Close', '閉じる'].includes(button.getAttribute('aria-label') || '')) {
                button.setAttribute('aria-label', tr('关闭'));
            }
        }
    }, [language, edit]);
    const locked = busy || s.job.running || updating;
    const tracksByKey = new Map(s.tracks.map(t => [t.key, t]));
    const source = page === 'device' ? s.device_tracks : page === 'queue' ? s.queue.map(k => tracksByKey.get(k)).filter((t): t is Track => !!t) : s.tracks;
    const visible = source.filter(t => (!filter || groupKey(t) === filter) && `${t.title} ${t.artist} ${t.album} ${t.genre}`.toLowerCase().includes(q.toLowerCase())).sort((a, b) => sort === 'track' ? a.disc - b.disc || a.track - b.track : String(a[sort as keyof Track] || '').localeCompare(String(b[sort as keyof Track] || ''), language, { numeric: true }));
    useEffect(() => setResultPage(1), [page, q, filter, sort]);
    const connected = roots.includes(s.device_root.replaceAll('\\', '/')) || roots.includes(s.device_root);
    async function queue(keys: string[]) { await action(async () => { await api('queue', { keys }); setNotice(tr("\u5DF2\u52A0\u5165\u4F20\u8F93\u961F\u5217")); }); }
    async function browseTo(path: string) { const d = await api('browse', { path }); setBrowsePath(d.path); setParent(d.parent || ''); setEntries(d.entries); setPicked([]); }
    function openBrowse(mode: 'music' | 'cover') {
        if (isTauri()) {
            void action(async () => {
                const result = await open({ multiple: mode === 'music', filters: [{ name: mode === 'music' ? 'MP3' : 'Artwork', extensions: mode === 'music' ? ['mp3'] : ['jpg', 'jpeg', 'png', 'webp'] }] });
                if (!result) return;
                const paths = Array.isArray(result) ? result : [result];
                if (mode === 'cover' && edit) { await api('cover', { key: edit.key, path: paths[0] }); setEdit({ ...edit, cover: paths[0] }); }
                else await api('import', { paths });
            });
            return;
        }
        setBrowseMode(mode); setBrowse(true); void action(() => browseTo(''));
    }
    function importFolder() {
        void action(async () => { const path = await open({ directory: true, multiple: false }); if (path) await api('import', { paths: [path] }); });
    }
    async function importPicked(folder = false) { await action(async () => { if (browseMode === 'cover' && edit) {
        const p = picked[0];
        if (!p)
            throw Error(tr("\u8BF7\u5148\u9009\u62E9\u56FE\u7247"));
        await api('cover', { key: edit.key, path: p });
        setEdit({ ...edit, cover: p });
    }
    else {
        await api('import', { paths: folder ? [browsePath] : picked });
    } setBrowse(false); }); }
    function play(t: Track) { if (t.device_id !== null)
        return; if (current?.key === t.key) {
        audio.current?.play().catch(e => setError(tr("\u8BD5\u542C\u5931\u8D25\uFF1A{0}", e.message)));
        return;
    } wantPlayback.current = true; setPlaying(false); setCurrent(t); setPosition(0); }
    function changePage(p: string) { setPage(p); setQ(''); setFilter(''); setSelected([]); }
    const totalDuration = s.tracks.reduce((n, t) => n + t.duration_ms, 0);
    const groups = new Map<string, Track[]>();
    for (const t of visible) {
        const key = groupKey(t);
        const group = groups.get(key);
        if (group)
            group.push(t);
        else
            groups.set(key, [t]);
    }
    const groupedView = (page === 'albums' || page === 'artists') && !filter;
    const pageSize = groupedView ? 48 : 100;
    const resultCount = groupedView ? groups.size : visible.length;
    const pageCount = Math.max(1, Math.ceil(resultCount / pageSize));
    const activePage = Math.min(resultPage, pageCount);
    const offset = (activePage - 1) * pageSize;
    const pageTracks = visible.slice(offset, offset + pageSize);
    const pageGroups = [...groups].slice(offset, offset + pageSize);
    return <Background type="default" className="app"><aside className="sidebar">
  <div className="brand"><span className="brand-icon"><img src={new URL("../../assets/animal-island-logo.png", import.meta.url).href} alt="Animal Island UI"/></span><div><strong>{tr("\u5531\u7247\u5C0F\u5C9B")}</strong><small>ISLAND WALKMAN</small></div></div>
  <div className="side-label">{tr("\u6211\u7684\u97F3\u4E50\u4E16\u754C")}</div><nav>{nav().map(([id, label, Icon], i) => <React.Fragment key={id}>{i === 3 && <div className="side-label spaced">{tr("\u968F\u8EAB\u542C\u4E0E\u4F20\u8F93")}</div>}<Button type={page === id ? 'primary' : 'text'} aria-label={label} aria-current={page === id ? 'page' : undefined} className={page === id ? 'nav-item active' : 'nav-item'} icon={<Icon size={21}/>} disabled={updating} onClick={() => changePage(id)}><span className="nav-label">{label}</span>{id === 'queue' && s.queue.length > 0 && <span className="count">{s.queue.length}</span>}</Button></React.Fragment>)}</nav>
  <div className="side-bottom"><Card type="dashed"><HeadphonesIcon size={28}/><strong>NW-A918</strong><p>{connected ? tr("\u5DF2\u8FDE\u63A5\uFF0C\u53EF\u4EE5\u51C6\u5907\u4F20\u6B4C") : tr("\u7B49\u5F85\u8FDE\u63A5\u4F60\u7684 WALKMAN")}</p><Tag color={connected ? 'app-teal' : 'default'} variant="soft">{connected ? tr("\u8BBE\u5907\u5DF2\u8FDE\u63A5") : tr("\u79BB\u7EBF\u4E5F\u80FD\u6574\u7406\u66F2\u5E93")}</Tag></Card><small>{tr("\u5728\u5C0F\u5C9B\u6574\u7406\uFF0C\u5E26\u7740\u97F3\u4E50\u51FA\u53D1\u3002")}</small></div>
 </aside><div className="workspace"><header className="topbar"><label className="language-picker"><span>{tr("Language")}</span><select aria-label="Language" value={language} onChange={event => setLanguage(event.target.value as "en" | "zh-CN" | "ja")}><option value="en">English</option><option value="zh-CN">中文</option><option value="ja">日本語</option></select></label><span>{tr("\u4F60\u7684\u968F\u8EAB\u97F3\u4E50\u6816\u606F\u5730")}</span><div className="row"><Tag color="app-teal" variant="soft">{tr("\u672C\u5730\u8FD0\u884C")}</Tag><Button size="small" type="text" icon={<RefreshIcon size={18}/>} disabled={locked} onClick={() => action(devices)}>{tr("\u5237\u65B0\u8BBE\u5907")}</Button></div></header>
 <main><div className="page-heading"><div><div className="eyebrow">A LITTLE ISLAND, A LOT OF MUSIC</div><h1>{nav().find(n => n[0] === page)?.[1]}</h1><p>{page === 'queue' ? tr("\u6574\u7406\u597D\u884C\u56CA\uFF0C\u628A\u559C\u6B22\u7684\u6B4C\u88C5\u8FDB\u968F\u8EAB\u542C\u3002") : page === 'device' ? tr("\u770B\u770B\u968F\u8EAB\u542C\u91CC\u7684\u6536\u85CF\uFF0C\u51C6\u5907\u4E0B\u4E00\u6BB5\u65C5\u7A0B\u3002") : page === 'settings' ? tr("\u8BA9\u6BCF\u4E00\u9996\u6B4C\uFF0C\u4EE5\u5408\u9002\u7684\u65B9\u5F0F\u62B5\u8FBE\u3002") : tr("\u6162\u6162\u6536\u85CF\uFF0C\u6BCF\u4E00\u9996\u90FD\u6709\u81EA\u5DF1\u7684\u6545\u4E8B\u3002")}</p></div><div className="row">{isTauri() && <Button icon={<FolderIcon size={20}/>} disabled={locked} onClick={importFolder}>{tr("导入文件夹")}</Button>}<Button type="primary" icon={<PlusIcon size={20}/>} disabled={locked} onClick={() => openBrowse('music')}>{tr("\u5BFC\u5165\u97F3\u4E50")}</Button></div></div>
 {error && <div className="alert error" role="alert"><InfoIcon size={20}/><span>{diagnostic(error)}</span><Button type="text" size="small" aria-label={tr("\u5173\u95ED\u9519\u8BEF")} onClick={() => setError('')}><CloseIcon size={16}/></Button></div>}
 {notice && <div className="alert success" role="status"><CheckCircleIcon size={20}/>{notice}</div>}
 {page === 'settings' ? <div className="settings"><Updates locked={busy || s.job.running} onWorking={setUpdating}/><Card><Title size="small" color="app-teal">{tr("\u97F3\u4E50\u4E0E\u6807\u7B7E")}</Title><h3>{tr("\u5B8C\u6574\u4FDD\u7559\uFF0C\u6309\u8BBE\u5907\u80FD\u529B\u5448\u73B0")}</h3><p>{tr("\u8BFB\u53D6 ID3 v2.2\u3001v2.3\u3001v2.4\u3002\u4E13\u8F91\u827A\u672F\u5BB6\u3001\u66F2\u53F7\uFF0F\u603B\u66F2\u6570\u3001\u789F\u53F7\uFF0F\u603B\u789F\u6570\u548C\u5E74\u4EFD\u5206\u522B\u4FDD\u5B58\u3002\u6807\u7B7E\u4FEE\u6539\u4FDD\u5B58\u5728\u5C0F\u5C9B\u66F2\u5E93\uFF0C\u4E0D\u6539\u5199\u539F\u59CB\u6B4C\u66F2\u3002")}</p><p>{tr("\u8BBE\u5907\u5217\u8868\u5B57\u6BB5\u6700\u591A\u4F7F\u7528 60 \u4E2A UTF-16 \u5355\u4F4D\uFF0C\u9884\u7559\u7ED3\u675F\u7B26\uFF1B\u4E2D\u6587\u548C\u7279\u6B8A\u5B57\u7B26\u4E0D\u4F1A\u4ECE\u4E2D\u95F4\u5207\u65AD\u3002\u5B8C\u6574\u6587\u672C\u5199\u5165 OMA \u6807\u7B7E\u3002")}</p></Card><Card><Title size="small" color="app-yellow">{tr("\u4F20\u8F93\u4E0E\u5907\u4EFD")}</Title><h3>{tr("\u5148\u51C6\u5907\uFF0C\u72EC\u7ACB\u9A8C\u8BC1\u540E\u518D\u5F00\u653E\u5199\u5165")}</h3><p>{tr("\u8BBE\u5907\u5199\u5165\u9ED8\u8BA4\u5173\u95ED\uFF1BSony \u683C\u5F0F\u72EC\u7ACB\u9A8C\u8BC1\u5B8C\u6210\u524D\uFF0C\u4EC5\u5728\u663E\u5F0F\u542F\u7528\u5B9E\u9A8C\u6A21\u5F0F\u65F6\u5141\u8BB8\u6D4B\u8BD5\u4F20\u8F93\u3002")}</p><p>{tr("\u672C\u5730\u751F\u6210\u97F3\u9891\u4E0E\u6570\u636E\u5E93\uFF0C\u56DE\u8BFB\u9A8C\u8BC1\u3001\u5907\u4EFD\u65E7\u66F2\u5E93\u3001\u9010\u6587\u4EF6\u6821\u9A8C\uFF0C\u518D\u63D0\u4EA4\u5230\u8BBE\u5907\u3002\u672C\u5730\u4FDD\u5B58\u6821\u9A8C\u5907\u4EFD\uFF0C\u8BBE\u5907\u4E0A\u7684 ISLAND-BACKUP-* \u76EE\u5F55\u4FDD\u7559\u539F\u66F2\u5E93\uFF0C\u6682\u4E0D\u81EA\u52A8\u6E05\u7406\u3002")}</p><p>{tr("\u672C\u7248\u76F4\u63A5\u5BFC\u5165 MP3\u3002FLAC\u3001WAV \u7B49\u683C\u5F0F\u8BF7\u5148\u8F6C\u6210 MP3\u3002\u6B64\u7248\u672C\u4E0D\u4F1A\u79FB\u9664\u8BBE\u5907\u4E2D\u7684\u5DF2\u6709\u6B4C\u66F2\u3002")}</p></Card><Card><Title size="small" color="app-pink">{tr("\u5C01\u9762\u517C\u5BB9\u6027")}</Title><h3>{tr("\u5C01\u9762\u663E\u793A\u5DF2\u901A\u8FC7 A918 \u5B9E\u673A\u9A8C\u8BC1")}</h3><p>{tr("\u4F18\u5148\u4F7F\u7528 ID3 \u7684\u6B63\u9762\u5C01\u9762\uFF0C\u652F\u6301 JPEG\u3001PNG\u3001WebP\u3002\u53EF\u624B\u52A8\u66FF\u6362\u5C01\u9762\uFF0C\u539F\u59CB\u6587\u4EF6\u4E0D\u53D8\u3002\u5DF2\u901A\u8FC7 Sony \u56FA\u4EF6\u4E0E SonicStage \u7684 REA \u5206\u6790\u5B9A\u4F4D\u5C01\u9762\u7D22\u5F15\u89C4\u5219\uFF0C\u5B9E\u673A\u8865\u4E01\u5DF2\u663E\u793A\u5C01\u9762\u3002\u5B8C\u6574\u65B0\u7248\u4F20\u8F93\u4ECD\u5728\u9A8C\u8BC1\u3002")}</p><div className="setting-switch"><span>{tr("\u4F20\u8F93\u4E13\u8F91\u5C01\u9762")}</span><Switch checked={covers} disabled={locked} onChange={setCovers}/></div></Card><Card><h3>{tr("\u5173\u4E8E\u5531\u7247\u5C0F\u5C9B \u00B7 0.2.2")}</h3><p>{tr("Rust \u97F3\u9891\u4E0E\u4F20\u8F93\u6838\u5FC3 \u00B7 React + Animal Island UI\u3002\u9762\u5411 Windows \u4E0E Linux\uFF1B\u5DF2\u901A\u8FC7 Debian 12 \u672C\u5730\u8FD0\u884C\u4E0E\u66F2\u5E93\u751F\u6210\u9A8C\u8BC1\uFF0CUSB \u5B9E\u673A\u4F20\u8F93\u4ECD\u5728\u9A8C\u8BC1\u3002\u521D\u7248 OMGAUDIO \u5B9E\u73B0\u53C2\u8003 JSymphonic\uFF08GPL-3.0-or-later\uFF09\uFF1B\u6B63\u5728\u4F7F\u7528 REA \u5BF9 Sony \u4E8C\u8FDB\u5236\u72EC\u7ACB\u9A8C\u8BC1\u534F\u8BAE\uFF0C\u5C1A\u4E0D\u80FD\u4FDD\u8BC1\u5B8C\u6574\u517C\u5BB9\u3002\u8FD0\u884C\u65F6\u4E0D\u9700\u8981 Java\u3002")}</p></Card></div> : <>
 {(page === 'library' || page === 'albums' || page === 'artists') && <section className="stats"><Card><span className="stat-icon"><MusicIcon size={25}/></span><div><strong>{s.tracks.length}<small>{tr("\u9996")}</small></strong><p>{tr("\u5C0F\u5C9B\u6536\u85CF")}</p></div></Card><Card><span className="stat-icon peach"><GridIcon size={25}/></span><div><strong>{new Set(s.tracks.map(t => `${t.album_artist || t.artist}\0${t.album}`)).size}<small>{tr("\u5F20")}</small></strong><p>{tr("\u73CD\u85CF\u4E13\u8F91")}</p></div></Card><Card><span className="stat-icon lilac"><HeadphonesIcon size={25}/></span><div><strong>{Math.round(totalDuration / 60000)}<small>{tr("\u5206\u949F")}</small></strong><p>{tr("\u966A\u4F34\u4F60\u7684\u65F6\u5149")}</p></div></Card></section>}
 {page === 'device' && <Card className="device-panel"><div className="row"><HeadphonesIcon size={43}/><div><h2>SONY WALKMAN · NW-A918</h2><p>{s.device_tracks.length}{tr(" \u9996\u6B4C\u66F2 \u00B7 ")}{size(s.device_tracks.reduce((n, t) => n + t.bytes, 0))}{tr(" \u97F3\u9891")}</p></div><Tag color={connected ? 'app-teal' : 'app-orange'}>{connected ? tr("\u5DF2\u8FDE\u63A5") : tr("\u8BF7\u8FDE\u63A5\u8BBE\u5907")}</Tag></div><div className="row device-input"><Input aria-label={tr("\u8BBE\u5907\u6839\u76EE\u5F55")} placeholder={tr("Windows: F:/  \u00B7  Debian: /media/\u7528\u6237\u540D/WALKMAN")} value={devicePath} onChange={e => setDevicePath(e.target.value)}/><Button disabled={locked} onClick={() => action(async () => { await api('connect', { path: devicePath }); setRoots([...roots, devicePath]); })}>{tr("\u8FDE\u63A5\u8BBE\u5907")}</Button></div></Card>}
 {page === 'queue' && !deviceWriteEnabled && <div className="alert success" role="status"><InfoIcon size={20}/><span>{tr('Experimental transfers are available. Choose Start transfer, then confirm this transfer in the dialog. Local preparation does not write to the player.')}</span></div>}
 {page === 'queue' && <Card className="queue-summary"><div><Title size="small" color="app-yellow">{tr("\u97F3\u4E50\u51FA\u53D1\u7AD9")}</Title><h2>{s.queue.length ? tr("{0} \u9996\u6B4C\uFF0C\u51C6\u5907\u767B\u5C9B", s.queue.length) : tr("\u884C\u56CA\u8FD8\u662F\u7A7A\u7684")}</h2><p>{s.queue.length ? tr("\u4F20\u8F93\u524D\u5907\u4EFD\u8BBE\u5907\u66F2\u5E93\u3002\u6B4C\u66F2\u4EC5\u8FFD\u52A0\uFF0C\u4E0D\u4F1A\u9690\u5F0F\u8986\u76D6\u8BBE\u5907\u4E2D\u7684\u540C\u540D\u6B4C\u66F2\u3002") : tr("\u5728\u66F2\u5E93\u4E2D\u9009\u62E9\u6B4C\u66F2\uFF0C\u70B9\u51FB\u201C\u52A0\u5165\u4F20\u8F93\u961F\u5217\u201D\u3002")}</p></div><div className="row"><Button disabled={locked || !s.queue.length} icon={<FolderIcon size={20}/>} onClick={() => action(async () => { await api('prepare', {}); setNotice(tr("\u5DF2\u5F00\u59CB\u672C\u5730\u9884\u6F14\uFF0C\u8F93\u51FA\u4E0E\u9A8C\u8BC1\u7ED3\u679C\u89C1\u4EFB\u52A1\u8BB0\u5F55")); })}>{tr("\u672C\u5730\u9884\u6F14")}</Button><Button type="primary" disabled={locked || !s.queue.length || !connected} icon={<UploadIcon size={21}/>} onClick={() => setConfirm(true)}>{tr("\u5F00\u59CB\u4F20\u8F93")}</Button></div></Card>}
 {s.job.running && <Card className="job-card"><div className="row"><strong>{diagnostic(s.job.message)}</strong><Tag color="app-teal">{tr("\u5904\u7406\u4E2D")}</Tag></div><Progress percent={s.job.percent} aria-label={tr("\u4EFB\u52A1\u8FDB\u5EA6")}/></Card>}
 {!s.job.running && s.job.logs.length > 0 && page === 'queue' && <Card className="job-card"><strong>{diagnostic(s.job.message)}</strong><details><summary>{tr("\u67E5\u770B\u4F20\u8F93\u8BB0\u5F55")}</summary>{s.job.logs.map((l, i) => <p key={i}>{diagnostic(l)}</p>)}{s.job.backup && <p>{tr("\u672C\u6B21\u4EFB\u52A1\u8F93\u51FA\u76EE\u5F55\uFF1A")}{s.job.backup}</p>}</details></Card>}
 <section className="collection"><div className="collection-toolbar"><div className="row"><Title size="small" color="app-teal">{(filter ? (groups.get(filter)?.[0]?.[page === 'artists' ? 'artist' : 'album'] || tr("\u97F3\u4E50\u6536\u85CF")) : '') || `${page === 'device' ? tr("\u8BBE\u5907\u6B4C\u66F2") : page === 'queue' ? tr("\u7B49\u5F85\u51FA\u53D1") : tr("\u97F3\u4E50\u6536\u85CF")}`}</Title><span className="muted">{visible.length}{tr(" \u9996")}</span>{filter && <Button size="small" type="text" onClick={() => setFilter('')}>{tr("\u5168\u90E8\u6536\u85CF")}</Button>}</div><div className="row tools"><Input aria-label={tr("\u641C\u7D22\u66F2\u5E93")} prefix={<SearchIcon size={18}/>} placeholder={tr("\u641C\u7D22\u6B4C\u66F2\u3001\u4E13\u8F91\u3001\u827A\u672F\u5BB6")} value={q} onChange={e => setQ(e.target.value)} allowClear/><Select aria-label={tr("\u6392\u5E8F\u65B9\u5F0F")} value={sort} options={[{ key: 'title', label: tr("\u6309\u66F2\u540D\u6392\u5E8F") }, { key: 'artist', label: tr("\u6309\u827A\u672F\u5BB6\u6392\u5E8F") }, { key: 'album', label: tr("\u6309\u4E13\u8F91\u6392\u5E8F") }, { key: 'track', label: tr("\u6309\u66F2\u53F7\u6392\u5E8F") }]} onChange={setSort}/></div></div>
 {!groupedView && visible.length > 100 && <p className="pagination-hint muted">{tr("\u8868\u5934\u590D\u9009\u6846\u9009\u62E9\u5F53\u524D\u9875\u6B4C\u66F2\uFF1B\u7FFB\u9875\u540E\u4FDD\u7559\u5DF2\u9009\u6B4C\u66F2\u3002")}</p>}
 {selected.length > 0 && page !== 'device' && page !== 'queue' && <div className="selection"><span>{tr("\u5DF2\u9009 ")}{selected.length}{tr(" \u9996")}</span><Button size="small" type="primary" disabled={locked} icon={<ArrowRightIcon size={16}/>} onClick={() => queue(selected)}>{tr("\u52A0\u5165\u4F20\u8F93\u961F\u5217")}</Button><Button type="text" size="small" onClick={() => setSelected([])}>{tr("\u53D6\u6D88\u9009\u62E9")}</Button></div>}
 {(page === 'albums' || page === 'artists') && !filter ? <div className="album-grid">{pageGroups.map(([name, ts]) => <Card key={name} hoverable><div role="button" tabIndex={0} onClick={() => setFilter(name)} onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            setFilter(name);
        } }}><Cover paused={s.job.running} t={ts[0]} large/><h3>{page === 'artists' ? (ts[0].artist || tr("\u672A\u77E5\u827A\u672F\u5BB6")) : (ts[0].album || tr("\u672A\u547D\u540D\u4E13\u8F91"))}</h3><p>{page === 'albums' ? (ts[0].album_artist || ts[0].artist || tr("\u672A\u77E5\u827A\u672F\u5BB6")) : tr("{0} \u5F20\u4E13\u8F91", new Set(ts.map(t => t.album)).size)} · {ts.length}{tr(" \u9996")}</p></div><Button type="text" size="small" disabled={locked} onClick={() => queue(ts.map(t => t.key))}>{tr("\u6574\u7EC4\u52A0\u5165\u961F\u5217")}</Button></Card>)}</div> : <div className="table-wrap"><table><thead><tr><th className="check-cell">{page !== 'device' && page !== 'queue' && <Checkbox options={[{ value: 'all', label: <span className="sr-only">{tr("\u9009\u62E9\u5F53\u524D\u9875\u6B4C\u66F2")}</span> }]} value={pageTracks.length > 0 && pageTracks.every(t => selected.includes(t.key)) ? ['all'] : []} onChange={v => setSelected(v.length ? [...new Set([...selected, ...pageTracks.map(t => t.key)])] : selected.filter(k => !pageTracks.some(t => t.key === k)))} aria-label={tr("\u9009\u62E9\u5F53\u524D\u9875\u6B4C\u66F2")}/>}</th><th>{tr("\u6B4C\u66F2")}</th><th>{tr("\u4E13\u8F91")}</th><th>{tr("\u683C\u5F0F / \u6807\u7B7E")}</th><th>{tr("\u65F6\u957F")}</th><th>{tr("\u64CD\u4F5C")}</th></tr></thead><tbody>{pageTracks.map((t, index) => <tr key={t.key}><td className="check-cell">{page === 'device' || page === 'queue' ? <span className="muted">{offset + index + 1}</span> : <Checkbox options={[{ value: t.key, label: <span className="sr-only">{tr("\u9009\u62E9\u6B4C\u66F2 ")}{t.title || tr("\u672A\u547D\u540D\u6B4C\u66F2")}，{t.artist || tr("\u672A\u77E5\u827A\u672F\u5BB6")}</span> }]} value={selected.includes(t.key) ? [t.key] : []} onChange={v => setSelected(v.length ? [...selected, t.key] : selected.filter(k => k !== t.key))}/>}</td><td><div className="song-cell"><Cover paused={s.job.running} t={t}/><div><strong>{t.title || tr("\u672A\u547D\u540D\u6B4C\u66F2")}</strong><p>{t.artist || tr("\u672A\u77E5\u827A\u672F\u5BB6")}</p></div></div></td><td><span className="album-name">{t.album || tr("\u672A\u547D\u540D\u4E13\u8F91")}</span><small>{t.year > 0 ? t.year : ''}{t.track > 0 ? tr(" \u00B7 \u7B2C {0} \u66F2", t.track) : ''}</small></td><td><Tag color="app-teal" variant="soft">{t.device_id ? 'OMA' : 'MP3'}</Tag><small>{t.device_id ? tr("\u8BBE\u5907\u66F2\u5E93") : `${t.bitrate} kbps · ${tagVersion(t.tag_version)}`}</small></td><td className="duration">{time(t.duration_ms)}</td><td><div className="row compact">{!t.device_id && <><Button type="text" size="small" aria-label={tr("\u64AD\u653E {0}", t.title)} onClick={() => play(t)}><PlayIcon size={18}/></Button><Button type="text" size="small" aria-label={tr("\u7F16\u8F91 {0}", t.title)} disabled={locked} onClick={() => setEdit({ ...t })}><EditIcon size={18}/></Button>{page === 'queue' ? <Button type="text" size="small" disabled={locked} aria-label={tr("\u79FB\u51FA\u961F\u5217 {0}", t.title)} onClick={() => action(async () => { await api('queue/remove', { key: t.key }); })}><CloseIcon size={18}/></Button> : <Button type="text" size="small" disabled={locked || s.queue.includes(t.key)} aria-label={tr("\u52A0\u5165\u961F\u5217 {0}", t.title)} onClick={() => queue([t.key])}><PlusIcon size={18}/></Button>}</>}</div></td></tr>)}</tbody></table></div>}
 {resultCount > pageSize && <div className="pagination-bar"><span className="muted">{tr("\u5171 ")}{resultCount} {groupedView ? tr("\u7EC4") : tr("\u9996")}{tr(" \u00B7 \u6BCF\u9875 ")}{pageSize} {groupedView ? tr("\u7EC4") : tr("\u9996")}</span><div className="row"><Button size="small" disabled={activePage <= 1} onClick={() => setResultPage(activePage - 1)}>{tr("\u4E0A\u4E00\u9875")}</Button><span aria-live="polite">{activePage} / {pageCount}</span><Button size="small" disabled={activePage >= pageCount} onClick={() => setResultPage(activePage + 1)}>{tr("\u4E0B\u4E00\u9875")}</Button></div></div>}
 {visible.length === 0 && <div className="empty"><MusicIcon size={48}/><h3>{q ? tr("\u8FD8\u6CA1\u6709\u627E\u5230\u8FD9\u9996\u6B4C") : page === 'device' ? tr("\u8BBE\u5907\u66F2\u5E93\u7B49\u5F85\u8FDE\u63A5") : tr("\u5C0F\u5C9B\u7B49\u5F85\u7B2C\u4E00\u9996\u6B4C")}</h3><p>{q ? tr("\u8BD5\u8BD5\u5176\u4ED6\u66F2\u540D\u3001\u4E13\u8F91\u6216\u827A\u672F\u5BB6\u3002") : tr("\u5BFC\u5165\u672C\u5730 MP3\uFF0C\u6536\u85CF\u4F60\u7684\u97F3\u4E50\u3002")}</p>{!q && page !== 'device' && <Button disabled={locked} icon={<FolderIcon size={18}/>} onClick={() => openBrowse('music')}>{tr("\u6311\u9009\u97F3\u4E50")}</Button>}</div>}
 </section></>}
 <div className="footnote"><LeafIcon size={15}/><span>{tr("\u97F3\u4E50\u7559\u5728\u4F60\u7684\u7535\u8111\u91CC\uFF0C\u5FEB\u4E50\u53EF\u4EE5\u968F\u8EAB\u643A\u5E26\u3002")}</span><span>ISLAND WALKMAN · 0.2.2</span></div></main>
 <footer className="player"><div className="now-playing"><Cover paused={s.job.running} t={current || undefined}/><div><strong>{current?.title || tr("\u4ECA\u5929\uFF0C\u60F3\u542C\u70B9\u4EC0\u4E48\uFF1F")}</strong><small>{current?.artist || tr("\u9009\u4E00\u9996\u6B4C\uFF0C\u5728\u5C0F\u5C9B\u6B47\u4E00\u4F1A\u513F")}</small></div></div><div className="playback"><Button type="primary" size="small" aria-label={playing ? tr("\u6682\u505C") : tr("\u64AD\u653E")} disabled={!current} onClick={() => { if (playing)
        audio.current?.pause();
    else
        audio.current?.play().catch(e => setError(e.message)); }}>{playing ? <PauseIcon size={20}/> : <PlayIcon size={20}/>}</Button><span>{time(position)}</span><Progress percent={current && current.duration_ms > 0 ? Math.min(100, position / current.duration_ms * 100) : 0} showInfo={false} size="small" aria-label={tr("\u8BD5\u542C\u8FDB\u5EA6")}/><span>{time(current?.duration_ms || 0)}</span></div><Tag color="app-yellow" variant="soft">{tr("\u539F\u59CB MP3 \u8BD5\u542C")}</Tag><audio ref={audio} src={current ? `/api/audio/${current.key}` : undefined} onCanPlay={() => { if (wantPlayback.current) {
        wantPlayback.current = false;
        audio.current?.play().catch(e => setError(tr("\u8BD5\u542C\u5931\u8D25\uFF1A{0}", e.message)));
    } }} onError={() => { wantPlayback.current = false; setPlaying(false); setError(tr("\u65E0\u6CD5\u8BD5\u542C\u6B64\u97F3\u9891\uFF0C\u8BF7\u786E\u8BA4\u6587\u4EF6\u5B58\u5728\u4E14\u6D4F\u89C8\u5668\u652F\u6301\u8BE5 MP3\u3002")); }} onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onEnded={() => setPlaying(false)} onTimeUpdate={() => setPosition((audio.current?.currentTime || 0) * 1000)}/></footer>
 </div>
 <Drawer open={!!edit} onClose={() => setEdit(null)} title={tr("\u6574\u7406\u8FD9\u9996\u6B4C")} width={470} footer={<div className="row"><Button onClick={() => setEdit(null)}>{tr("\u53D6\u6D88")}</Button><Button type="primary" disabled={locked} onClick={() => action(async () => { await api('edit', { track: edit }); setEdit(null); setNotice(tr("\u6807\u7B7E\u5DF2\u4FDD\u5B58\u5230\u672C\u5730\u66F2\u5E93\uFF0C\u539F\u59CB\u6587\u4EF6\u4FDD\u6301\u5B8C\u6574")); })}>{tr("\u4FDD\u5B58\u6807\u7B7E")}</Button></div>}>
 {edit && <div className="editor"><div className="editor-cover"><Cover paused={s.job.running} t={edit} large/><div><Tag color="app-teal">{tagVersion(edit.tag_version)}</Tag><p>{edit.bitrate} kbps · {(edit.sample_rate / 1000).toFixed(1)} kHz</p><Button size="small" icon={<ImageIcon size={16}/>} disabled={locked} onClick={() => openBrowse('cover')}>{tr("\u66F4\u6362\u5C01\u9762")}</Button></div></div><p className="muted">{tr("\u7528\u4E8E\u66F2\u5E93\u4E0E\u4E0B\u4E00\u6B21\u4F20\u8F93\uFF0C\u4E0D\u6539\u5199\u6E90 MP3\u3002")}</p>{([['title', tr("\u66F2\u540D")], ['artist', tr("\u827A\u672F\u5BB6")], ['album', tr("\u4E13\u8F91")], ['album_artist', tr("\u4E13\u8F91\u827A\u672F\u5BB6")], ['genre', tr("\u6D41\u6D3E")]] as const).map(([field, label]) => <label key={field}>{label}<Input aria-label={label} value={edit[field]} onChange={e => setEdit({ ...edit, [field]: e.target.value })}/></label>)}<div className="field-grid">{([['year', tr("\u5E74\u4EFD")], ['track', tr("\u66F2\u53F7")], ['track_total', tr("\u603B\u66F2\u6570")], ['disc', tr("\u789F\u53F7")], ['disc_total', tr("\u603B\u789F\u6570")]] as const).map(([field, label]) => <label key={field}>{label}<Input aria-label={label} type="number" min={0} max={field === 'year' ? 9999 : 65535} value={edit[field]} onChange={e => setEdit({ ...edit, [field]: Math.max(0, Number(e.target.value)) })}/></label>)}</div>{edit.warnings.map(w => <p className="warning" key={w}>{diagnostic(w)}</p>)}<details><summary>{tr("\u6587\u4EF6\u4F4D\u7F6E")}</summary><p className="file-path">{edit.path}</p></details></div>}
 </Drawer>
 <Modal open={browse} title={browseMode === 'music' ? tr("\u5E26\u4E00\u4E9B\u97F3\u4E50\u56DE\u5C0F\u5C9B") : tr("\u6311\u9009\u4E00\u5F20\u5C01\u9762")} width={710} typewriter={false} onClose={() => setBrowse(false)} footer={<div className="row"><span className="muted">{tr("\u5DF2\u9009 ")}{picked.length}{tr(" \u4E2A\u6587\u4EF6")}</span>{browseMode === 'music' && <Button disabled={locked || !browsePath} onClick={() => importPicked(true)}>{tr("\u5BFC\u5165\u5F53\u524D\u6587\u4EF6\u5939")}</Button>}<Button type="primary" disabled={locked || !picked.length} onClick={() => importPicked(false)}>{browseMode === 'music' ? tr("\u5BFC\u5165\u6240\u9009\u6B4C\u66F2") : tr("\u4F7F\u7528\u8FD9\u5F20\u5C01\u9762")}</Button></div>}>
 <div className="row browse-location"><Button size="small" aria-label={tr("\u4E0A\u4E00\u7EA7\u76EE\u5F55")} disabled={!parent || locked} onClick={() => action(() => browseTo(parent))}><ChevronLeftIcon size={18}/></Button><Input aria-label={tr("\u6D4F\u89C8\u76EE\u5F55")} value={browsePath} onChange={e => setBrowsePath(e.target.value)} onKeyDown={e => { if (e.key === 'Enter')
        action(() => browseTo(browsePath)); }}/><Button disabled={locked} onClick={() => action(() => browseTo(browsePath))}>{tr("\u524D\u5F80")}</Button></div><div className="file-list">{entries.filter(e => e.directory || (browseMode === 'music' ? e.name.toLowerCase().endsWith('.mp3') : /\.(jpg|jpeg|png|webp)$/i.test(e.name))).map(e => <div className="file-entry" key={e.path}>{e.directory ? <Button type="text" icon={<FolderIcon size={20}/>} onClick={() => action(() => browseTo(e.path))}>{e.name}</Button> : <Checkbox options={[{ value: e.path, label: e.name }]} value={picked.includes(e.path) ? [e.path] : []} onChange={v => setPicked(v.length ? (browseMode === 'cover' ? [e.path] : [...picked, e.path]) : picked.filter(p => p !== e.path))}/>}</div>)}</div>
 </Modal>
 {confirm && <TransferConfirmation count={s.queue.length} root={s.device_root} locked={locked} connected={connected} writeEnabled={deviceWriteEnabled} covers={covers} onCovers={setCovers} onClose={() => setConfirm(false)} onTransfer={experimental_confirmed => action(async () => { await api('transfer', { covers, experimental_confirmed }); setConfirm(false); setPage('queue'); })}/>}
 </Background>;
}
createRoot(document.getElementById('root')!).render(<App />);

void restoreDesktopLanguage();

void runDesktopSmoke();
