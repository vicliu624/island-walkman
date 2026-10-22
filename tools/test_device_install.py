"""Offline device-install boundary and in-memory image-reader tests only."""
import ast
import hashlib
import json
from pathlib import Path
import struct
import unittest
from firmware_research.device_install_model import (
    SCRIPT_STEPS, InstallScenario, TailRecord, flag_write, replay_install_script,
    select_tail_in_already_chosen_nvp_block, zone_writer_returncode_slice,
    application_fallback, nvp_bios_read_retry, bios_program_or_erase_status,
)
from firmware_research.ext2_reader import Ext2Image
from firmware_research.elf_reader import Elf32


def synthetic_ext2():
    image = bytearray(400 * 1024)
    def word(offset, value):
        struct.pack_into('<I',image,offset,value)
    word(1024,32)
    word(1028,400)
    word(1044,1)
    word(1048,0)
    word(1056,400)
    word(1064,32)
    struct.pack_into('<H',image,1080,0xEF53)
    word(1100,1)
    struct.pack_into('<H',image,1112,128)
    word(1120,2)
    word(2048+8,5)
    def inode(number,mode,size):
        at=5*1024+(number-1)*128
        struct.pack_into('<H',image,at,mode)
        word(at+4,size)
        return at
    root=inode(2,0o40755,1024)
    word(root+40,9)
    payload=inode(12,0o100644,270*1024-7)
    for index in range(12):
        word(payload+40+index*4,40+index)
    word(payload+40+12*4,10)
    word(payload+40+13*4,11)
    for index in range(256):
        word(10*1024+index*4,52+index)
    word(11*1024,13)
    word(13*1024,308)
    word(13*1024+4,309)
    for index in range(270):
        image[(40+index)*1024:(41+index)*1024]=bytes([index%251])*1024
    link=inode(13,0o120777,7)
    image[link+40:link+47]=b'payload'
    offset=9*1024
    for number,name,length,kind in [(2,b'.',12,2),(2,b'..',12,2),(12,b'payload',16,1),(13,b'alias',984,7)]:
        struct.pack_into('<IHBB',image,offset,number,length,len(name),kind)
        image[offset+8:offset+8+len(name)]=name
        offset+=length
    expected=b''.join(bytes([index%251])*1024 for index in range(270))[:-7]
    return bytes(image),expected


class OfflineImageReaderTests(unittest.TestCase):
    def test_ext2_direct_single_and_double_indirect_reads_are_complete(self):
        image, expected = synthetic_ext2()
        before = hashlib.sha256(image).hexdigest()
        reader = Ext2Image(image)
        self.assertEqual(reader.contents(12),expected)
        self.assertEqual(hashlib.sha256(image).hexdigest(),before)
        self.assertEqual([row['path']for row in reader.entries()],['/','/payload','/alias'])
        self.assertEqual(reader.entries()[-1]['target'],'payload')

    def test_symlink_contents_are_reported_without_following_host_paths(self):
        image,_=synthetic_ext2()
        self.assertEqual(Ext2Image(image).contents(13),b'payload')

    def test_invalid_geometry_and_truncated_image_are_rejected(self):
        image,_=synthetic_ext2()
        with self.assertRaises(ValueError):
            Ext2Image(image[:-1024])
        changed=bytearray(image)
        struct.pack_into('<I',changed,1048,0xffffffff)
        with self.assertRaises(ValueError):
            Ext2Image(bytes(changed))
        with self.assertRaises(ValueError):
            Ext2Image(bytes(2048))

    def test_invalid_directory_and_block_references_are_rejected(self):
        image,_=synthetic_ext2()
        changed=bytearray(image)
        struct.pack_into('<H',changed,9*1024+4,0)
        with self.assertRaises(ValueError):
            Ext2Image(bytes(changed)).entries()
        changed=bytearray(image)
        struct.pack_into('<I',changed,5*1024+11*128+40,401)
        with self.assertRaises(ValueError):
            Ext2Image(bytes(changed)).contents(12)

    def test_elf_reader_rejects_foreign_and_truncated_headers(self):
        for data in [b'',b'MZ'+bytes(100),b'\x7fELF\x02\x01'+bytes(100)]:
            with self.assertRaises(ValueError):
                Elf32(data)


