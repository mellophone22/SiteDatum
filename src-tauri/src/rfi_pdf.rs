use std::{
    fs,
    path::{Path, PathBuf},
};

use encoding_rs::WINDOWS_1252;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object,
};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    project::Project,
    rfi::Rfi,
};

const TEMPLATE: &[u8] = include_bytes!("../resources/templates/blank-rfi-template.pdf");

fn pdf_error(
    code: &'static str,
    message: &str,
    recovery: &str,
    detail: impl Into<String>,
) -> AppError {
    AppError::from_technical(code, message, recovery, detail.into())
}

fn safe_text(value: &str) -> String {
    value
        .replace(
            ['\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}'],
            "-",
        )
        .replace(['\u{2018}', '\u{2019}'], "'")
        .replace(['\u{201c}', '\u{201d}'], "\"")
}

fn format_date(value: &str) -> String {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() == 3 {
        format!("{}/{}/{}", parts[1], parts[2], parts[0])
    } else {
        value.to_owned()
    }
}

fn wrap_text(
    value: &str,
    max_characters: usize,
    max_lines: usize,
    field: &str,
) -> AppResult<Vec<String>> {
    let mut lines = Vec::new();
    for paragraph in safe_text(value).lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            if word.chars().count() > max_characters {
                return Err(pdf_error(
                    "RFI_PDF_WORD_TOO_LONG",
                    &format!("{field} contains a word that will not fit the PDF template."),
                    "Add spaces or shorten that text, then create the PDF again.",
                    word,
                ));
            }
            let proposed = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if proposed.chars().count() > max_characters {
                lines.push(current);
                current = word.to_owned();
            } else {
                current = proposed;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.len() > max_lines {
        return Err(pdf_error(
            "RFI_PDF_CONTENT_TOO_LONG",
            &format!("{field} is too long for the one-page RFI template."),
            "Shorten that field or move supporting detail to an attachment, then create the PDF again.",
            format!("{field}: {} lines; maximum {max_lines}", lines.len()),
        ));
    }
    Ok(lines)
}

fn text_operations(
    font: &str,
    size: f64,
    x: f64,
    y: f64,
    lines: &[String],
    leading: f64,
) -> Vec<Operation> {
    let mut operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("rg", vec![0.into(), 0.into(), 0.into()]),
        Operation::new(
            "Tf",
            vec![Object::Name(font.as_bytes().to_vec()), size.into()],
        ),
    ];
    for (index, line) in lines.iter().enumerate() {
        operations.push(Operation::new(
            "Tm",
            vec![
                1.into(),
                0.into(),
                0.into(),
                1.into(),
                x.into(),
                (y - index as f64 * leading).into(),
            ],
        ));
        let (encoded, _, _) = WINDOWS_1252.encode(line);
        operations.push(Operation::new(
            "Tj",
            vec![Object::string_literal(encoded.into_owned())],
        ));
    }
    operations.push(Operation::new("ET", vec![]));
    operations
}

fn add_block(
    operations: &mut Vec<Operation>,
    value: &str,
    field: &str,
    font: &str,
    size: f64,
    x: f64,
    y: f64,
    width: f64,
    max_lines: usize,
    leading: f64,
) -> AppResult<()> {
    if value.trim().is_empty() {
        return Ok(());
    }
    let max_characters = (width / (size * 0.52)).floor().max(1.0) as usize;
    let lines = wrap_text(value, max_characters, max_lines, field)?;
    operations.extend(text_operations(font, size, x, y, &lines, leading));
    Ok(())
}

fn template_document() -> AppResult<(Document, lopdf::ObjectId)> {
    let mut document = Document::load_mem(TEMPLATE).map_err(|error| {
        pdf_error(
            "RFI_PDF_TEMPLATE_INVALID",
            "The bundled RFI PDF template could not be opened.",
            "Reinstall AnyDesk and try again.",
            error.to_string(),
        )
    })?;
    let page_id = document.get_pages().get(&1).copied().ok_or_else(|| {
        pdf_error(
            "RFI_PDF_TEMPLATE_INVALID",
            "The bundled RFI PDF template has no first page.",
            "Reinstall AnyDesk and try again.",
            "Missing page 1.",
        )
    })?;
    let body_font = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let bold_font = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
        "Encoding" => "WinAnsiEncoding",
    });
    let page = document
        .get_object_mut(page_id)
        .and_then(Object::as_dict_mut)
        .map_err(|error| {
            pdf_error(
                "RFI_PDF_TEMPLATE_INVALID",
                "The bundled RFI PDF template is damaged.",
                "Reinstall AnyDesk and try again.",
                error.to_string(),
            )
        })?;
    let resources = page
        .get_mut(b"Resources")
        .and_then(Object::as_dict_mut)
        .map_err(|error| {
            pdf_error(
                "RFI_PDF_TEMPLATE_INVALID",
                "The bundled RFI PDF template resources are damaged.",
                "Reinstall AnyDesk and try again.",
                error.to_string(),
            )
        })?;
    let fonts = resources
        .get_mut(b"Font")
        .and_then(Object::as_dict_mut)
        .map_err(|error| {
            pdf_error(
                "RFI_PDF_TEMPLATE_INVALID",
                "The bundled RFI PDF template fonts are damaged.",
                "Reinstall AnyDesk and try again.",
                error.to_string(),
            )
        })?;
    fonts.set("ADBody", body_font);
    fonts.set("ADBold", bold_font);
    Ok((document, page_id))
}

