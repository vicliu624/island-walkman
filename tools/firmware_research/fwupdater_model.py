"""Pure symbolic replay of the pinned A91x MSC file-upload worker.

This DOES NOT execute Sony code, emulate a CPU, send USB, access files/devices,
load DLLs, sleep in wall time, or provide a pluggable hardware transport.
Its oracle is static instructions; it is not a captured hardware transcript.
Unsupported branches stop explicitly rather than pretending to be reproduced.
"""
from dataclasses import dataclass, field

COPY_ERRORS = {112: 0x80047008, 5: 0x80047007}
ACCEPTED_ACTIVATION_STATUS = 0xFF020400
ACTIVATION_CDB = (0xFC, 0, 4, 0x64, 0x62, 0x6D, 0x6E, 0, 0, 0, 0, 0)


def file_error(winerror):
    return COPY_ERRORS.get(winerror, 0x80047006)


def timer_seconds(text):
    """Validated m:ss research inputs only; not a clone of C strtol on bad INI."""
    minutes, seconds = text.split(":")
    if not (minutes.isdecimal() and seconds.isdecimal() and int(seconds) < 60):
        raise ValueError("Unsupported timer input")
    return int(minutes) * 60 + int(seconds)


@dataclass(frozen=True)
class CapturedDevice:
    info: bytes
    vid: int = 0x054C
    pid: int = 0x032A


def select_captured_a91x(devices):
    """Known successful MSC enumeration/read slices only, not driver emulation.

    EXE 0x404f26 compares product and first four vendor-specific bytes. A short
    frame is outside this seam, not evidence of an official length check.
    """
    matches = []
    for device in devices:
        if (device.vid, device.pid) != (0x054C, 0x032A):
            continue
        if len(device.info) != 128:
            raise ValueError("Short frame is outside the recovered successful-read seam")
        product = device.info[24:40].rstrip(b" \0")
        if product in (b"NW-A916", b"NW-A918", b"NW-A919") and device.info[96:100] == bytes(4):
            matches.append(product.decode("ascii"))
    if len(matches) != 1:
        return None, 0x80042200 if not matches else 0x80042201
    return matches[0], None


def version_prompt(current, target, confirmed):
    """Known digit.dd domain; reproduces EXE 0x40798d branch and IDOK check."""
    def numeric(value):
        if len(value) != 4 or value[1] != "." or not (value[0] + value[2:]).isascii() or not (value[0] + value[2:]).isdigit():
            raise ValueError("Version outside the validated digit.dd domain")
        return int(value[0]), int(value[2:])
    old, new = numeric(current), numeric(target)
    resource_id = 203 if old == new else 202 if old < new else 204
    return resource_id, confirmed is True


@dataclass(frozen=True)
class Config:
    timer_ms: int = 100_000
    timeout_ms: int = 150_000
    battery_threshold: int = -1
    command_type: int = 0

    @classmethod
    def from_timer(cls, timer="1:40", timeout=None, battery_threshold=-1, command_type=0):
        seconds = timer_seconds(timer)
        if seconds <= 0:
            raise ValueError("Timer must be positive")
        return cls(seconds * 1000,
                   (timer_seconds(timeout) if timeout is not None else seconds + (seconds >> 1)) * 1000,
                   battery_threshold, command_type)


@dataclass(frozen=True)
class Scenario:
    name: str = "normal"
    # Starting seam: model/config selected and source path resolved by the EXE.
    initial_snapshot_matches: bool = True
    source_kind: str = "file"
    source_bytes: int = 100
    free_bytes: int = 1000
    free_space_winerror: int = 0
    measure_hresult: int = 0
    copy_ms: int = 500
    copy_winerror: int = 0
    staged_hash_matches: bool = True
    activation_ok: bool = True
    drive_index_valid: bool = True
    packed_status: int = 0
    disconnect_at_ms: int | None = 2000
    reconnect_at_ms: int | None = 10000
    open_failures: int = 0
    open_failure_hresult: int = 0x80047002
    reopen_before_cleanup_hresult: int = 0
    readback_snapshot_matches: bool = True
    stat_failures: int = 0
    delete_winerror: int = 0
    version_error: int = 0
    target_version: str = "1.03"
    installed_version: str = "1.03"
    battery_percent: int | None = None
    power_error: int = 0
    tick_start: int = 0


