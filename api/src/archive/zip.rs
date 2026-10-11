use chrono::{DateTime, Datelike, Timelike, Utc};

const LOCAL_HEADER: u32 = 0x0403_4b50;
const DATA_DESCRIPTOR: u32 = 0x0807_4b50;
const CENTRAL_HEADER: u32 = 0x0201_4b50;
const END_OF_DIRECTORY: u32 = 0x0605_4b50;
const ZIP64_END_OF_DIRECTORY: u32 = 0x0606_4b50;
const ZIP64_LOCATOR: u32 = 0x0706_4b50;

const UTF8_NAME: u16 = 1 << 11;
const DESCRIPTOR_FOLLOWS: u16 = 1 << 3;
const ZIP64_FIELD: u16 = 0x0001;

const NEEDED: u16 = 20;
const NEEDED_ZIP64: u16 = 45;
const MS_DOS_DIRECTORY: u32 = 0x10;

const U16_FULL: u16 = u16::MAX;
const U32_FULL: u32 = u32::MAX;

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub directory: bool,
    pub size: u64,
    pub crc: u32,
    pub offset: u64,
    pub modified: DateTime<Utc>,
}

fn big(size: u64) -> bool {
    size >= u64::from(U32_FULL)
}

fn dos_time(at: DateTime<Utc>) -> (u16, u16) {
    let year = u32::try_from(at.year()).unwrap_or(1980).clamp(1980, 2107) - 1980;
    let date = (year << 9) | (at.month() << 5) | at.day();
    let time = (at.hour() << 11) | (at.minute() << 5) | (at.second() / 2);
    (
        u16::try_from(time).unwrap_or(0),
        u16::try_from(date).unwrap_or(0),
    )
}

fn zip64_extra(values: &[u64]) -> Vec<u8> {
    let mut extra = Vec::with_capacity(4 + values.len() * 8);
    extra.extend_from_slice(&ZIP64_FIELD.to_le_bytes());
    extra.extend_from_slice(&u16::try_from(values.len() * 8).unwrap_or(0).to_le_bytes());
    for value in values {
        extra.extend_from_slice(&value.to_le_bytes());
    }
    extra
}

#[must_use]
pub fn local_header(entry: &Entry) -> Vec<u8> {
    let zip64 = !entry.directory && big(entry.size);
    let extra = if zip64 {
        zip64_extra(&[0, 0])
    } else {
        Vec::new()
    };
    let (time, date) = dos_time(entry.modified);
    let flags = UTF8_NAME
        | if entry.directory {
            0
        } else {
            DESCRIPTOR_FOLLOWS
        };
    let sizes = if zip64 { U32_FULL } else { 0 };

    let mut header = Vec::with_capacity(30 + entry.name.len() + extra.len());
    header.extend_from_slice(&LOCAL_HEADER.to_le_bytes());
    header.extend_from_slice(&(if zip64 { NEEDED_ZIP64 } else { NEEDED }).to_le_bytes());
    header.extend_from_slice(&flags.to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes());
    header.extend_from_slice(&time.to_le_bytes());
    header.extend_from_slice(&date.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&sizes.to_le_bytes());
    header.extend_from_slice(&sizes.to_le_bytes());
    header.extend_from_slice(&u16::try_from(entry.name.len()).unwrap_or(0).to_le_bytes());
    header.extend_from_slice(&u16::try_from(extra.len()).unwrap_or(0).to_le_bytes());
    header.extend_from_slice(entry.name.as_bytes());
    header.extend_from_slice(&extra);
    header
}

#[must_use]
pub fn data_descriptor(size: u64, crc: u32) -> Vec<u8> {
    let mut descriptor = Vec::with_capacity(24);
    descriptor.extend_from_slice(&DATA_DESCRIPTOR.to_le_bytes());
    descriptor.extend_from_slice(&crc.to_le_bytes());
    if big(size) {
        descriptor.extend_from_slice(&size.to_le_bytes());
        descriptor.extend_from_slice(&size.to_le_bytes());
    } else {
        let size = u32::try_from(size).unwrap_or(U32_FULL);
        descriptor.extend_from_slice(&size.to_le_bytes());
        descriptor.extend_from_slice(&size.to_le_bytes());
    }
    descriptor
}

