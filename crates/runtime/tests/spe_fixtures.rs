//! Integration tests for SPE directory parsing with synthetic byte fixtures.

use std::io::Write;
use tempfile::NamedTempFile;

use abuse_runtime::data::spe::{SpeDirectory, SpeError, SpeParseMode, SpecType, SPEC_SIGNATURE};

/// Helper to create a minimal valid SPE file with custom entries.
fn create_spe_fixture(entries: Vec<TestEntry>) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("failed to create temp file");

    // Write signature
    file.write_all(SPEC_SIGNATURE).unwrap();

    // Write entry count
    let count = entries.len() as u16;
    file.write_all(&count.to_le_bytes()).unwrap();

    // Calculate directory size to determine payload offsets
    let mut directory_size = 10_u32; // signature (8) + count (2)
    for entry in &entries {
        directory_size += 1; // type
        directory_size += 1; // name_len
        directory_size += entry.name.len() as u32;
        directory_size += 1; // flags
        directory_size += 4; // size
        directory_size += 4; // offset
    }

    let mut current_offset = directory_size;

    // Write directory entries
    for entry in &entries {
        file.write_all(&[entry.spec_type]).unwrap();
        file.write_all(&[entry.name.len() as u8]).unwrap();
        file.write_all(entry.name.as_bytes()).unwrap();
        file.write_all(&[entry.flags]).unwrap();
        file.write_all(&(entry.payload.len() as u32).to_le_bytes())
            .unwrap();
        file.write_all(&current_offset.to_le_bytes()).unwrap();
        current_offset += entry.payload.len() as u32;
    }

    // Write payloads
    for entry in &entries {
        file.write_all(&entry.payload).unwrap();
    }

    file.flush().unwrap();
    file
}

struct TestEntry {
    spec_type: u8,
    name: String,
    flags: u8,
    payload: Vec<u8>,
}

impl TestEntry {
    fn new(spec_type: u8, name: &str, flags: u8, payload: Vec<u8>) -> Self {
        Self {
            spec_type,
            name: name.to_string(),
            flags,
            payload,
        }
    }
}

#[test]
fn parses_valid_empty_spe() {
    let file = create_spe_fixture(vec![]);
    let dir = SpeDirectory::open(file.path()).expect("should parse empty SPE");
    assert_eq!(dir.entries.len(), 0);
}

#[test]
fn parses_valid_spe_with_entries() {
    let entries = vec![
        TestEntry::new(2, "palette\0", 0, vec![1, 2, 3]),
        TestEntry::new(4, "image001", 0, vec![10, 20, 30, 40]),
        TestEntry::new(5, "tile42", 1, vec![5, 6]),
    ];

    let file = create_spe_fixture(entries);
    let dir = SpeDirectory::open(file.path()).expect("should parse valid SPE");

    assert_eq!(dir.entries.len(), 3);

    assert_eq!(dir.entries[0].spec_type, SpecType::Palette);
    assert_eq!(dir.entries[0].name, "palette");
    assert_eq!(dir.entries[0].flags, 0);
    assert_eq!(dir.entries[0].size, 3);

    assert_eq!(dir.entries[1].spec_type, SpecType::Image);
    assert_eq!(dir.entries[1].name, "image001");
    assert_eq!(dir.entries[1].size, 4);

    assert_eq!(dir.entries[2].spec_type, SpecType::ForeTile);
    assert_eq!(dir.entries[2].name, "tile42");
    assert_eq!(dir.entries[2].flags, 1);
    assert_eq!(dir.entries[2].size, 2);
}

#[test]
fn rejects_invalid_signature() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"INVALID\0").unwrap();
    file.write_all(&[0_u8; 2]).unwrap(); // count
    file.flush().unwrap();

    let result = SpeDirectory::open(file.path());
    assert!(matches!(result, Err(SpeError::BadSignature { .. })));
}

#[test]
fn strict_rejects_unknown_type() {
    let entries = vec![TestEntry::new(99, "unknown", 0, vec![])];
    let file = create_spe_fixture(entries);

    let result = SpeDirectory::open_with_mode(file.path(), SpeParseMode::Strict);
    assert!(matches!(result, Err(SpeError::InvalidType { .. })));
}

