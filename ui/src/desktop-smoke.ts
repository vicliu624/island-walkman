import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';

declare global { interface Window { __ISLAND_DESKTOP_SMOKE__?: boolean } }

export async function runDesktopSmoke() {
    if (!window.__ISLAND_DESKTOP_SMOKE__) return;
    await new Promise(resolve => setTimeout(resolve, 1500));
    const state = await (await fetch('/api/state')).json();
    const report = async (stage: string, error?: string) => {
        await fetch('/api/desktop-smoke-report', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Island-Token': state.token }, body: JSON.stringify({ passed: false, stage, error }) });
    };
    let error: string | null = null;
    try {
        await report('React loaded');
        const version = await Promise.race([getVersion(), new Promise((_, reject) => setTimeout(() => reject(Error('Native IPC timed out')), 8000))]);
        if (!version) throw Error('Native version unavailable');
        await report('Native IPC available');
        await invoke('get_language');
        await invoke('verify_release_update');
        if (state.device_write_enabled) throw Error('Device writes unexpectedly enabled');
        if (!document.querySelector('h1') || !document.querySelector('button')) throw Error('React controls missing');
    } catch (e) { error = String(e); await report('failed', error); }
    try { await invoke('desktop_smoke_result', { error }); }
    catch (e) { await report('Result IPC failed', String(e)); }
}
