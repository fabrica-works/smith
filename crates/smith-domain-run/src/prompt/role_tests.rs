//! The bounded role and guide scaffolding use the system renderer's bytes.

use super::role_host_bytes;

const ROLE_BYTES: u32 = 4096;
const DIRECTORY_NAME_BYTES: u32 = 256;
const GUIDE_BYTES: u32 = 16_384;
const ONE_DIRECTORY: u32 = 1;
const TWO_DIRECTORIES: u32 = 2;

#[test]
fn one_directory_counts_the_working_directory_guide_heading() {
    assert_eq!(role_host_bytes(ROLE_BYTES, ONE_DIRECTORY, DIRECTORY_NAME_BYTES, GUIDE_BYTES), Some(20_575));
}

#[test]
fn several_directories_count_each_bounded_name_and_guide() {
    assert_eq!(role_host_bytes(ROLE_BYTES, TWO_DIRECTORIES, DIRECTORY_NAME_BYTES, GUIDE_BYTES), Some(37_526));
}

#[test]
fn overflowing_guide_counts_return_no_host_byte_bound() {
    assert_eq!(role_host_bytes(u32::MAX, u32::MAX, u32::MAX, u32::MAX), None);
}