#[test]
fn lenient_accepts_unknown_type() {
    let entries = vec![
        TestEntry::new(99, "unknown", 0, vec![1, 2]),
        TestEntry::new(2, "valid", 0, vec![3, 4]),
    ];
    let file = create_spe_fixture(entries);

    let dir = SpeDirectory::open_lenient(file.path()).expect("lenient should parse");
    assert_eq!(dir.entries.len(), 2);
    assert_eq!(dir.entries[0].spec_type, SpecType::Invalid);
    assert_eq!(dir.entries[0].name, "unknown");
    assert_eq!(dir.entries[1].spec_type, SpecType::Palette);
}

#[test]
fn strict_rejects_invalid_utf8_name() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(SPEC_SIGNATURE).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap(); // 1 entry

    // Entry with invalid UTF-8
    file.write_all(&[2_u8]).unwrap(); // type = Palette
    file.write_all(&[3_u8]).unwrap(); // name_len = 3
    file.write_all(&[0xFF, 0xFE, 0xFD]).unwrap(); // invalid UTF-8
    file.write_all(&[0_u8]).unwrap(); // flags
    file.write_all(&2_u32.to_le_bytes()).unwrap(); // size
    file.write_all(&20_u32.to_le_bytes()).unwrap(); // offset

    // Payload
    file.write_all(&[1, 2]).unwrap();
    file.flush().unwrap();

    let result = SpeDirectory::open_with_mode(file.path(), SpeParseMode::Strict);
    assert!(matches!(result, Err(SpeError::InvalidNameEncoding { .. })));
}

#[test]
fn lenient_accepts_invalid_utf8_name() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(SPEC_SIGNATURE).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();

    file.write_all(&[2_u8]).unwrap();
    file.write_all(&[3_u8]).unwrap();
    file.write_all(&[0xFF, 0xFE, 0xFD]).unwrap();
    file.write_all(&[0_u8]).unwrap();
    file.write_all(&2_u32.to_le_bytes()).unwrap();
    file.write_all(&20_u32.to_le_bytes()).unwrap();

    file.write_all(&[1, 2]).unwrap();
    file.flush().unwrap();

    let dir = SpeDirectory::open_lenient(file.path()).expect("lenient should parse");
    assert_eq!(dir.entries.len(), 1);
    assert_eq!(dir.entries[0].name, ""); // lenient uses empty string
}

#[test]
fn rejects_zero_length_name() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(SPEC_SIGNATURE).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();

    file.write_all(&[2_u8]).unwrap(); // type
    file.write_all(&[0_u8]).unwrap(); // name_len = 0 (invalid)
    file.flush().unwrap();

    let result = SpeDirectory::open(file.path());
    assert!(matches!(result, Err(SpeError::InvalidNameLength)));
}

#[test]
fn rejects_out_of_bounds_entry() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(SPEC_SIGNATURE).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();

    file.write_all(&[2_u8]).unwrap();
    file.write_all(&[4_u8]).unwrap();
    file.write_all(b"test").unwrap();
    file.write_all(&[0_u8]).unwrap();
    file.write_all(&1000_u32.to_le_bytes()).unwrap(); // huge size
    file.write_all(&20_u32.to_le_bytes()).unwrap(); // offset
    file.flush().unwrap();

    let result = SpeDirectory::open(file.path());
    assert!(matches!(result, Err(SpeError::InvalidEntryBounds { .. })));
}

#[test]
fn handles_null_terminated_names() {
    let entries = vec![
        TestEntry::new(2, "name\0", 0, vec![]),
        TestEntry::new(4, "plain", 0, vec![]),
    ];
    let file = create_spe_fixture(entries);

    let dir = SpeDirectory::open(file.path()).expect("should parse");
    assert_eq!(dir.entries[0].name, "name"); // null stripped
    assert_eq!(dir.entries[1].name, "plain"); // no null
}

#[test]
fn find_by_name_returns_first_match() {
    let entries = vec![
        TestEntry::new(2, "palette", 0, vec![]),
        TestEntry::new(4, "image", 0, vec![]),
        TestEntry::new(2, "palette", 0, vec![]), // duplicate name
    ];
    let file = create_spe_fixture(entries);
    let dir = SpeDirectory::open(file.path()).unwrap();

    let found = dir.find_by_name("palette").expect("should find");
    assert_eq!(found.spec_type, SpecType::Palette);
    // First entry offset (after 10-byte header + 3 directory entries)
    // Each entry: 1(type) + 1(len) + name_bytes + 1(flags) + 4(size) + 4(offset)
    // = 1 + 1 + 7 + 1 + 4 + 4 = 18 for "palette"
    // = 1 + 1 + 5 + 1 + 4 + 4 = 16 for "image"
    // Total: 10 + 18 + 16 + 18 = 62
    assert_eq!(found.offset, 62);

    assert!(dir.find_by_name("missing").is_none());
}

