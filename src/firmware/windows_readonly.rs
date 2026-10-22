//! Windows-only, fixed data-in requests. No file writing, raw CDB endpoint,
//! activation, reset, retries, dismounts or driver changes are implemented.
use super::responses::{self, ReadQuery};
use anyhow::{Context, Result, ensure};
use nusb::MaybeFuture;
use std::{
    ffi::c_void,
    fs::OpenOptions,
    mem::{offset_of, size_of},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn DeviceIoControl(
        handle: *mut c_void,
        code: u32,
        input: *mut c_void,
        input_len: u32,
        output: *mut c_void,
        output_len: u32,
        returned: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
}

#[repr(C)]
#[derive(Default)]
struct ScsiDirect {
    length: u16,
    status: u8,
    path: u8,
    target: u8,
    lun: u8,
    cdb_len: u8,
    sense_len: u8,
    data_in: u8,
    data_len: u32,
    timeout_seconds: u32,
    data: *mut c_void,
    sense_offset: u32,
    cdb: [u8; 16],
}
#[repr(C)]
#[derive(Default)]
struct Packet {
    scsi: ScsiDirect,
    sense: [u8; 32],
}
#[repr(C, align(4096))]
struct AlignedBuffer([u8; 4096]);

fn ioctl(
    file: &std::fs::File,
    code: u32,
    input: *mut c_void,
    input_len: usize,
    output: *mut c_void,
    output_len: usize,
) -> Result<u32> {
    let mut returned = 0;
    // All buffers are alive for this synchronous request; callers provide their
    // exact allocation lengths. The handle is never opened in overlapped mode.
    let ok = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            code,
            input,
            input_len as u32,
            output,
            output_len as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    ensure!(
        ok != 0,
        "Read-only device control failed: {}",
        std::io::Error::last_os_error()
    );
    ensure!(
        returned as usize <= output_len,
        "Invalid device-control response length"
    );
    Ok(returned)
}
fn descriptor_serial(bytes: &[u8]) -> Result<&str> {
    ensure!(bytes.len() >= 36, "Truncated storage descriptor");
    let word = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let size = word(4) as usize;
    ensure!(
        word(0) >= 36 && word(0) as usize <= size && size >= 36 && size <= bytes.len(),
        "Invalid storage descriptor header"
    );
    ensure!(word(28) == 7, "Selected volume is not USB storage");
    let at = word(24) as usize;
    ensure!(at >= 36 && at < size, "Storage serial is unavailable");
    let tail = &bytes[at..size];
    let end = tail
        .iter()
        .position(|b| *b == 0)
        .context("Unterminated storage serial")?;
    Ok(std::str::from_utf8(&tail[..end])?.trim())
}
fn verify_binding(file: &std::fs::File, expected: &str) -> Result<()> {
    let mut query = [0u8; 12];
    let mut response = [0u8; 4096];
    let n = ioctl(
        file,
        0x2d1400,
        query.as_mut_ptr().cast(),
        query.len(),
        response.as_mut_ptr().cast(),
        response.len(),
    )?;
    ensure!(
        descriptor_serial(&response[..n as usize])? == expected,
        "Volume serial does not match the selected USB device; no vendor request sent"
    );
    Ok(())
}
fn read_query(file: &std::fs::File, query: ReadQuery, address: &[u8; 8]) -> Result<Vec<u8>> {
    let mut data = Box::new(AlignedBuffer([0; 4096]));
    let mut p = packet_for(query, address, data.0.as_mut_ptr().cast())?;
    let pointer = (&mut p as *mut Packet).cast();
    let returned = ioctl(
        file,
        0x4d014,
        pointer,
        size_of::<Packet>(),
        pointer,
        size_of::<Packet>(),
    )?;
    validate_query_response(query, &p, returned)?;
    Ok(data.0[..query.response_len()].to_vec())
}
fn validate_query_response(query: ReadQuery, p: &Packet, returned: u32) -> Result<()> {
    ensure!(
        returned as usize >= size_of::<ScsiDirect>() && returned as usize <= size_of::<Packet>(),
        "Truncated or oversized SCSI status response"
    );
    ensure!(
        p.scsi.status == 0,
        "Read-only SCSI request failed with status {} and sense {:02x?}",
        p.scsi.status,
        p.sense
    );
    ensure!(
        p.scsi.data_len as usize == query.response_len(),
        "Short firmware information response"
    );
    Ok(())
}

fn packet_for(query: ReadQuery, address: &[u8; 8], data: *mut c_void) -> Result<Packet> {
    let mut p = Packet::default();
    p.scsi.length = size_of::<ScsiDirect>() as u16;
    p.scsi.path = address[5];
    p.scsi.target = address[6];
    p.scsi.lun = address[7];
    ensure!(p.scsi.lun == 0, "Only the researched LUN zero is permitted");
    p.scsi.cdb_len = 12;
    p.scsi.sense_len = 32;
    p.scsi.data_in = 1;
    p.scsi.data_len = query.response_len() as u32;
    p.scsi.timeout_seconds = 5;
    p.scsi.data = data;
    p.scsi.sense_offset = offset_of!(Packet, sense) as u32;
    p.scsi.cdb[..12].copy_from_slice(&query.cdb());
    Ok(p)
}

pub fn probe(volume: &str, usb_serial: &str) -> Result<serde_json::Value> {
    let _operation = crate::device_operation::acquire()?;
    ensure!(
        volume.len() == 2
            && volume.as_bytes()[0].is_ascii_alphabetic()
            && volume.as_bytes()[1] == b':',
        "Expected a drive letter such as F:"
    );
    ensure!(!usb_serial.is_empty(), "An explicit USB serial is required");
    let matches = nusb::list_devices()
        .wait()?
        .filter(|d| {
            crate::devices::usb_profile(d.vendor_id(), d.product_id()).is_some()
                && d.serial_number() == Some(usb_serial)
        })
        .count();
    ensure!(
        matches == 1,
        "Exactly one matching supported USB device is required"
    );
    // Windows SPTI requires read/write handle access even for DATA_IN. Opening
    // an existing volume does not write it. This module has no data-out request.
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(3)
        .open(format!(r"\\.\{}", volume.to_ascii_uppercase()))
        .context("Cannot open the selected volume for fixed read-only SCSI queries")?;
    verify_binding(&file, usb_serial)?;
    let mut address = [0u8; 8];
    address[..4].copy_from_slice(&8u32.to_le_bytes());
    let n = ioctl(
        &file,
        0x41018,
        std::ptr::null_mut(),
        0,
        address.as_mut_ptr().cast(),
        address.len(),
    )?;
    ensure!(n == 8, "Cannot establish the selected SCSI address");
    verify_binding(&file, usb_serial)?;
    let info_bytes = read_query(&file, ReadQuery::DeviceInfo, &address)?;
    let info = responses::device_info(&info_bytes)?;
    ensure!(
        crate::devices::firmware_profile(&info.vendor, &info.model).is_some(),
        "Information response does not identify a supported firmware family"
    );
    verify_binding(&file, usb_serial)?;
    let power_result = read_query(&file, ReadQuery::PowerStatus, &address)
        .and_then(|b| responses::power_status(&b));
    let power = power_result.as_ref().ok();
    Ok(
        serde_json::json!({"source":"windows_fixed_read_only_scsi","usb_serial":usb_serial,"device":info.report()?,"power":power,"power_error":power_result.as_ref().err().map(|e|format!("{e:#}")),"fixed_queries_attempted":["device_info","power_status"],"safety":super::session::preflight(Some(&info),power)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_layout_and_alignment_match_windows_x64_spti() {
        assert_eq!(size_of::<ScsiDirect>(), 56);
        assert_eq!(offset_of!(ScsiDirect, data), 24);
        assert_eq!(offset_of!(ScsiDirect, cdb), 36);
        assert_eq!(offset_of!(Packet, sense), 56);
        assert_eq!(std::mem::align_of::<AlignedBuffer>(), 4096);
    }
    #[test]
    fn every_constructible_packet_is_bounded_data_in_with_no_mutating_variant() {
        let address = [8, 0, 0, 0, 0, 0, 1, 0];
        for query in [ReadQuery::DeviceInfo, ReadQuery::PowerStatus] {
            let p = packet_for(query, &address, std::ptr::null_mut()).unwrap();
            assert_eq!(p.scsi.data_in, 1);
            assert_eq!(p.scsi.cdb_len, 12);
            assert_eq!(p.scsi.data_len as usize, query.response_len());
            assert_eq!(p.scsi.timeout_seconds, 5);
            assert_eq!(&p.scsi.cdb[..12], &query.cdb());
        }
        let mut wrong = address;
        wrong[7] = 1;
        assert!(packet_for(ReadQuery::DeviceInfo, &wrong, std::ptr::null_mut()).is_err());
    }
    #[test]
    fn failed_short_and_corrupt_driver_responses_never_reach_the_decoder() {
        let address = [8, 0, 0, 0, 0, 0, 1, 0];
        for query in [ReadQuery::DeviceInfo, ReadQuery::PowerStatus] {
            let mut p = packet_for(query, &address, std::ptr::null_mut()).unwrap();
            for returned in [0, 55, 89, u32::MAX] {
                assert!(validate_query_response(query, &p, returned).is_err());
            }
            for status in [1, 2, 8, 255] {
                p.scsi.status = status;
                assert!(validate_query_response(query, &p, 88).is_err());
            }
            p.scsi.status = 0;
            for length in [0, 1, 31, 127, 129, 4097, u32::MAX] {
                p.scsi.data_len = length;
                assert!(validate_query_response(query, &p, 88).is_err());
            }
        }
    }
    #[test]
    fn volume_binding_rejects_unknown_serial_non_usb_and_truncated_descriptors() {
        let mut b = vec![0; 64];
        b[..4].copy_from_slice(&36u32.to_le_bytes());
        b[4..8].copy_from_slice(&64u32.to_le_bytes());
        b[24..28].copy_from_slice(&36u32.to_le_bytes());
        b[28..32].copy_from_slice(&7u32.to_le_bytes());
        b[36..41].copy_from_slice(b"TEST\0");
        assert_eq!(descriptor_serial(&b).unwrap(), "TEST");
        for n in 0..37 {
            assert!(descriptor_serial(&b[..n]).is_err());
        }
        b[28] = 3;
        assert!(descriptor_serial(&b).is_err());
        b[28] = 7;
        b[24..28].fill(255);
        assert!(descriptor_serial(&b).is_err());
        b[24..28].copy_from_slice(&36u32.to_le_bytes());
        for size in [0u32, 35, 36, 128, u32::MAX] {
            b[4..8].copy_from_slice(&size.to_le_bytes());
            assert!(descriptor_serial(&b).is_err());
        }
        b[4..8].copy_from_slice(&64u32.to_le_bytes());
        b[..4].fill(255);
        assert!(descriptor_serial(&b).is_err());
    }
}