@dataclass(frozen=True)
class Event:
    elapsed_ms: int
    operation: str
    detail: str = ""


@dataclass
class Result:
    scenario: str
    outcome: str = "unresolved"
    hresult: int | None = None
    events: list[Event] = field(default_factory=list)
    elapsed_ms: int = 0
    activation_count: int = 0
    firmware_write_enabled: bool = False
    evidence_kind: str = "static_source_derived_symbolic_replay"


class StopReplay(Exception):
    def __init__(self, outcome, hresult=None):
        self.outcome, self.hresult = outcome, hresult


def replay(scenario=Scenario(), config=Config()):
    """Bounded fake-clock interpreter of selected, documented worker branches.

    There is deliberately no hardware/transport argument. All faults are values.
    Pre-dialog model selection and device-side installation are outside this seam.
    """
    result = Result(scenario.name)
    now = 0
    start = scenario.tick_start & 0xFFFFFFFF

    def elapsed():
        return (((start + now) & 0xFFFFFFFF) - start) & 0xFFFFFFFF

    def event(operation, detail=""):
        result.events.append(Event(elapsed(), operation, str(detail)))

    def sleep(milliseconds):
        nonlocal now
        event("sleep", milliseconds)
        now += milliseconds

    def present():
        value = (scenario.disconnect_at_ms is None or now < scenario.disconnect_at_ms
                 or (scenario.reconnect_at_ms is not None and now >= scenario.reconnect_at_ms))
        event("presence", int(value))
        return value

    def progress(value):
        event("progress", value)

    def delete():
        event("delete_staged_package")
        if scenario.delete_winerror:
            raise StopReplay("cleanup_failed", file_error(scenario.delete_winerror))

    def fail_upload(outcome, code):
        # EXE negative HRESULT branch: cleanup for source kind 0 (regular file).
        # Source-kind -1 (missing) and 1 (directory) skip that cleanup.
        if scenario.source_kind == "file":
            delete()
        raise StopReplay(outcome, code)

    try:
        if config.command_type != 0:
            raise StopReplay("unmodeled_command_type")
        if scenario.source_kind not in ("file", "missing"):
            raise StopReplay("unmodeled_directory_or_source_kind")
        if not (0 <= scenario.open_failures <= 10 and 0 <= scenario.stat_failures <= 10
                and 0 <= scenario.copy_ms <= 0xFFFFFFFF // 2
                and 0 <= scenario.source_bytes < 2**63 and scenario.free_bytes >= 0):
            raise ValueError("Scenario outside bounded replay domain")
        if config.battery_threshold > 0:
            event("power_query")
            if scenario.power_error:
                raise StopReplay("power_query_failed", scenario.power_error)
            if scenario.battery_percent is None:
                raise StopReplay("unmodeled_power_response")
            if scenario.battery_percent < config.battery_threshold:
                raise StopReplay("battery_below_configured_threshold")
        # This preflight slice is modeled separately; actual worker's tick starts
        # after pre-dialog power checking. Fake queries above have zero duration.
        event("worker_start")
        event("revalidate_initial_snapshot")
        if not scenario.initial_snapshot_matches:
            fail_upload("initial_identity_failed", 0x80004005)
        event("classify_source", scenario.source_kind)
        if scenario.source_kind == "missing":
            fail_upload("source_missing", 0x80047003)
        event("query_data_volume_space", scenario.free_bytes)
        if scenario.free_space_winerror:
            fail_upload("free_space_query_failed", file_error(scenario.free_space_winerror))
        event("measure_source", scenario.source_bytes)
        if scenario.measure_hresult:
            fail_upload("source_measure_failed", scenario.measure_hresult)
        if not scenario.source_bytes:
            fail_upload("empty_source", 0x80004005)
        required = scenario.source_bytes + scenario.source_bytes // 20
        if scenario.free_bytes < required:
            fail_upload("insufficient_data_volume_space", 0x80047008)
        event("copy_file", r"\NW_A91X.UPG")
        sleep(scenario.copy_ms)
        if scenario.copy_winerror:
            fail_upload("copy_failed", file_error(scenario.copy_winerror))
        # No staged SHA-256 readback exists in this recovered helper.
        event("activation_request", bytes(ACTIVATION_CDB).hex())
        result.activation_count += 1
        if not scenario.activation_ok:
            if not scenario.drive_index_valid:
                fail_upload("activation_no_drive", 0x80047002)
            if scenario.packed_status & 0xFF0000 and scenario.packed_status != ACCEPTED_ACTIVATION_STATUS:
                fail_upload("activation_status_rejected", 0x80047001)
            event("legacy_false_transport_result_tolerated", hex(scenario.packed_status))
        for index in range(10):
            if elapsed() > config.timeout_ms:
                break
            progress(min(99, elapsed() * 100 // config.timer_ms))
            sleep(1000)
            if index >= 5 and not present():
                break
        if present():
            raise StopReplay("device_did_not_disappear", 0x80042300)
        event("update_host_registry_cache")
        while True:
            if elapsed() > config.timeout_ms:
                raise StopReplay("reconnect_timeout_message_403")
            progress(min(99, elapsed() * 100 // config.timer_ms))
            sleep(2000)
            if present():
                break
        for attempt in range(10):
            progress(100)
            sleep(300)
            event("open_and_compare_snapshot", attempt + 1)
            if attempt >= scenario.open_failures and scenario.readback_snapshot_matches:
                break
            event("query_host_os_version")
        else:
            raise StopReplay("reopen_exhausted",
                             scenario.open_failure_hresult if scenario.readback_snapshot_matches else 0x80004005)
        # Mainstream NT branch. The legacy Windows Me OS-code-4 early break is
        # separately documented, not silently modeled as NT behavior.
        sleep(1000)
        event("reopen_before_cleanup")
        if scenario.reopen_before_cleanup_hresult:
            raise StopReplay("reopen_before_cleanup_failed", scenario.reopen_before_cleanup_hresult)
        for attempt in range(10):
            event("stat_staged_path", attempt + 1)
            if attempt >= scenario.stat_failures:
                break
            sleep(1000)
        delete()
        event("read_installed_numeric_version", scenario.installed_version)
        if scenario.version_error:
            raise StopReplay("version_query_failed", scenario.version_error)
        if scenario.installed_version != scenario.target_version:
            raise StopReplay("installed_version_mismatch", 0x80004005)
        event("completion_message_401")
        result.outcome = "numeric_version_matched"
    except StopReplay as stop:
        result.outcome, result.hresult = stop.outcome, stop.hresult
    result.elapsed_ms = elapsed()
    return result


@dataclass(frozen=True)
class SafetyEvidence:
    """Proposed policy inputs; no live readback or persisted journal implied."""
    exact_identity: bool = False
    power_policy_verified: bool = False
    staged_hash_verified: bool = False
    candidate_qualified: bool = False
    recovery_verified: bool = False
    full_build_readback: bool = False
    durable_intent: bool = False
    unresolved_activation_intent: bool = False
    activation_ambiguous: bool = False


def proposed_decision(evidence, *, after_activation=False):
    """Offline requirements oracle, NOT the production firmware executor."""
    if evidence.activation_ambiguous or evidence.unresolved_activation_intent:
        return "review_no_resend_no_delete_no_other_device_writes"
    if not all((evidence.exact_identity, evidence.power_policy_verified,
                evidence.staged_hash_verified, evidence.candidate_qualified,
                evidence.recovery_verified, evidence.durable_intent)):
        return "blocked_before_activation"
    if after_activation and not evidence.full_build_readback:
        return "review_no_success_no_delete"
    return "simulated_requirements_met_live_executor_still_disabled"
