"""Persist purely symbolic device-install failure observations. No target I/O."""
import argparse
from dataclasses import asdict
import hashlib
import json
from pathlib import Path
from firmware_research.device_install_model import (
    SCRIPT_STEPS, InstallScenario, TailRecord, replay_install_script, flag_write,
    select_tail_in_already_chosen_nvp_block, nvp_bios_read_retry, bios_program_or_erase_status,
)


def report():
    scenarios=[InstallScenario(),InstallScenario(name='old_version',version_text='20070507Z'),
               InstallScenario(name='empty_version_stdout',version_text='')]
    scenarios += [InstallScenario(name='failure_'+step,failed_step=step) for step in SCRIPT_STEPS[1:]
                  if step!='opaque_fwpup_tar_pipeline']
    scenarios += [InstallScenario(name='masked_producer_failure',fwpup_exit=7),
                  InstallScenario(name='tar_failure',tar_exit=2),
                  InstallScenario(name='producer_and_consumer_fail',fwpup_exit=7,tar_exit=2)]
    scenarios += [InstallScenario(name='power_cut_'+step,power_cut_step=step) for step in SCRIPT_STEPS]
    scripts=[]
    for scenario in scenarios:
        result=replay_install_script(scenario)
        if result.firmware_write_enabled or result.rollback_verified or result.independent_recovery_verified:
            raise RuntimeError('Model invented a safety/recovery guarantee')
        scripts.append({'scenario':asdict(scenario),'result':asdict(result)})
    flags=[asdict(flag_write(caller,count))for caller in ['gemini','apl_conn']for count in [-5,0,1,2,3,4]]
    flags.append(asdict(flag_write('apl_conn',4,setting=False)))
    tails=[]
    for name,records in [
        ('new_complete_tail',[TailRecord(0x70,10),TailRecord(0x70,11)]),
        ('last_read_fails',[TailRecord(0x70,10),TailRecord(0x70,11,read_ok=False)]),
        ('erased_after_complete',[TailRecord(0x70,10),TailRecord(0xff,0)]),
        ('metadata_only',[TailRecord(0x70,10),TailRecord(0x70,11,data_matches_expected=False)]),
        ('sequence_wrap',[TailRecord(0x70,0xffffffff),TailRecord(0x70,0)]),
    ]:
        tails.append({'name':name,'records':[asdict(row)for row in records],
                      'selected':select_tail_in_already_chosen_nvp_block(records),
                      'scope':'Second tail scan in an already selected NVP block, not boot recovery'})
    reads=[{'supplied_bios_results':values,'wrapper_result_and_attempts':nvp_bios_read_retry(values)}
           for values in [[0],[-1,0],[1,0],[-1,-1],[1,1]]]
    statuses=[]
    for args in [{},{'status':0xc1},{'status':0x40},{'status':0x40,'panic_context':True},
                 {'completed':False},{'completed':False,'panic_on_timeout':True}]:
        statuses.append({'supplied':args,'result':bios_program_or_erase_status(**args)})
    root=Path(__file__).resolve().parent
    sources=['firmware_research/device_install_model.py','firmware_research/ext2_reader.py',
             'firmware_research/elf_reader.py','test_device_install.py']
    return {'evidence_kind':'source_derived_symbolic_models_not_physical_tests','physical_device_access':False,
            'sony_code_executed':False,'firmware_write_enabled':False,
            'source_sha256':{name:hashlib.sha256((root/name).read_bytes()).hexdigest()for name in sources},
            'script_cases':scripts,'flag_write_cases':flags,'nvp_tail_cases':tails,
            'nvp_read_cases':reads,'bios_status_cases':statuses,
            'scenario_count':len(scripts)+len(flags)+len(tails)+len(reads)+len(statuses),
            'limitations':['Short writes are injected observations; no actual driver short-write path is established',
                           'fwpup internals and boot flag consumption/clearing remain opaque',
                           'No power circuitry, OOB torn writes, ECC or actual NAND storage is emulated',
                           'NVP selected-block tail scan is not the complete partition scanner or boot selector',
                           'Every script result retains unknown post-restart flag/recovery state']}


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args()
    result=report()
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x',encoding='utf-8')as output:
        json.dump(result,output,indent=2)
        output.write('\n')
    print(json.dumps({'scenario_count':result['scenario_count'],'script_cases':len(result['script_cases']),
                      'physical_device_access':False,'firmware_write_enabled':False,'output':str(args.output)},indent=2))
