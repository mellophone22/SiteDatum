//! Explicit, versioned content allowlist. Never serialize database rows or UI models.
#![allow(dead_code)] // No command, network transport, or enabled Sync feature.

use crate::error::{AppError, AppResult};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use zeroize::Zeroizing;

pub(crate) const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_CIPHERTEXT: usize = 262_144;
const NONCE_BYTES: usize = 24;
const RECORD_LABEL: &[u8] = b"sitedatum.sync-v2.record.v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordContent {
    pub schema_version: u16,
    #[serde(deserialize_with = "deserialize_fields")]
    pub fields: Option<Value>,
}
fn deserialize_fields<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    Option::<Value>::deserialize(deserializer)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecordHeader {
    pub owner_id: Uuid,
    pub workspace_id: Uuid,
    pub record_id: Uuid,
    pub record_kind: u8,
    pub mutation_id: Uuid,
    pub expected_server_version: u64,
    pub workspace_key_version: u32,
    pub protocol_version: u16,
    pub tombstone: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SealedRecord {
    pub header: RecordHeader,
    pub ciphertext: Vec<u8>,
}

// All fields are required in v1; '?' means nullable, not silently optional.
// s=text, u=UUID, b=boolean, i=integer, a=text array, p=portable reference.
const SCHEMAS: [&str; 13] = [
    "id:u number:s name:s status:s phase:s custom_phase_name:s? customer:s? general_contractor:s? engineer:s? project_manager:s? superintendent:s? location:s? start_date:s? target_date:s? description:s? important_notes:s? portable_root:p is_pinned:b archived_at_utc:s? created_at_utc:s updated_at_utc:s",
    "id:u project_id:u title:s description:s? priority:s status:s category:s? due_date:s? follow_up_date:s? waiting_since_utc:s? waiting_on:s? related_contact_id:u? created_at_utc:s updated_at_utc:s",
    "id:u project_id:u number:s subject:s question:s recipient:s? status:s created_date:s submitted_date:s? response_due_date:s? response_received_date:s? response:s? notes:s? rfi_location:s? drawing_number:s? cost_impact:s? time_delay:s? suggested_solution:s? requested_by:s? created_at_utc:s updated_at_utc:s",
    "id:u project_id:u parent_submittal_id:u? number:s name:s package:s? revision:s recipient:s? status:s disposition:s? created_date:s submitted_date:s? response_date:s? resubmission_required:b notes:s? created_at_utc:s updated_at_utc:s",
    "rfi_id:u task_id:u created_at_utc:s",
    "submittal_id:u task_id:u created_at_utc:s",
    "id:u rfi_id:u portable_path:p display_name:s created_at_utc:s",
    "id:u submittal_id:u portable_path:p display_name:s created_at_utc:s",
    "id:u project_id:u file_name:s portable_path:p operation:s discipline:s? drawing_number:s? title:s? revision:s? revision_date:s? received_date:s? is_current:b created_at_utc:s updated_at_utc:s",
    "id:u project_id:u? body:s created_at_utc:s updated_at_utc:s",
    "id:u name:s company:s? email:s? phone:s? role:s? created_at_utc:s updated_at_utc:s",
    "id:u event_type:s entity_type:s entity_id:u summary:s occurred_at_utc:s",
    // Work items and project templates share the hosted kind 13, with a strict subtype.
    "",
];
const WORK_ITEM: &str = "subtype:s id:u project_id:u item_type:s number:s? title:s description:s? status:s priority:s due_date:s? occurred_date:s? responsible_party:s? company:s? location:s? amount_cents:i? checklist_total:i checklist_completed:i archived_at_utc:s? created_at_utc:s updated_at_utc:s";
const TEMPLATE: &str =
    "subtype:s id:u name:s task_titles:a milestone_titles:a created_at_utc:s updated_at_utc:s";

pub(crate) fn validate_header(header: &RecordHeader) -> AppResult<()> {
    if header.protocol_version != 1
        || !(1..=13).contains(&header.record_kind)
        || header.workspace_key_version == 0
        || header.workspace_key_version > i32::MAX as u32
        || header.expected_server_version >= MAX_SAFE_INTEGER
        || [
            header.owner_id,
            header.workspace_id,
            header.record_id,
            header.mutation_id,
        ]
        .iter()
        .any(|id| {
            id.get_variant() != uuid::Variant::RFC4122 || !(1..=5).contains(&id.get_version_num())
        })
    {
        return Err(invalid());
    }
    Ok(())
}

pub(crate) fn validate_content(header: &RecordHeader, content: &RecordContent) -> AppResult<()> {
    validate_header(header)?;
    if content.schema_version != 1 || header.tombstone != content.fields.is_none() {
        return Err(invalid());
    }
    let Some(fields) = &content.fields else {
        return Ok(());
    };
    let object = fields.as_object().ok_or_else(invalid)?;
    let schema = if header.record_kind == 13 {
        match object.get("subtype").and_then(Value::as_str) {
            Some("work_item") => WORK_ITEM,
            Some("project_template") => TEMPLATE,
            _ => return Err(invalid()),
        }
    } else {
        SCHEMAS[header.record_kind as usize - 1]
    };
    if object.len() != schema.split_whitespace().count() {
        return Err(invalid());
    }
    for field in schema.split_whitespace() {
        let (name, rule) = field.split_once(':').expect("static schema");
        let value = object.get(name).ok_or_else(invalid)?;
        if value.is_null() && rule.ends_with('?') {
            continue;
        }
        let valid = match rule.as_bytes()[0] {
            b's' => value.as_str().is_some_and(|text| text.len() <= 131_072),
            b'u' => value
                .as_str()
                .and_then(|text| Uuid::parse_str(text).ok())
                .is_some_and(|id| !id.is_nil()),
            b'b' => value.is_boolean(),
            b'i' => value
                .as_i64()
                .is_some_and(|n| n.unsigned_abs() <= MAX_SAFE_INTEGER),
            b'a' => value.as_array().is_some_and(|items| {
                items.len() <= 1000
                    && items
                        .iter()
                        .all(|item| item.as_str().is_some_and(|s| s.len() <= 4096))
            }),
            b'p' => valid_portable_reference(value),
            _ => false,
        };
        if !valid {
            return Err(invalid());
        }
    }
    if let Some(id) = object.get("id") {
        if Uuid::parse_str(id.as_str().ok_or_else(invalid)?).map_err(|_| invalid())?
            != header.record_id
        {
            return Err(invalid());
        }
    }
    validate_domain(header.record_kind, object)?;
    Ok(())
}

fn validate_domain(kind: u8, object: &serde_json::Map<String, Value>) -> AppResult<()> {
    let text = |name: &str| object.get(name).and_then(Value::as_str);
    let choose = |name: &str, choices: &[&str]| -> AppResult<()> {
        if text(name).is_some_and(|s| choices.contains(&s)) {
            Ok(())
        } else {
            Err(invalid())
        }
    };
    if matches!(kind, 2 | 13) && text("subtype") != Some("project_template") {
        choose("priority", &["low", "medium", "high", "urgent"])?;
    }
    match kind {
        1 => {
            choose("status", &["active", "on_hold", "completed", "cancelled"])?;
            choose(
                "phase",
                &[
                    "preconstruction",
                    "engineering",
                    "submittals",
                    "procurement",
                    "construction",
                    "programming",
                    "startup",
                    "commissioning",
                    "closeout",
                    "custom",
                ],
            )?;
            if (text("phase") == Some("custom"))
                != text("custom_phase_name").is_some_and(|s| !s.trim().is_empty())
            {
                return Err(invalid());
            }
        }
        2 => {
            choose(
                "status",
                &[
                    "open",
                    "in_progress",
                    "waiting",
                    "blocked",
                    "completed",
                    "cancelled",
                ],
            )?;
            if text("status") == Some("waiting")
                && (!text("waiting_on").is_some_and(|s| !s.trim().is_empty())
                    || text("waiting_since_utc").is_none())
            {
                return Err(invalid());
            }
        }
        3 => choose("status", &["draft", "open", "response_received", "closed"])?,
        4 => {
            choose(
                "status",
                &[
                    "draft",
                    "preparing",
                    "submitted",
                    "under_review",
                    "approved",
                    "approved_as_noted",
                    "revise_and_resubmit",
                    "rejected",
                    "closed",
                ],
            )?;
            if text("disposition").is_some() {
                choose(
                    "disposition",
                    &[
                        "approved",
                        "approved_as_noted",
                        "revise_and_resubmit",
                        "rejected",
                    ],
                )?;
            }
            if (text("status") == Some("closed") && text("disposition").is_none())
                || (object.get("resubmission_required").and_then(Value::as_bool) == Some(true)
                    && text("disposition") != Some("revise_and_resubmit"))
            {
                return Err(invalid());
            }
        }
        9 => choose("operation", &["register", "copy", "move"])?,
        13 if text("subtype") == Some("work_item") => {
            choose(
                "item_type",
                &[
                    "meeting_minute",
                    "procurement",
                    "change_event",
                    "transmittal",
                    "milestone",
                    "punch_item",
                    "daily_report",
                    "startup_check",
                    "commissioning_check",
                    "commissioning_issue",
                ],
            )?;
            choose(
                "status",
                &[
                    "draft",
                    "open",
                    "in_progress",
                    "waiting",
                    "completed",
                    "cancelled",
                ],
            )?;
            let total = object["checklist_total"].as_i64().ok_or_else(invalid)?;
            let completed = object["checklist_completed"].as_i64().ok_or_else(invalid)?;
            if total < 0 || completed < 0 || completed > total {
                return Err(invalid());
            }
        }
        _ => {}
    }
    for (name, value) in object {
        if let Some(text) = value.as_str() {
            if name.ends_with("_date") {
                crate::task::validate_date(Some(text)).map_err(|_| invalid())?;
            }
            if name.ends_with("_at_utc") || name == "waiting_since_utc" {
                if !valid_utc(text) {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}
fn valid_utc(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() < 20
        || !bytes.is_ascii()
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes.last() != Some(&b'Z')
    {
        return false;
    }
    if crate::task::validate_date(Some(&text[..10])).is_err() {
        return false;
    }
    for (range, max) in [(11..13, 23), (14..16, 59), (17..19, 59)] {
        if !text[range.clone()].bytes().all(|b| b.is_ascii_digit())
            || text[range].parse::<u8>().map_or(true, |n| n > max)
        {
            return false;
        }
    }
    bytes.len() == 20
        || (bytes.len() <= 29
            && bytes[19] == b'.'
            && bytes.len() > 21
            && bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit))
}

fn valid_portable_reference(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 2 {
        return false;
    }
    let kind = object.get("kind").and_then(Value::as_str);
    let Some(components) = object.get("components").and_then(Value::as_array) else {
        return false;
    };
    if !matches!(kind, Some("workspace_relative" | "external_name"))
        || components.len() > 128
        || (kind == Some("external_name") && components.len() != 1)
    {
        return false;
    }
    components.iter().all(|component| {
        component.as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 255
                && s != "."
                && s != ".."
                && !s.ends_with(['.', ' '])
                && !s
                    .chars()
                    .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        })
    })
}

fn aad(header: &RecordHeader) -> Vec<u8> {
    let mut bytes = RECORD_LABEL.to_vec();
    for id in [
        header.owner_id,
        header.workspace_id,
        header.record_id,
        header.mutation_id,
    ] {
        bytes.extend_from_slice(id.as_bytes());
    }
    bytes.extend_from_slice(&header.protocol_version.to_be_bytes());
    bytes.push(header.record_kind);
    bytes.extend_from_slice(&header.workspace_key_version.to_be_bytes());
    bytes.extend_from_slice(&header.expected_server_version.to_be_bytes());
    bytes.push(u8::from(header.tombstone));
    bytes
}

pub(crate) fn seal_record(
    header: RecordHeader,
    key: &[u8; 32],
    content: &RecordContent,
) -> AppResult<SealedRecord> {
    validate_content(&header, content)?;
    let plaintext = Zeroizing::new(serde_json::to_vec(content).map_err(|_| invalid())?);
    if plaintext.len() + NONCE_BYTES + 16 > MAX_CIPHERTEXT {
        return Err(invalid());
    }
    let mut nonce = [0; NONCE_BYTES];
    getrandom::fill(&mut nonce).map_err(|_| invalid())?;
    let cipher = XChaCha20Poly1305::new(key.into());
    let encrypted = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plaintext,
                aad: &aad(&header),
            },
        )
        .map_err(|_| invalid())?;
    let mut ciphertext = nonce.to_vec();
    ciphertext.extend_from_slice(&encrypted);
    Ok(SealedRecord { header, ciphertext })
}

