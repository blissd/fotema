// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Reads person-name face-region tags embedded in photo XMP metadata so existing
// names written by other software (digiKam, Picasa, Windows Photo Gallery, Apple
// Photos, ...) can be imported. Two common schemas are supported:
//
//  - MWG Regions (`mwg-rs:Regions` / `mwg-rs:Name` + `mwg-rs:Area` whose `x` is
//    the region centre, normalized 0..1).
//  - Microsoft People Tags (`MPReg:PersonDisplayName` + `MPReg:Rectangle`
//    "x, y, w, h", top-left, normalized 0..1).
//
// This is a best-effort, read-only parser: anything it can't understand simply
// yields no tags. Only the horizontal centre is extracted, which is enough to
// order regions left-to-right and match them to detected faces.

use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// A named face region read from a photo's XMP.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceTag {
    /// Person name.
    pub name: String,
    /// Horizontal centre of the region, normalized to 0..1.
    pub center_x: f32,
}

/// Read named face-region tags from a photo file. Returns an empty vector when
/// the file has no readable XMP person tags.
pub fn read_face_tags(path: &Path) -> Vec<FaceTag> {
    match extract_xmp(path) {
        Some(xmp) => parse_face_tags(&xmp),
        None => Vec::new(),
    }
}

/// Pull the `<x:xmpmeta>...</x:xmpmeta>` packet out of the first part of the
/// file. Bounded to 1 MiB: the face-region packet sits in an early APP1 segment
/// in practice, and this keeps the cost small across a large library.
fn extract_xmp(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    file.take(1024 * 1024).read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);

    let start = text.find("<x:xmpmeta")?;
    let rest = &text[start..];
    let end = rest.find("</x:xmpmeta>")? + "</x:xmpmeta>".len();
    Some(rest[..end].to_string())
}

#[derive(Clone, Copy)]
enum Field {
    Name,
    Type,
    Rect,
}