class DeviceInstallBoundaryTests(unittest.TestCase):
    def test_flag_helper_and_normal_app_disagree_on_injected_short_write(self):
        for count in [0,1,2,3]:
            self.assertTrue(flag_write('apl_conn',count).caller_may_continue_to_reboot)
            self.assertFalse(flag_write('gemini',count).caller_may_continue_to_reboot)
        for caller in ['apl_conn','gemini']:
            self.assertTrue(flag_write(caller,4).caller_accepts)
            self.assertFalse(flag_write(caller,-5).caller_accepts)
            self.assertFalse(flag_write(caller,4).persistence_verified)
            self.assertFalse(flag_write(caller,4).firmware_write_enabled)

    def test_helper_clear_branch_is_all_ones_not_zero_and_not_proven_called(self):
        result=flag_write('apl_conn',4,setting=False)
        self.assertEqual(result.requested_word,0xffffffff)
        self.assertFalse(result.caller_may_continue_to_reboot)
        self.assertEqual(flag_write('apl_conn',4).requested_word,0x70555766)
        with self.assertRaises(ValueError):
            flag_write('gemini',4,setting=False)

    def test_selected_nvp_block_ignores_unreadable_and_incomplete_tail_records(self):
        records=[TailRecord(0x70,10),TailRecord(0x70,11,read_ok=False),TailRecord(0xff,0)]
        self.assertEqual(select_tail_in_already_chosen_nvp_block(records),0)
        self.assertIsNone(select_tail_in_already_chosen_nvp_block([TailRecord(0x70,12,read_ok=False)]))
        self.assertEqual(select_tail_in_already_chosen_nvp_block([TailRecord(0x70,10),TailRecord(0x70,11)]),1)

    def test_nvp_tail_scan_stops_at_erased_tag_only_after_a_record_was_found(self):
        self.assertEqual(select_tail_in_already_chosen_nvp_block([TailRecord(0xff,0),TailRecord(0x70,1)]),1)
        self.assertEqual(select_tail_in_already_chosen_nvp_block([TailRecord(0x70,1),TailRecord(0xff,0),TailRecord(0x70,2)]),0)

    def test_nvp_tail_metadata_is_not_a_payload_digest_or_wraparound_clock(self):
        self.assertEqual(select_tail_in_already_chosen_nvp_block([TailRecord(0x70,1),TailRecord(0x70,2,data_matches_expected=False)]),1)
        self.assertEqual(select_tail_in_already_chosen_nvp_block([TailRecord(0x70,0xffffffff),TailRecord(0x70,0)]),0)
        self.assertEqual(zone_writer_returncode_slice(0,0,False),zone_writer_returncode_slice(0,0,True))
        self.assertEqual(zone_writer_returncode_slice(-5,0,True),'retry_or_error_branch')
        self.assertEqual(zone_writer_returncode_slice(0,-5,True),'retry_or_error_branch')

    def test_nvp_bios_reads_retry_at_most_twice_and_require_zero(self):
        self.assertEqual(nvp_bios_read_retry([0]),(0,1))
        self.assertEqual(nvp_bios_read_retry([-1,0]),(0,2))
        self.assertEqual(nvp_bios_read_retry([1,0]),(0,2))
        self.assertEqual(nvp_bios_read_retry([-1,-1]),(-5,2))
        self.assertEqual(nvp_bios_read_retry([1,1,0]),(-5,2))
        with self.assertRaises(ValueError):
            nvp_bios_read_retry([-1])

    def test_bios_status_and_timeout_do_not_imply_controlled_rollback(self):
        self.assertEqual(bios_program_or_erase_status(0xc0),'controller_status_success')
        self.assertEqual(bios_program_or_erase_status(0xc1),'program_or_erase_failure_one')
        self.assertEqual(bios_program_or_erase_status(0x40),'panic_path')
        self.assertEqual(bios_program_or_erase_status(0x40,panic_context=True),'program_or_erase_failure_one')
        self.assertEqual(bios_program_or_erase_status(completed=False,panic_on_timeout=False),'timeout_error_minus_one')
        self.assertEqual(bios_program_or_erase_status(completed=False,panic_on_timeout=True),'panic_path')

    def test_early_old_or_empty_version_exit_zero_does_not_mean_firmware_installed(self):
        for version in ['20070507Z','']:
            result=replay_install_script(InstallScenario(version_text=version))
            self.assertEqual(result.script_exit,0)
            self.assertEqual(result.outcome,'early_exit_no_installation')
            self.assertNotIn('format_root',result.trace)
            self.assertEqual(result.old_root,'untouched_by_this_script')

    def test_format_precedes_missing_package_and_update_name_failures(self):
        for step in ['read_update_filename','check_package_exists']:
            result=replay_install_script(InstallScenario(failed_step=step))
            self.assertEqual(result.script_exit,1)
            self.assertLess(result.trace.index('format_root'),result.trace.index(step))
            self.assertEqual(result.old_root,'formatted_old_root_unavailable')
            self.assertFalse(result.rollback_verified)

    def test_all_explicit_shell_error_branches_keep_recovery_unproven(self):
        for step in SCRIPT_STEPS[1:]:
            if step=='opaque_fwpup_tar_pipeline':
                continue
            result=replay_install_script(InstallScenario(failed_step=step))
            self.assertEqual(result.script_exit,1)
            self.assertIn('display_error_and_exit',result.trace)
            self.assertFalse(result.independent_recovery_verified)
            self.assertFalse(result.firmware_write_enabled)
        self.assertEqual(replay_install_script(InstallScenario(failed_step='cd_root')).old_root,'untouched_by_this_script')
        self.assertEqual(replay_install_script(InstallScenario(failed_step='format_root')).old_root,'not_guaranteed_once_format_begins')

    def test_failed_fwpup_can_be_masked_by_successful_consumer(self):
        result=replay_install_script(InstallScenario(fwpup_exit=7,tar_exit=0))
        self.assertEqual(result.script_exit,0)
        self.assertEqual(result.outcome,'masked_producer_failure_script_exit_0')
        self.assertIn('unmount',result.trace)
        self.assertEqual(result.component_write_order,'unknown_fwpup_binary_unavailable')
        result=replay_install_script(InstallScenario(fwpup_exit=7,tar_exit=2))
        self.assertEqual(result.script_exit,1)
        self.assertNotIn('sync',result.trace)

    def test_every_power_cut_boundary_is_explicit_without_inventing_a_resume(self):
        for step in SCRIPT_STEPS:
            result=replay_install_script(InstallScenario(power_cut_step=step))
            self.assertEqual(result.outcome,'power_lost_recovery_unknown')
            self.assertIsNone(result.script_exit)
            self.assertFalse(result.rollback_verified)
            self.assertFalse(result.independent_recovery_verified)
            self.assertFalse(result.firmware_write_enabled)
            self.assertEqual(result.upgrade_flag_after_restart,'unknown_boot_consumer_and_clear_policy')
            self.assertFalse(result.flag_clear_observed_in_shell)
            self.assertNotIn('resume',result.trace)

    def test_successful_shell_exit_never_proves_custom_build_or_clear_policy(self):
        result=replay_install_script()
        self.assertEqual(result.script_exit,0)
        self.assertEqual(result.outcome,'script_exit_0_installed_build_not_verified')
        self.assertFalse(result.flag_clear_observed_in_shell)
        self.assertFalse(result.firmware_write_enabled)
        self.assertEqual(result.component_write_order,'unknown_fwpup_binary_unavailable')

    def test_missing_root_cannot_use_the_normal_app_emergency_fallback(self):
        self.assertEqual(application_fallback(False,True,True),'normal_application_fallback_dependencies_missing')
        self.assertEqual(application_fallback(True,False,True),'normal_application_fallback_dependencies_missing')
        self.assertEqual(application_fallback(True,True,True),'application_level_fallback_only_independent_recovery_unverified')

    def test_pinned_metadata_covers_full_initrd_and_keeps_installation_gaps_explicit(self):
        area=Path(__file__).resolve().parents[1]/'docs/audits/device-install-2026-10-19/static'
        facts=json.loads((area/'facts.json').read_text(encoding='utf-8'))
        self.assertEqual(facts['rootfs_members'],613)
        self.assertEqual(facts['initrd_entries'],25)
        self.assertEqual(facts['named_fwpup_in_rootfs'],[])
        self.assertEqual(facts['named_fwpup_in_initrd'],[])
        self.assertEqual(facts['elf_inputs_not_parsed'],[])
        field=facts['nvp_flag_mapping']
        self.assertEqual((field['zone'],field['node'],field['logical_field_offset'],field['field_bytes']),(0,8,2044,4))
        self.assertEqual(field['node_table'],'MSCZone')
        self.assertEqual(field['relocation_kind'],'R_ARM_ABS32')
        self.assertTrue(field['physical_nand_address_unknown'])
        lines=facts['stock_script']['anchors_1_based']
        self.assertLess(lines['format_root'],lines['read_update_filename'])
        self.assertLess(lines['read_update_filename'],lines['check_package_exists'])
        self.assertLess(lines['check_package_exists'],lines['opaque_fwpup_tar_pipeline'])
        self.assertFalse(facts['stock_script']['explicit_fup_clear_commands_found'])
        self.assertFalse(facts['powerfail_script_in_rootfs'])
        self.assertTrue(facts['powerfail_references'])
        self.assertEqual([row['path']for row in facts['kernel_embedded_initramfs']['entries']],['/dev','/dev/console','/root'])
        self.assertNotIn('fWUp',facts['raw_kernel_literal_hits'])

    def test_private_export_hashes_when_local_evidence_is_available(self):
        area=Path(__file__).resolve().parents[1]/'docs/audits/device-install-2026-10-19/static'
        facts=json.loads((area/'facts.json').read_text(encoding='utf-8'))
        if not all((area/row['file']).is_file() for row in facts['static_exports']):
            self.skipTest('Private Sony analysis exports are not distributed; hash replay not run')
        for row in facts['static_exports']:
            self.assertEqual(hashlib.sha256((area/row['file']).read_bytes()).hexdigest(),row['sha256'])

    def test_pure_install_model_cannot_import_a_driver_or_execute_a_script(self):
        path=Path(__file__).parent/'firmware_research/device_install_model.py'
        for node in ast.walk(ast.parse(path.read_text(encoding='utf-8'))):
            if isinstance(node,ast.Import):
                self.fail('No imports other than dataclasses permitted')
            if isinstance(node,ast.ImportFrom):
                self.assertEqual(node.module,'dataclasses')
            if isinstance(node,ast.Call) and isinstance(node.func,ast.Name):
                self.assertNotIn(node.func.id,['open','exec','eval','__import__','compile'])


if __name__=='__main__':
    unittest.main()