pub fn suggested_path(rfi: &Rfi, project: &Project) -> PathBuf {
    let folder = if rfi.status == "closed" {
        "Closed"
    } else {
        "Open"
    };
    let raw = format!("{} - {}", rfi.number, rfi.subject);
    let mut safe = raw
        .chars()
        .map(|character| {
            if matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) {
                '-'
            } else {
                character
            }
        })
        .collect::<String>();
    safe = safe.split_whitespace().collect::<Vec<_>>().join(" ");
    safe = safe.trim_matches([' ', '.']).chars().take(120).collect();
    if safe.is_empty() {
        safe = "RFI".into();
    }
    Path::new(&project.project_path)
        .join("03 RFIs")
        .join(folder)
        .join(format!("{safe}.pdf"))
}

pub fn write(rfi: &Rfi, project: &Project, destination: &Path) -> AppResult<()> {
    if !destination.is_absolute()
        || destination
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| !value.eq_ignore_ascii_case("pdf"))
            .unwrap_or(true)
    {
        return Err(pdf_error(
            "RFI_PDF_PATH_INVALID",
            "Choose an absolute PDF file path.",
            "Use the Save PDF dialog and keep the .pdf extension.",
            destination.display().to_string(),
        ));
    }
    if destination.exists() {
        return Err(pdf_error(
            "RFI_PDF_ALREADY_EXISTS",
            "A PDF already exists at that location.",
            "Choose a different filename. AnyDesk never overwrites an existing document.",
            destination.display().to_string(),
        ));
    }
    let parent = destination.parent().ok_or_else(|| {
        pdf_error(
            "RFI_PDF_PATH_INVALID",
            "The PDF destination is invalid.",
            "Choose another folder and try again.",
            destination.display().to_string(),
        )
    })?;
    if !parent.is_dir() {
        return Err(pdf_error(
            "RFI_PDF_FOLDER_UNAVAILABLE",
            "The PDF folder is unavailable.",
            "Reconnect the project drive or choose another existing folder.",
            parent.display().to_string(),
        ));
    }

    let (mut document, page_id) = template_document()?;
    let mut operations = vec![Operation::new("q", vec![])];
    add_block(
        &mut operations,
        &rfi.number,
        "RFI number",
        "ADBody",
        10.0,
        414.0,
        698.0,
        140.0,
        1,
        11.0,
    )?;
    add_block(
        &mut operations,
        &format_date(&rfi.created_date),
        "RFI date",
        "ADBody",
        10.0,
        414.0,
        682.0,
        140.0,
        1,
        11.0,
    )?;
    add_block(
        &mut operations,
        &project.name,
        "Job name",
        "ADBody",
        9.0,
        414.0,
        666.0,
        140.0,
        2,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.recipient.as_deref().unwrap_or(""),
        "Recipient",
        "ADBody",
        10.0,
        68.0,
        588.0,
        218.0,
        3,
        12.0,
    )?;
    add_block(
        &mut operations,
        project.location.as_deref().unwrap_or(""),
        "Project location",
        "ADBody",
        10.0,
        343.0,
        588.0,
        216.0,
        3,
        12.0,
    )?;
    add_block(
        &mut operations,
        rfi.cost_impact.as_deref().unwrap_or(""),
        "Cost impact",
        "ADBody",
        9.0,
        164.0,
        521.0,
        124.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.time_delay.as_deref().unwrap_or(""),
        "Time delay",
        "ADBody",
        9.0,
        164.0,
        508.0,
        124.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.rfi_location.as_deref().unwrap_or(""),
        "RFI location",
        "ADBody",
        9.0,
        414.0,
        521.0,
        145.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.drawing_number.as_deref().unwrap_or(""),
        "Drawing number",
        "ADBody",
        9.0,
        414.0,
        508.0,
        145.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        &format!("Subject: {}", rfi.subject),
        "RFI subject",
        "ADBold",
        9.5,
        68.0,
        461.0,
        490.0,
        2,
        11.0,
    )?;
    add_block(
        &mut operations,
        &rfi.question,
        "RFI question",
        "ADBody",
        9.5,
        68.0,
        437.0,
        490.0,
        13,
        11.0,
    )?;
    add_block(
        &mut operations,
        rfi.suggested_solution.as_deref().unwrap_or(""),
        "Suggested solution",
        "ADBody",
        9.5,
        68.0,
        272.0,
        490.0,
        13,
        11.0,
    )?;
    add_block(
        &mut operations,
        rfi.requested_by.as_deref().unwrap_or(""),
        "Requested by",
        "ADBody",
        10.0,
        414.0,
        87.0,
        145.0,
        1,
        11.0,
    )?;
    operations.push(Operation::new("Q", vec![]));
    let content = Content { operations }.encode().map_err(|error| {
        pdf_error(
            "RFI_PDF_CREATE_FAILED",
            "The RFI PDF could not be prepared.",
            "Review the RFI information and try again.",
            error.to_string(),
        )
    })?;
    document
        .add_page_contents(page_id, content)
        .map_err(|error| {
            pdf_error(
                "RFI_PDF_CREATE_FAILED",
                "The RFI PDF could not be prepared.",
                "Review the RFI information and try again.",
                error.to_string(),
            )
        })?;

    let temporary = parent.join(format!(".anydesk-rfi-{}.tmp", Uuid::new_v4()));
    let result = document.save(&temporary).map_err(|error| {
        pdf_error(
            "RFI_PDF_WRITE_FAILED",
            "The RFI PDF could not be saved.",
            "Check folder permissions and available disk space, then try again.",
            error.to_string(),
        )
    });
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(pdf_error(
            "RFI_PDF_WRITE_FAILED",
            "The RFI PDF could not be finalized.",
            "Check folder permissions and confirm that the filename is not already in use.",
            error.to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rfi() -> Rfi {
        Rfi {
            id: "r1".into(),
            project_id: "p1".into(),
            project_number: "P-100".into(),
            project_name: "Test Project".into(),
            number: "RFI-012".into(),
            subject: "Clarify valve sequence".into(),
            question: "Which interlock is required for the control valve sequence?".into(),
            recipient: Some("Design Engineer".into()),
            status: "open".into(),
            created_date: "2026-09-22".into(),
            submitted_date: None,
            response_due_date: None,
            response_received_date: None,
            response: None,
            notes: None,
            rfi_location: Some("Mechanical room".into()),
            drawing_number: Some("M-401".into()),
            cost_impact: Some("None anticipated".into()),
            time_delay: Some("None anticipated".into()),
            suggested_solution: Some("Use sequence B as shown in the controls narrative.".into()),
            requested_by: Some("Project Engineer".into()),
            related_task_id: None,
        }
    }

    fn project(path: &Path) -> Project {
        Project {
            id: "p1".into(),
            number: "P-100".into(),
            name: "Test Project".into(),
            status: "active".into(),
            phase: "engineering".into(),
            custom_phase_name: None,
            customer: None,
            general_contractor: None,
            engineer: None,
            project_manager: None,
            superintendent: None,
            location: Some("100 Main Street".into()),
            start_date: None,
            target_date: None,
            description: None,
            important_notes: None,
            project_path: path.to_string_lossy().into_owned(),
            is_pinned: false,
            archived_at_utc: None,
            created_at_utc: "2026-09-22T00:00:00Z".into(),
            updated_at_utc: "2026-09-22T00:00:00Z".into(),
        }
    }

    #[test]
    fn writes_template_pdf_and_refuses_overwrite() {
        let folder = std::env::temp_dir().join(format!("anydesk-rfi-pdf-{}", Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let destination = folder.join("rfi.pdf");
        write(&rfi(), &project(&folder), &destination).unwrap();
        let document = Document::load(&destination).unwrap();
        assert_eq!(document.get_pages().len(), 1);
        let text = document.extract_text(&[1]).unwrap();
        assert!(text.contains("RFI-012"));
        assert!(text.contains("Clarify valve sequence"));
        assert!(text.contains("Use sequence B"));
        if let Ok(review_path) = std::env::var("ANYDESK_RFI_PDF_REVIEW_PATH") {
            fs::copy(&destination, review_path).unwrap();
        }
        assert_eq!(
            write(&rfi(), &project(&folder), &destination)
                .unwrap_err()
                .code,
            "RFI_PDF_ALREADY_EXISTS"
        );
        fs::remove_dir_all(folder).unwrap();
    }
}