fn central_entry(entry: &Entry) -> Vec<u8> {
    let sizes_big = !entry.directory && big(entry.size);
    let offset_big = big(entry.offset);

    let mut wide = Vec::new();
    if sizes_big {
        wide.push(entry.size);
        wide.push(entry.size);
    }
    if offset_big {
        wide.push(entry.offset);
    }
    let extra = if wide.is_empty() {
        Vec::new()
    } else {
        zip64_extra(&wide)
    };

    let (time, date) = dos_time(entry.modified);
    let flags = UTF8_NAME
        | if entry.directory {
            0
        } else {
            DESCRIPTOR_FOLLOWS
        };
    let needed = if wide.is_empty() {
        NEEDED
    } else {
        NEEDED_ZIP64
    };
    let size = if sizes_big {
        U32_FULL
    } else {
        u32::try_from(entry.size).unwrap_or(U32_FULL)
    };
    let offset = if offset_big {
        U32_FULL
    } else {
        u32::try_from(entry.offset).unwrap_or(U32_FULL)
    };
    let external = if entry.directory { MS_DOS_DIRECTORY } else { 0 };

    let mut header = Vec::with_capacity(46 + entry.name.len() + extra.len());
    header.extend_from_slice(&CENTRAL_HEADER.to_le_bytes());
    header.extend_from_slice(&needed.to_le_bytes());
    header.extend_from_slice(&needed.to_le_bytes());
    header.extend_from_slice(&flags.to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes());
    header.extend_from_slice(&time.to_le_bytes());
    header.extend_from_slice(&date.to_le_bytes());
    header.extend_from_slice(&entry.crc.to_le_bytes());
    header.extend_from_slice(&size.to_le_bytes());
    header.extend_from_slice(&size.to_le_bytes());
    header.extend_from_slice(&u16::try_from(entry.name.len()).unwrap_or(0).to_le_bytes());
    header.extend_from_slice(&u16::try_from(extra.len()).unwrap_or(0).to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes());
    header.extend_from_slice(&external.to_le_bytes());
    header.extend_from_slice(&offset.to_le_bytes());
    header.extend_from_slice(entry.name.as_bytes());
    header.extend_from_slice(&extra);
    header
}

