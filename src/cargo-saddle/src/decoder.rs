use bstr::ByteSlice;

use crate::validator::Mutability;

const PREFIX: &'static str = "SaddleInternalV1DeclFor";
const SUFFIX_DEP_REF: &'static str = "DepRef";
const SUFFIX_DEP_MUT: &'static str = "DepMut";
const SUFFIX_GRANT_REF: &'static str = "GrantRef";
const SUFFIX_GRANT_MUT: &'static str = "GrantMut";
const SUFFIX_CALLS: &'static str = "Call";

const MALFORMED_SADDLE_MARKER_ERR: &'static str = "Malformed Saddle marker in binary";

#[derive(Debug, Copy, Clone)]
pub enum DecoderEntryKind {
    Dep(Mutability),
    Grant(Mutability),
    Calls,
}

pub fn decode_binary(
    data: &[u8],
    mut handler: impl FnMut(DecoderEntryKind, String, String),
) -> anyhow::Result<()> {
    let mut min_offset = 0;

    for start_offset in memchr::memmem::find_iter(data, PREFIX.as_bytes()) {
        // This is an overlapping scan.
        if start_offset < min_offset {
            continue;
        }

        // Parse mode of type we encountered.
        let mut cursor = &data[start_offset..][PREFIX.len()..];

        let kind = if cursor.starts_with(SUFFIX_DEP_REF.as_bytes()) {
            cursor = &cursor[SUFFIX_DEP_REF.len()..];
            DecoderEntryKind::Dep(Mutability::Immutable)
        } else if cursor.starts_with(SUFFIX_DEP_MUT.as_bytes()) {
            cursor = &cursor[SUFFIX_DEP_MUT.len()..];
            DecoderEntryKind::Dep(Mutability::Mutable)
        } else if cursor.starts_with(SUFFIX_GRANT_REF.as_bytes()) {
            cursor = &cursor[SUFFIX_GRANT_REF.len()..];
            DecoderEntryKind::Grant(Mutability::Immutable)
        } else if cursor.starts_with(SUFFIX_GRANT_MUT.as_bytes()) {
            cursor = &cursor[SUFFIX_GRANT_MUT.len()..];
            DecoderEntryKind::Grant(Mutability::Mutable)
        } else if cursor.starts_with(SUFFIX_CALLS.as_bytes()) {
            cursor = &cursor[SUFFIX_CALLS.len()..];
            DecoderEntryKind::Calls
        } else {
            anyhow::bail!("{MALFORMED_SADDLE_MARKER_ERR}");
        };

        // Parse generics
        let mut cursor = cursor.chars();

        fn parse_ty(cursor: &mut bstr::Chars<'_>) -> anyhow::Result<String> {
            let mut collector = String::new();

            let mut generic_level = 0;
            while let Some(char) = cursor.clone().next() {
                if char == ' ' {
                    let _ = cursor.next();
                    continue;
                } else if char == '<' {
                    generic_level += 1;
                    let _ = cursor.next();
                } else if char == '>' {
                    if generic_level > 0 {
                        generic_level -= 1;
                        let _ = cursor.next();
                    } else {
                        break;
                    }
                } else if char == ',' && generic_level == 0 {
                    break;
                } else {
                    let _ = cursor.next();
                }

                collector.push(char);
            }

            Ok(collector)
        }

        anyhow::ensure!(cursor.next() == Some('<'), "{MALFORMED_SADDLE_MARKER_ERR}");
        let ty_1 = parse_ty(&mut cursor)?;
        anyhow::ensure!(cursor.next() == Some(','), "{MALFORMED_SADDLE_MARKER_ERR}");
        let ty_2 = parse_ty(&mut cursor)?;
        anyhow::ensure!(cursor.next() == Some('>'), "{MALFORMED_SADDLE_MARKER_ERR}");

        handler(kind, ty_1, ty_2);

        min_offset = data.len() - cursor.as_bytes().len();
    }

    Ok(())
}