#[test]
fn find_by_type_returns_first_match() {
    let entries = vec![
        TestEntry::new(4, "img1", 0, vec![]),
        TestEntry::new(2, "pal", 0, vec![]),
        TestEntry::new(4, "img2", 0, vec![]),
    ];
    let file = create_spe_fixture(entries);
    let dir = SpeDirectory::open(file.path()).unwrap();

    let found = dir.find_by_type(SpecType::Image).expect("should find");
    assert_eq!(found.name, "img1");

    assert!(dir.find_by_type(SpecType::ExternSfx).is_none());
}

#[test]
fn entries_of_type_returns_all_matches() {
    let entries = vec![
        TestEntry::new(4, "img1", 0, vec![]),
        TestEntry::new(2, "pal", 0, vec![]),
        TestEntry::new(4, "img2", 0, vec![]),
        TestEntry::new(4, "img3", 0, vec![]),
    ];
    let file = create_spe_fixture(entries);
    let dir = SpeDirectory::open(file.path()).unwrap();

    let images: Vec<_> = dir.entries_of_type(SpecType::Image).collect();
    assert_eq!(images.len(), 3);
    assert_eq!(images[0].name, "img1");
    assert_eq!(images[1].name, "img2");
    assert_eq!(images[2].name, "img3");
}

// ============================================================================
// Environment-gated real-data compatibility tests
// ============================================================================

#[cfg(test)]
mod compat_tests {
    use super::*;
    use std::path::PathBuf;

    fn legacy_root() -> Option<PathBuf> {
        std::env::var("ABUSE_LEGACY_ROOT").ok().map(PathBuf::from)
    }

    #[test]
    fn compat_parses_backgrnd_spe() {
        let Some(root) = legacy_root() else {
            eprintln!("Skipping compat test: ABUSE_LEGACY_ROOT not set");
            return;
        };

        let path = root.join("data/art/back/backgrnd.spe");
        if !path.exists() {
            eprintln!("Skipping: {} not found", path.display());
            return;
        }

        let dir = SpeDirectory::open_lenient(&path).expect("should parse legacy backgrnd.spe");

        assert!(dir.entries.len() > 0, "should have entries");

        // Should have a palette
        let palette = dir.find_by_type(SpecType::Palette);
        assert!(palette.is_some(), "should contain palette entry");

        // Should have background tiles
        let bg_tiles: Vec<_> = dir.entries_of_type(SpecType::BackTile).collect();
        assert!(bg_tiles.len() > 0, "should contain background tiles");
    }

    #[test]
    fn compat_parses_foregrnd_spe() {
        let Some(root) = legacy_root() else {
            eprintln!("Skipping compat test: ABUSE_LEGACY_ROOT not set");
            return;
        };

        let path = root.join("data/art/fore/foregrnd.spe");
        if !path.exists() {
            eprintln!("Skipping: {} not found", path.display());
            return;
        }

        let dir = SpeDirectory::open_lenient(&path).expect("should parse legacy foregrnd.spe");

        assert!(dir.entries.len() > 0);

        // Should have foreground tiles
        let fg_tiles: Vec<_> = dir.entries_of_type(SpecType::ForeTile).collect();
        assert!(fg_tiles.len() > 0, "should contain foreground tiles");
    }

    #[test]
    fn compat_parses_level00_spe() {
        let Some(root) = legacy_root() else {
            eprintln!("Skipping compat test: ABUSE_LEGACY_ROOT not set");
            return;
        };

        let path = root.join("data/levels/level00.spe");
        if !path.exists() {
            eprintln!("Skipping: {} not found", path.display());
            return;
        }

        let dir = SpeDirectory::open_lenient(&path).expect("should parse legacy level00.spe");

        assert!(dir.entries.len() > 0);

        // Should have level data sections (stored as DataArray type 20)
        let fg_map = dir.find_by_name("fgmap");
        assert!(fg_map.is_some(), "should contain fgmap");
        assert_eq!(fg_map.unwrap().spec_type, SpecType::GrueFgMap);

        let bg_map = dir.find_by_name("bgmap");
        assert!(bg_map.is_some(), "should contain bgmap");
        assert_eq!(bg_map.unwrap().spec_type, SpecType::GrueBgMap);

        // Object data is stored as DataArray entries
        let object_list = dir.find_by_name("object_list");
        assert!(object_list.is_some(), "should contain object_list");
        assert_eq!(object_list.unwrap().spec_type, SpecType::DataArray);
    }
}