pub(crate) fn open_record(record: &SealedRecord, key: &[u8; 32]) -> AppResult<RecordContent> {
    validate_header(&record.header)?;
    if !(NONCE_BYTES + 16..=MAX_CIPHERTEXT).contains(&record.ciphertext.len()) {
        return Err(invalid());
    }
    let (nonce, encrypted) = record.ciphertext.split_at(NONCE_BYTES);
    let plaintext = Zeroizing::new(
        XChaCha20Poly1305::new(key.into())
            .decrypt(
                XNonce::from_slice(nonce),
                Payload {
                    msg: encrypted,
                    aad: &aad(&record.header),
                },
            )
            .map_err(|_| invalid())?,
    );
    let content: RecordContent = serde_json::from_slice(&plaintext).map_err(|_| invalid())?;
    validate_content(&record.header, &content)?;
    Ok(content)
}

pub(crate) fn invalid() -> AppError {
    AppError::from_technical(
        "SYNC_V2_RECORD_INVALID",
        "The encrypted Sync record could not be verified.",
        "Keep the local workspace and retry after reviewing Sync status.",
        "Record schema, bounds, or authentication failed.",
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn header(kind: u8) -> RecordHeader {
        RecordHeader {
            owner_id: Uuid::from_u128(0x91000000000040008000000000000001),
            workspace_id: Uuid::from_u128(0x92000000000040008000000000000001),
            record_id: Uuid::new_v4(),
            record_kind: kind,
            mutation_id: Uuid::new_v4(),
            expected_server_version: 0,
            workspace_key_version: 1,
            protocol_version: 1,
            tombstone: false,
        }
    }
    pub(crate) fn note(h: &RecordHeader, body: &str) -> RecordContent {
        RecordContent {
            schema_version: 1,
            fields: Some(
                json!({"id":h.record_id.to_string(),"project_id":null,"body":body,"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}),
            ),
        }
    }
    #[test]
    fn ciphertext_and_every_authenticated_header_resist_substitution() {
        let h = header(10);
        let content = note(&h, "Fictional confidential engineering note");
        let record = seal_record(h.clone(), &[7; 32], &content).unwrap();
        assert_eq!(open_record(&record, &[7; 32]).unwrap(), content);
        assert!(!record.ciphertext.windows(12).any(|w| w == b"confidential"));
        assert!(open_record(&record, &[8; 32]).is_err());
        let mut candidates = Vec::new();
        let mut x = h.clone();
        x.owner_id = Uuid::new_v4();
        candidates.push(x);
        let mut x = h.clone();
        x.workspace_id = Uuid::new_v4();
        candidates.push(x);
        let mut x = h.clone();
        x.record_id = Uuid::new_v4();
        candidates.push(x);
        let mut x = h.clone();
        x.mutation_id = Uuid::new_v4();
        candidates.push(x);
        let mut x = h.clone();
        x.record_kind = 11;
        candidates.push(x);
        let mut x = h.clone();
        x.expected_server_version = 1;
        candidates.push(x);
        let mut x = h.clone();
        x.workspace_key_version = 2;
        candidates.push(x);
        let mut x = h.clone();
        x.protocol_version = 2;
        candidates.push(x);
        let mut x = h.clone();
        x.tombstone = true;
        candidates.push(x);
        for header in candidates {
            assert!(open_record(
                &SealedRecord {
                    header,
                    ciphertext: record.ciphertext.clone()
                },
                &[7; 32]
            )
            .is_err());
        }
        let mut corrupt = record.clone();
        corrupt.ciphertext[30] ^= 1;
        assert!(open_record(&corrupt, &[7; 32]).is_err());
        // Independent seals use new OS randomness; retries reuse the stored seal.
        assert_ne!(
            record.ciphertext,
            seal_record(h, &[7; 32], &content).unwrap().ciphertext
        );
    }
    #[test]
    fn strict_schema_refuses_new_fields_missing_fields_skew_and_wrong_types() {
        let h = header(10);
        let good = note(&h, "Fictional note");
        let mut unknown = good.clone();
        unknown.fields.as_mut().unwrap()["file_path"] = json!("C:\\private\\document.pdf");
        let mut missing = good.clone();
        missing
            .fields
            .as_mut()
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("body");
        let mut wrong = good.clone();
        wrong.fields.as_mut().unwrap()["project_id"] = json!(17);
        let mut skew = good.clone();
        skew.schema_version = 2;
        let mut identity = good.clone();
        identity.fields.as_mut().unwrap()["id"] = json!(Uuid::new_v4().to_string());
        let mut timestamp = good.clone();
        timestamp.fields.as_mut().unwrap()["created_at_utc"] = json!("2026-10-10T99:00:00Z");
        assert!(serde_json::from_str::<RecordContent>(r#"{"schema_version":1}"#).is_err());
        for content in [unknown, missing, wrong, skew, identity, timestamp] {
            assert!(seal_record(h.clone(), &[7; 32], &content).is_err());
        }
        let mut extreme = h;
        extreme.expected_server_version = u64::MAX;
        assert!(seal_record(extreme, &[7; 32], &good).is_err());
    }
    #[test]
    fn every_kind_has_content_free_authenticated_tombstones() {
        for kind in 1..=13 {
            let mut h = header(kind);
            h.tombstone = true;
            let tombstone = RecordContent {
                schema_version: 1,
                fields: None,
            };
            let sealed = seal_record(h.clone(), &[7; 32], &tombstone).unwrap();
            assert_eq!(open_record(&sealed, &[7; 32]).unwrap(), tombstone);
            assert!(seal_record(h.clone(), &[7; 32], &note(&h, "deleted plaintext")).is_err());
        }
    }
    #[test]
    fn explicit_entity_fixtures_round_trip_all_thirteen_kinds_and_template_subtype() {
        let id = "93000000-0000-4000-8000-000000000001";
        let time = "2026-10-10T00:00:00Z";
        let path = json!({"kind":"workspace_relative","components":["Fictional","drawing.pdf"]});
        let fixtures = vec![
            json!({"id":id,"number":"F-1","name":"Fictional project","status":"active","phase":"construction","custom_phase_name":null,"customer":null,"general_contractor":null,"engineer":null,"project_manager":null,"superintendent":null,"location":null,"start_date":null,"target_date":null,"description":null,"important_notes":null,"portable_root":path,"is_pinned":false,"archived_at_utc":null,"created_at_utc":time,"updated_at_utc":time}),
            json!({"id":id,"project_id":id,"title":"Fictional task","description":null,"priority":"medium","status":"open","category":null,"due_date":null,"follow_up_date":null,"waiting_since_utc":null,"waiting_on":null,"related_contact_id":null,"created_at_utc":time,"updated_at_utc":time}),
            json!({"id":id,"project_id":id,"number":"F-1","subject":"Fictional RFI","question":"Fictional question","recipient":null,"status":"draft","created_date":"2026-10-10","submitted_date":null,"response_due_date":null,"response_received_date":null,"response":null,"notes":null,"rfi_location":null,"drawing_number":null,"cost_impact":null,"time_delay":null,"suggested_solution":null,"requested_by":null,"created_at_utc":time,"updated_at_utc":time}),
            json!({"id":id,"project_id":id,"parent_submittal_id":null,"number":"F-1","name":"Fictional submittal","package":null,"revision":"0","recipient":null,"status":"draft","disposition":null,"created_date":"2026-10-10","submitted_date":null,"response_date":null,"resubmission_required":false,"notes":null,"created_at_utc":time,"updated_at_utc":time}),
            json!({"rfi_id":id,"task_id":id,"created_at_utc":time}),
            json!({"submittal_id":id,"task_id":id,"created_at_utc":time}),
            json!({"id":id,"rfi_id":id,"portable_path":path,"display_name":"Fictional attachment","created_at_utc":time}),
            json!({"id":id,"submittal_id":id,"portable_path":path,"display_name":"Fictional attachment","created_at_utc":time}),
            json!({"id":id,"project_id":id,"file_name":"drawing.pdf","portable_path":path,"operation":"register","discipline":null,"drawing_number":null,"title":null,"revision":null,"revision_date":null,"received_date":null,"is_current":true,"created_at_utc":time,"updated_at_utc":time}),
            json!({"id":id,"project_id":null,"body":"Fictional note","created_at_utc":time,"updated_at_utc":time}),
            json!({"id":id,"name":"Fictional contact","company":null,"email":null,"phone":null,"role":null,"created_at_utc":time,"updated_at_utc":time}),
            json!({"id":id,"event_type":"created","entity_type":"task","entity_id":id,"summary":"Fictional event","occurred_at_utc":time}),
            json!({"subtype":"work_item","id":id,"project_id":id,"item_type":"milestone","number":null,"title":"Fictional milestone","description":null,"status":"open","priority":"medium","due_date":null,"occurred_date":null,"responsible_party":null,"company":null,"location":null,"amount_cents":null,"checklist_total":0,"checklist_completed":0,"archived_at_utc":null,"created_at_utc":time,"updated_at_utc":time}),
            json!({"subtype":"project_template","id":id,"name":"Fictional template","task_titles":["Review"],"milestone_titles":["Closeout"],"created_at_utc":time,"updated_at_utc":time}),
        ];
        for (index, fields) in fixtures.into_iter().enumerate() {
            let mut h = header((index + 1).min(13) as u8);
            h.record_id = Uuid::parse_str(id).unwrap();
            let content = RecordContent {
                schema_version: 1,
                fields: Some(fields),
            };
            assert_eq!(
                open_record(&seal_record(h, &[7; 32], &content).unwrap(), &[7; 32]).unwrap(),
                content
            );
        }
    }
    #[test]
    fn portable_references_never_accept_absolute_or_traversal_paths() {
        for component in [
            "..",
            ".",
            "C:\\secrets",
            "/etc",
            "folder/file",
            "file:stream",
            "bad\0name",
            "file.",
        ] {
            assert!(!valid_portable_reference(
                &json!({"kind":"workspace_relative","components":[component]})
            ));
        }
        assert!(!valid_portable_reference(
            &json!({"kind":"external_name","components":["a","b"]})
        ));
        assert!(!valid_portable_reference(
            &json!({"kind":"workspace_relative","components":[],"absolute_path":"C:\\secret"})
        ));
        assert!(valid_portable_reference(
            &json!({"kind":"external_name","components":["drawing.pdf"]})
        ));
    }
}
