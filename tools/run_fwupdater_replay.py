"""Run fixed offline fault scenarios and preserve source-derived replay evidence.
No player, driver, Sony executable or live application server is accessed.
"""
import argparse
from dataclasses import asdict
import hashlib
import json
from pathlib import Path
from firmware_research.fwupdater_model import Config, Scenario, replay


def build_report():
    scenarios = [
        ('normal', {}, Config()),
        ('missing_source', {'source_kind':'missing'}, Config()),
        ('empty_source', {'source_bytes':0}, Config()),
        ('insufficient_space', {'free_bytes':104}, Config()),
        ('disk_full_during_copy', {'copy_winerror':112}, Config()),
        ('copy_denied', {'copy_winerror':5}, Config()),
        ('copy_crc_error', {'copy_winerror':23}, Config()),
        ('cleanup_masks_copy_disk_full', {'copy_winerror':112,'delete_winerror':5}, Config()),
        ('staged_hash_not_checked_by_legacy', {'staged_hash_matches':False}, Config()),
        ('activation_false_zero_status', {'activation_ok':False}, Config()),
        ('activation_false_accepted_status', {'activation_ok':False,'packed_status':0xff020400}, Config()),
        ('activation_false_bad_status', {'activation_ok':False,'packed_status':0xff030000}, Config()),
        ('activation_no_drive', {'activation_ok':False,'drive_index_valid':False}, Config()),
        ('never_disappears', {'disconnect_at_ms':None}, Config()),
        ('never_reconnects', {'reconnect_at_ms':None}, Config()),
        ('copy_consumes_deadline', {'copy_ms':149000,'disconnect_at_ms':149000,'reconnect_at_ms':None}, Config()),
        ('poll_crosses_deadline', {'reconnect_at_ms':150500}, Config()),
        ('reopen_ten_failures', {'open_failures':10}, Config()),
        ('wrong_snapshot', {'readback_snapshot_matches':False}, Config()),
        ('cleanup_denied_after_100_percent', {'delete_winerror':5}, Config()),
        ('version_query_failed_after_100_percent', {'version_error':0x80047001}, Config()),
        ('wrong_version_after_package_deleted', {'installed_version':'1.02'}, Config()),
        ('stat_failure_ten_retries', {'stat_failures':10}, Config()),
        ('battery_below_explicit_threshold', {'battery_percent':29}, Config.from_timer(battery_threshold=30)),
        ('unknown_raw_battery_255', {'battery_percent':255}, Config.from_timer(battery_threshold=30)),
        ('tick_wrap', {'tick_start':0xffffff00}, Config()),
        ('directory_unmodeled', {'source_kind':'directory'}, Config()),
        ('command_type_1_unmodeled', {}, Config(command_type=1)),
    ]
    results = []
    for name, values, config in scenarios:
        scenario = Scenario(name=name, **values)
        result = replay(scenario, config)
        if result.firmware_write_enabled or result.activation_count > 1:
            raise RuntimeError('Offline replay invariant failed')
        results.append({'scenario':asdict(scenario),'config':asdict(config),'result':asdict(result)})
    root = Path(__file__).resolve().parent
    sources = [root/'firmware_research/fwupdater_model.py',root/'firmware_research/golden_traces.json',root/'test_fwupdater_flow.py']
    return {'evidence_kind':'static_source_derived_symbolic_replay_not_captured_device_trace',
            'sony_code_executed':False,'physical_device_access':False,'hardware_qualification':False,
            'firmware_write_enabled':False,
            'source_sha256':{str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
            'scenario_count':len(results),'cases':results,
            'limitations':['Fake operation durations; blocking OS/SCSI call durations are not reproduced',
                           'MSC regular-file CommandType=0 worker and selected preflight slices only',
                           'No full device installation, rollback or rescue emulation',
                           'No complete MTP/directory/Windows Me branch equivalence claim']}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    report = build_report()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as output:
        json.dump(report, output, indent=2)
        output.write('\n')
    print(json.dumps({'scenario_count':report['scenario_count'],'physical_device_access':False,
                      'firmware_write_enabled':False,'output':str(args.output)},indent=2))
