"""Pure models of observed device-side boundaries, never device executors.

The fwpup executable and resident boot selector are unavailable: their behavior
is deliberately opaque. No NAND geometry or power-loss recovery is simulated.
"""
from dataclasses import dataclass, field


@dataclass(frozen=True)
class FlagWriteResult:
    caller: str
    requested_word: int
    write_result: int
    caller_accepts: bool
    caller_may_continue_to_reboot: bool
    persistence_verified: bool = False
    firmware_write_enabled: bool = False


def flag_write(caller, returned, setting=True):
    if caller not in ('apl_conn', 'gemini'):
        raise ValueError('Unknown caller')
    # Only the helper has the observed all-ones branch. No call to that branch
    # has been recovered; modeling it does not authorize clearing anything.
    if not setting and caller != 'apl_conn':
        raise ValueError('No Gemini clear branch established')
    accepted = returned >= 0 if caller == 'apl_conn' else returned == 4
    return FlagWriteResult(caller, 0x70555766 if setting else 0xffffffff,
                           returned, accepted, accepted and setting)


@dataclass(frozen=True)
class TailRecord:
    tag: int
    sequence: int
    read_ok: bool = True
    data_matches_expected: bool = True


def select_tail_in_already_chosen_nvp_block(records, zone_tag=0x70):
    """Only analyze_partition's second scan, not full boot/NAND recovery.

    Initial block selection, ECC, intermediate pages and torn/OOB programming are
    external. sequence is the stored uint32 value, not an unbounded timestamp.
    """
    highest = 0
    selected = None
    for index, tail in enumerate(records):
        if not 0 <= tail.sequence <= 0xffffffff:
            raise ValueError('Not uint32 sequence')
        if not tail.read_ok:
            continue
        if selected is not None and tail.tag == 0xff:
            break
        if tail.tag == zone_tag and tail.sequence > highest:
            highest = tail.sequence
            selected = index
    return selected


def zone_writer_returncode_slice(write_rc, read_rc, content_matches):
    """The recovered zone writer checks return codes, not a byte comparison.

    The underlying BIOS/controller/ECC may detect faults. This does not assert
    that real corrupt NAND data can pass all lower layers.
    """
    if write_rc < 0 or read_rc < 0:
        return 'retry_or_error_branch'
    return 'returncode_checks_passed_bytes_not_compared_in_this_function'


def nvp_bios_read_retry(return_codes):
    """Two-attempt wrapper in icx1087_nvp_nand_read_page; zero alone succeeds."""
    if not return_codes:
        raise ValueError('No supplied BIOS result')
    for attempt in range(2):
        if attempt >= len(return_codes):
            raise ValueError('Missing result for required retry')
        if return_codes[attempt] == 0:
            return 0, attempt + 1
    return -5, 2


def bios_program_or_erase_status(status=0xc0, completed=True, panic_context=False,
                                 panic_on_timeout=False):
    """Observed status/timeout branch only; no MMIO, timing or flash emulation."""
    if not 0 <= status <= 0xff:
        raise ValueError('Expected supplied status byte')
    if not completed:
        return 'panic_path' if panic_on_timeout and not panic_context else 'timeout_error_minus_one'
    if status & 0xc1 == 0xc0:
        return 'controller_status_success'
    if status & 0xc0 != 0xc0 and not panic_context:
        return 'panic_path'
    return 'program_or_erase_failure_one'


SCRIPT_STEPS = ('check_version', 'cd_root', 'format_root', 'mount_root', 'cd_mnt',
                'read_update_filename', 'check_package_exists', 'opaque_fwpup_tar_pipeline',
                'cd_after_extract', 'sync', 'unmount')


@dataclass(frozen=True)
class InstallScenario:
    name: str = 'normal_shell_path'
    version_text: str = '20080714A'
    failed_step: str | None = None
    power_cut_step: str | None = None
    fwpup_exit: int = 0
    tar_exit: int = 0


@dataclass
class InstallResult:
    scenario: str
    outcome: str = 'unresolved'
    script_exit: int | None = None
    trace: list[str] = field(default_factory=list)
    old_root: str = 'untouched_by_this_script'
    component_write_order: str = 'unknown_fwpup_binary_unavailable'
    upgrade_flag_after_restart: str = 'unknown_boot_consumer_and_clear_policy'
    flag_clear_observed_in_shell: bool = False
    rollback_verified: bool = False
    independent_recovery_verified: bool = False
    firmware_write_enabled: bool = False


def replay_install_script(scenario=InstallScenario()):
    """Source-derived shell-control-flow replay, not shell or Sony execution."""
    if scenario.failed_step is not None and scenario.failed_step not in SCRIPT_STEPS[1:]:
        raise ValueError('Unsupported failure step')
    if scenario.power_cut_step is not None and scenario.power_cut_step not in SCRIPT_STEPS:
        raise ValueError('Unsupported power-cut boundary')
    if scenario.failed_step == 'opaque_fwpup_tar_pipeline':
        raise ValueError('Supply separate producer/consumer exit values for pipeline faults')
    result = InstallResult(scenario.name)
    masked = False
    for step in SCRIPT_STEPS:
        result.trace.append(step)
        if step == 'format_root':
            result.old_root = 'not_guaranteed_once_format_begins'
        if step == scenario.power_cut_step:
            result.outcome = 'power_lost_recovery_unknown'
            return result
        if step == 'check_version':
            # Stock script tests lexical /version; it does not check cat's exit
            # separately. Empty stdout follows this same early-exit branch.
            if scenario.version_text < '20070508A':
                result.outcome = 'early_exit_no_installation'
                result.script_exit = 0
                return result
        elif step == scenario.failed_step:
            result.trace.append('display_error_and_exit')
            result.outcome = 'script_failure_no_rollback_in_shell'
            result.script_exit = 1
            return result
        elif step == 'format_root':
            result.old_root = 'formatted_old_root_unavailable'
        elif step == 'opaque_fwpup_tar_pipeline':
            result.trace.append('pipeline_status_from_tar_only')
            if scenario.tar_exit != 0:
                result.trace.append('display_error_and_exit')
                result.outcome = 'script_failure_no_rollback_in_shell'
                result.script_exit = 1
                return result
            masked = scenario.fwpup_exit != 0
    result.script_exit = 0
    result.outcome = ('masked_producer_failure_script_exit_0' if masked
                      else 'script_exit_0_installed_build_not_verified')
    return result


def application_fallback(root_available, drivers_available, helpers_available):
    if not (root_available and drivers_available and helpers_available):
        return 'normal_application_fallback_dependencies_missing'
    return 'application_level_fallback_only_independent_recovery_unverified'
