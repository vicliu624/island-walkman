import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { tr } from './i18n';

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
        const settings = Array.from(document.querySelectorAll<HTMLButtonElement>('nav button')).find(button => button.getAttribute('aria-label') === tr('偏好与说明'));
        if (!settings) throw Error('Settings navigation missing');
        settings.click();
        await new Promise(resolve => setTimeout(resolve, 100));
        if (!document.querySelector('main')?.textContent?.includes(tr('只读研究预览'))) throw Error('Read-only firmware card missing');
        const probe = Array.from(document.querySelectorAll<HTMLButtonElement>('main button')).find(button => button.textContent === tr('只读检测 USB'));
        if (!probe || probe.disabled) throw Error('Read-only USB control missing or disabled');
        const response = await fetch('/api/firmware/probe', {method: 'POST', headers: {'Content-Type': 'application/json', 'X-Island-Token': state.token}, body: '{}'});
        if (!response.ok) throw Error('Firmware inventory API failed');
        const firmware = await response.json();
        if (firmware.firmware_write_enabled !== false || firmware.vendor_queries_enabled !== false || firmware.mode !== 'usb_inventory_only') throw Error('Firmware safety boundary changed');
        await report('Read-only firmware card and API available');
    } catch (e) { error = String(e); await report('failed', error); }
    try { await invoke('desktop_smoke_result', { error }); }
    catch (e) { await report('Result IPC failed', String(e)); }
}