/// Parse the XMP XML for MWG / Microsoft face regions.
fn parse_face_tags(xmp: &str) -> Vec<FaceTag> {
    let mut reader = Reader::from_str(xmp);
    let mut out: Vec<FaceTag> = Vec::new();

    let mut cur_name: Option<String> = None;
    let mut cur_cx: Option<f32> = None;
    let mut is_face = true;
    let mut capture: Option<Field> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                match e.local_name().as_ref() {
                    b"li" | b"Description" => {
                        cur_name = None;
                        cur_cx = None;
                        is_face = true;
                        // Compact form keeps name/area as attributes on the li.
                        scan_region_attrs(e.attributes(), &mut cur_name, &mut cur_cx);
                    }
                    b"Area" => scan_region_attrs(e.attributes(), &mut cur_name, &mut cur_cx),
                    b"Name" | b"PersonDisplayName" => capture = Some(Field::Name),
                    b"Type" => capture = Some(Field::Type),
                    b"Rectangle" => capture = Some(Field::Rect),
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                b"li" | b"Description" => {
                    cur_name = None;
                    cur_cx = None;
                    is_face = true;
                    scan_region_attrs(e.attributes(), &mut cur_name, &mut cur_cx);
                    flush(&mut out, &mut cur_name, &mut cur_cx, &mut is_face);
                }
                b"Area" => scan_region_attrs(e.attributes(), &mut cur_name, &mut cur_cx),
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if let Some(field) = capture.take() {
                    let txt = e.unescape().unwrap_or_default().trim().to_string();
                    match field {
                        Field::Name => {
                            if !txt.is_empty() {
                                cur_name = Some(txt);
                            }
                        }
                        Field::Type => {
                            if !txt.eq_ignore_ascii_case("Face") {
                                is_face = false;
                            }
                        }
                        Field::Rect => cur_cx = ms_rectangle_center_x(&txt),
                    }
                }
            }
            Ok(Event::End(e)) => {
                capture = None;
                if matches!(e.local_name().as_ref(), b"li" | b"Description") {
                    flush(&mut out, &mut cur_name, &mut cur_cx, &mut is_face);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    out
}

/// Push the current region if it is a complete face region, and reset state.
fn flush(
    out: &mut Vec<FaceTag>,
    cur_name: &mut Option<String>,
    cur_cx: &mut Option<f32>,
    is_face: &mut bool,
) {
    if *is_face {
        if let (Some(name), Some(center_x)) = (cur_name.take(), cur_cx.take()) {
            out.push(FaceTag { name, center_x });
        }
    }
    *cur_name = None;
    *cur_cx = None;
    *is_face = true;
}

/// Scan element attributes for a region name and centre-x (compact / MWG forms).
fn scan_region_attrs(
    attrs: quick_xml::events::attributes::Attributes,
    cur_name: &mut Option<String>,
    cur_cx: &mut Option<f32>,
) {
    for attr in attrs.flatten() {
        match attr.key.local_name().as_ref() {
            b"Name" | b"PersonDisplayName" => {
                let v = String::from_utf8_lossy(&attr.value).trim().to_string();
                if !v.is_empty() {
                    *cur_name = Some(v);
                }
            }
            // MWG `stArea:x` is the normalized region centre.
            b"x" => {
                if cur_cx.is_none() {
                    if let Ok(v) = String::from_utf8_lossy(&attr.value).trim().parse::<f32>() {
                        *cur_cx = Some(v);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Microsoft `Rectangle` is "x, y, w, h" (top-left, normalized); return centre x.
fn ms_rectangle_center_x(txt: &str) -> Option<f32> {
    let parts: Vec<f32> = txt
        .split(',')
        .filter_map(|s| s.trim().parse::<f32>().ok())
        .collect();
    if parts.len() == 4 {
        Some(parts[0] + parts[2] / 2.0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mwg_regions() {
        let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:mwg-rs="http://www.metadataworkinggroup.com/schemas/regions/"
                             xmlns:stArea="http://ns.adobe.com/xmp/sType/Area#">
              <mwg-rs:Regions>
                <mwg-rs:RegionList>
                  <rdf:Bag>
                    <rdf:li>
                      <mwg-rs:Name>Bob</mwg-rs:Name>
                      <mwg-rs:Type>Face</mwg-rs:Type>
                      <mwg-rs:Area stArea:x="0.8" stArea:y="0.4" stArea:w="0.1" stArea:h="0.2"/>
                    </rdf:li>
                    <rdf:li>
                      <mwg-rs:Name>Alice</mwg-rs:Name>
                      <mwg-rs:Type>Face</mwg-rs:Type>
                      <mwg-rs:Area stArea:x="0.2" stArea:y="0.4" stArea:w="0.1" stArea:h="0.2"/>
                    </rdf:li>
                  </rdf:Bag>
                </mwg-rs:RegionList>
              </mwg-rs:Regions>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>"#;

        let tags = parse_face_tags(xmp);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "Bob");
        assert!((tags[0].center_x - 0.8).abs() < 1e-6);
        assert_eq!(tags[1].name, "Alice");
        assert!((tags[1].center_x - 0.2).abs() < 1e-6);
    }

    #[test]
    fn ignores_non_face_regions() {
        let xmp = r#"<x:xmpmeta><rdf:li>
            <mwg-rs:Name>Pet</mwg-rs:Name>
            <mwg-rs:Type>Pet</mwg-rs:Type>
            <mwg-rs:Area stArea:x="0.5"/>
          </rdf:li></x:xmpmeta>"#;
        assert!(parse_face_tags(xmp).is_empty());
    }

    #[test]
    fn parses_microsoft_people_tags() {
        let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:MPReg="http://ns.microsoft.com/photo/1.2/t/Region#">
              <MPRI:Regions xmlns:MPRI="http://ns.microsoft.com/photo/1.2/t/RegionInfo#">
                <rdf:Bag>
                  <rdf:li>
                    <MPReg:Rectangle>0.10, 0.20, 0.20, 0.30</MPReg:Rectangle>
                    <MPReg:PersonDisplayName>Carol</MPReg:PersonDisplayName>
                  </rdf:li>
                </rdf:Bag>
              </MPRI:Regions>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>"#;

        let tags = parse_face_tags(xmp);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Carol");
        // centre x = 0.10 + 0.20/2 = 0.20
        assert!((tags[0].center_x - 0.20).abs() < 1e-6);
    }

    #[test]
    fn empty_when_no_xmp() {
        assert!(parse_face_tags("<html>no xmp here</html>").is_empty());
    }
}