#[must_use]
pub fn central_directory(entries: &[Entry], start: u64) -> Vec<u8> {
    let mut directory: Vec<u8> = entries.iter().flat_map(central_entry).collect();
    let size = directory.len() as u64;
    let count = entries.len() as u64;
    let wide = count >= u64::from(U16_FULL) || big(size) || big(start);

    if wide {
        let record_at = start + size;
        directory.extend_from_slice(&ZIP64_END_OF_DIRECTORY.to_le_bytes());
        directory.extend_from_slice(&44_u64.to_le_bytes());
        directory.extend_from_slice(&NEEDED_ZIP64.to_le_bytes());
        directory.extend_from_slice(&NEEDED_ZIP64.to_le_bytes());
        directory.extend_from_slice(&0_u32.to_le_bytes());
        directory.extend_from_slice(&0_u32.to_le_bytes());
        directory.extend_from_slice(&count.to_le_bytes());
        directory.extend_from_slice(&count.to_le_bytes());
        directory.extend_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&start.to_le_bytes());

        directory.extend_from_slice(&ZIP64_LOCATOR.to_le_bytes());
        directory.extend_from_slice(&0_u32.to_le_bytes());
        directory.extend_from_slice(&record_at.to_le_bytes());
        directory.extend_from_slice(&1_u32.to_le_bytes());
    }

    let (count16, size32, start32) = if wide {
        (U16_FULL, U32_FULL, U32_FULL)
    } else {
        (
            u16::try_from(count).unwrap_or(U16_FULL),
            u32::try_from(size).unwrap_or(U32_FULL),
            u32::try_from(start).unwrap_or(U32_FULL),
        )
    };
    directory.extend_from_slice(&END_OF_DIRECTORY.to_le_bytes());
    directory.extend_from_slice(&0_u16.to_le_bytes());
    directory.extend_from_slice(&0_u16.to_le_bytes());
    directory.extend_from_slice(&count16.to_le_bytes());
    directory.extend_from_slice(&count16.to_le_bytes());
    directory.extend_from_slice(&size32.to_le_bytes());
    directory.extend_from_slice(&start32.to_le_bytes());
    directory.extend_from_slice(&0_u16.to_le_bytes());
    directory
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn moment() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 11, 12, 30, 44)
            .single()
            .expect("a moment")
    }

    fn file(name: &str, size: u64, offset: u64) -> Entry {
        Entry {
            name: name.to_owned(),
            directory: false,
            size,
            crc: 0xdead_beef,
            offset,
            modified: moment(),
        }
    }

    fn word(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
    }

    #[test]
    fn a_small_file_has_no_zip64_field_anywhere() {
        let entry = file("a.txt", 10, 0);

        assert_eq!(local_header(&entry).len(), 30 + 5);
        assert_eq!(data_descriptor(10, 7).len(), 16);
        let directory = central_directory(&[entry], 100);
        assert_eq!(word(&directory, 0), CENTRAL_HEADER);
        assert_eq!(directory.len(), 46 + 5 + 22);
    }

    #[test]
    fn a_file_past_four_gibibytes_carries_zip64_sizes() {
        let size = 5 * 1024 * 1024 * 1024;
        let entry = file("big.bin", size, 0);

        let header = local_header(&entry);
        assert_eq!(word(&header, 18), U32_FULL);
        assert_eq!(header.len(), 30 + 7 + 20);
        assert_eq!(data_descriptor(size, 7).len(), 24);

        let directory = central_directory(&[entry], 100);
        assert_eq!(word(&directory, 20), U32_FULL);
        let extra_at = 46 + 7;
        assert_eq!(
            u16::from_le_bytes([directory[extra_at], directory[extra_at + 1]]),
            ZIP64_FIELD
        );
        assert_eq!(
            u64::from_le_bytes(
                directory[extra_at + 4..extra_at + 12]
                    .try_into()
                    .expect("8")
            ),
            size
        );
    }

    #[test]
    fn an_offset_past_four_gibibytes_goes_in_the_central_extra() {
        let entry = file("late.txt", 3, 6 * 1024 * 1024 * 1024);

        let directory = central_directory(&[entry], 7 * 1024 * 1024 * 1024);

        assert_eq!(word(&directory, 42), U32_FULL);
        let extra_at = 46 + 8;
        assert_eq!(
            u64::from_le_bytes(
                directory[extra_at + 4..extra_at + 12]
                    .try_into()
                    .expect("8")
            ),
            6 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn a_directory_entry_has_no_descriptor_flag_and_the_dos_attribute() {
        let entry = Entry {
            name: "photos/".to_owned(),
            directory: true,
            size: 0,
            crc: 0,
            offset: 0,
            modified: moment(),
        };

        let header = local_header(&entry);
        let flags = u16::from_le_bytes([header[6], header[7]]);
        assert_eq!(flags & DESCRIPTOR_FOLLOWS, 0);

        let directory = central_directory(&[entry], 0);
        assert_eq!(word(&directory, 38), MS_DOS_DIRECTORY);
    }

    #[test]
    fn many_entries_switch_to_the_zip64_end_of_directory() {
        let entries: Vec<Entry> = (0..70_000).map(|n| file(&format!("{n}"), 1, 0)).collect();

        let directory = central_directory(&entries, 0);
        let tail = &directory[directory.len() - 22..];

        assert_eq!(word(tail, 0), END_OF_DIRECTORY);
        assert_eq!(u16::from_le_bytes([tail[10], tail[11]]), U16_FULL);
        let locator = directory.len() - 22 - 20;
        assert_eq!(word(&directory, locator), ZIP64_LOCATOR);
    }

    #[test]
    fn a_dos_date_is_clamped_to_what_the_format_can_hold() {
        let ancient = Utc
            .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
            .single()
            .expect("t");

        let (_, date) = dos_time(ancient);

        assert_eq!(date >> 9, 0);
    }
}
