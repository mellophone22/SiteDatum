use std::{
    fs,
    path::{Path, PathBuf},
};

use encoding_rs::WINDOWS_1252;
use lopdf::{
    content::{Content, Operation},
    dictionary, Dictionary, Document, Object,
};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    project::Project,
    rfi::Rfi,
    rfi_pdf_settings::{validate_custom_template, RfiPdfSettings, RfiTemplateMode},
};

const PAGE_WIDTH: f64 = 612.0;
const PAGE_HEIGHT: f64 = 792.0;

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
        return Err(pdf_error("RFI_PDF_CONTENT_TOO_LONG", &format!("{field} is too long for the one-page RFI template."), "Shorten that field or move supporting detail to an attachment, then create the PDF again.", format!("{field}: {} lines; maximum {max_lines}", lines.len())));
    }
    Ok(lines)
}

fn color_operation(operator: &str, color: (f64, f64, f64)) -> Operation {
    Operation::new(
        operator,
        vec![color.0.into(), color.1.into(), color.2.into()],
    )
}

#[allow(clippy::too_many_arguments)]
fn text_operations(
    font: &str,
    size: f64,
    x: f64,
    y: f64,
    lines: &[String],
    leading: f64,
    color: (f64, f64, f64),
) -> Vec<Operation> {
    let mut operations = vec![
        Operation::new("BT", vec![]),
        color_operation("rg", color),
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

#[allow(clippy::too_many_arguments)]
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
    operations.extend(text_operations(
        font,
        size,
        x,
        y,
        &lines,
        leading,
        (0.08, 0.11, 0.16),
    ));
    Ok(())
}

fn add_label(operations: &mut Vec<Operation>, value: &str, x: f64, y: f64) {
    operations.extend(text_operations(
        "SDBold",
        7.0,
        x,
        y,
        &[value.to_owned()],
        8.0,
        (0.34, 0.39, 0.46),
    ));
}

fn add_line(operations: &mut Vec<Operation>, x1: f64, y1: f64, x2: f64, y2: f64) {
    operations.extend([
        color_operation("RG", (0.76, 0.80, 0.85)),
        Operation::new("w", vec![0.7.into()]),
        Operation::new("m", vec![x1.into(), y1.into()]),
        Operation::new("l", vec![x2.into(), y2.into()]),
        Operation::new("S", vec![]),
    ]);
}

fn parse_color(value: &str) -> (f64, f64, f64) {
    let channel = |range| u8::from_str_radix(&value[range], 16).unwrap_or_default() as f64 / 255.0;
    (channel(1..3), channel(3..5), channel(5..7))
}

fn standard_background(settings: &RfiPdfSettings) -> AppResult<Vec<Operation>> {
    let accent = parse_color(&settings.accent_color);
    let company = if settings.company_name.is_empty() {
        "SiteDatum RFI"
    } else {
        &settings.company_name
    };
    let mut operations = vec![
        color_operation("rg", accent),
        Operation::new("re", vec![54.into(), 720.into(), 504.into(), 18.into()]),
        Operation::new("f", vec![]),
    ];
    operations.extend(text_operations(
        "SDBold",
        16.0,
        54.0,
        752.0,
        &[safe_text(company)],
        18.0,
        (0.08, 0.14, 0.25),
    ));
    if !settings.company_details.is_empty() {
        let lines = wrap_text(&settings.company_details, 62, 2, "Company details")?;
        operations.extend(text_operations(
            "SDBody",
            7.5,
            54.0,
            740.0,
            &lines,
            8.5,
            (0.34, 0.39, 0.46),
        ));
    }
    operations.extend(text_operations(
        "SDBold",
        11.0,
        64.0,
        724.0,
        &["REQUEST FOR INFORMATION".into()],
        12.0,
        (1.0, 1.0, 1.0),
    ));
    for (label, x, y) in [
        ("RFI NUMBER", 348.0, 704.0),
        ("DATE", 348.0, 686.0),
        ("PROJECT", 348.0, 668.0),
        ("TO", 54.0, 616.0),
        ("PROJECT LOCATION", 330.0, 616.0),
        ("COST IMPACT", 54.0, 538.0),
        ("TIME DELAY", 54.0, 520.0),
        ("RFI LOCATION", 330.0, 538.0),
        ("DRAWING", 330.0, 520.0),
        ("QUESTION", 54.0, 488.0),
        ("SUGGESTED SOLUTION", 54.0, 306.0),
        ("REQUESTED BY", 348.0, 104.0),
    ] {
        add_label(&mut operations, label, x, y);
    }
    for y in [714.0, 648.0, 548.0, 498.0, 316.0, 116.0, 72.0] {
        add_line(&mut operations, 54.0, y, 558.0, y);
    }
    add_line(&mut operations, 312.0, 648.0, 312.0, 548.0);
    add_line(&mut operations, 312.0, 548.0, 312.0, 498.0);
    operations.extend(text_operations(
        "SDBody",
        7.0,
        54.0,
        54.0,
        &["Generated locally by SiteDatum. Project records remain on this computer.".into()],
        8.0,
        (0.34, 0.39, 0.46),
    ));
    Ok(operations)
}

fn data_operations(rfi: &Rfi, project: &Project) -> AppResult<Vec<Operation>> {
    let mut operations = Vec::new();
    add_block(
        &mut operations,
        &rfi.number,
        "RFI number",
        "SDBody",
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
        "SDBody",
        10.0,
        414.0,
        680.0,
        140.0,
        1,
        11.0,
    )?;
    add_block(
        &mut operations,
        &project.name,
        "Job name",
        "SDBody",
        9.0,
        414.0,
        662.0,
        140.0,
        2,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.recipient.as_deref().unwrap_or(""),
        "Recipient",
        "SDBody",
        10.0,
        68.0,
        602.0,
        218.0,
        3,
        12.0,
    )?;
    add_block(
        &mut operations,
        project.location.as_deref().unwrap_or(""),
        "Project location",
        "SDBody",
        10.0,
        343.0,
        602.0,
        216.0,
        3,
        12.0,
    )?;
    add_block(
        &mut operations,
        rfi.cost_impact.as_deref().unwrap_or(""),
        "Cost impact",
        "SDBody",
        9.0,
        132.0,
        532.0,
        154.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.time_delay.as_deref().unwrap_or(""),
        "Time delay",
        "SDBody",
        9.0,
        132.0,
        514.0,
        154.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.rfi_location.as_deref().unwrap_or(""),
        "RFI location",
        "SDBody",
        9.0,
        414.0,
        532.0,
        144.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        rfi.drawing_number.as_deref().unwrap_or(""),
        "Drawing number",
        "SDBody",
        9.0,
        414.0,
        514.0,
        144.0,
        1,
        10.0,
    )?;
    add_block(
        &mut operations,
        &format!("Subject: {}", rfi.subject),
        "RFI subject",
        "SDBold",
        9.5,
        68.0,
        472.0,
        490.0,
        2,
        11.0,
    )?;
    add_block(
        &mut operations,
        &rfi.question,
        "RFI question",
        "SDBody",
        9.5,
        68.0,
        448.0,
        490.0,
        13,
        11.0,
    )?;
    add_block(
        &mut operations,
        rfi.suggested_solution.as_deref().unwrap_or(""),
        "Suggested solution",
        "SDBody",
        9.5,
        68.0,
        286.0,
        490.0,
        13,
        11.0,
    )?;
    add_block(
        &mut operations,
        rfi.requested_by.as_deref().unwrap_or(""),
        "Requested by",
        "SDBody",
        10.0,
        414.0,
        87.0,
        145.0,
        1,
        11.0,
    )?;
    Ok(operations)
}

fn blank_document() -> (Document, lopdf::ObjectId) {
    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let page_id = document.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()], "Resources" => Dictionary::new() });
    document.objects.insert(
        pages_id,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 },
        ),
    );
    let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    document.trailer.set("Root", catalog_id);
    (document, page_id)
}

fn template_document(settings: &RfiPdfSettings) -> AppResult<(Document, lopdf::ObjectId)> {
    match settings.template_mode {
        RfiTemplateMode::SiteDatum => Ok(blank_document()),
        RfiTemplateMode::Custom => {
            let path = settings.custom_template_path.as_deref().ok_or_else(|| {
                pdf_error(
                    "RFI_TEMPLATE_REQUIRED",
                    "No custom RFI PDF template is selected.",
                    "Choose a template in Settings or switch to the SiteDatum layout.",
                    "Custom mode has no path.",
                )
            })?;
            let path = Path::new(path);
            validate_custom_template(path)?;
            let document = Document::load(path).map_err(|error| {
                pdf_error(
                    "RFI_TEMPLATE_INVALID",
                    "The custom RFI template could not be opened.",
                    "Choose another template in Settings.",
                    error.to_string(),
                )
            })?;
            let page_id = document.get_pages()[&1];
            Ok((document, page_id))
        }
    }
}

fn add_overlay(
    document: &mut Document,
    page_id: lopdf::ObjectId,
    operations: Vec<Operation>,
) -> AppResult<()> {
    let body_font = document.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" });
    let bold_font = document.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica-Bold", "Encoding" => "WinAnsiEncoding" });
    add_page_resource(document, page_id, "Font", "SDBody", body_font)?;
    add_page_resource(document, page_id, "Font", "SDBold", bold_font)?;
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
        })
}

fn add_page_resource(
    document: &mut Document,
    page_id: lopdf::ObjectId,
    category: &str,
    name: &str,
    object_id: lopdf::ObjectId,
) -> AppResult<()> {
    let resources = document
        .get_object(page_id)
        .and_then(Object::as_dict)
        .ok()
        .and_then(|page| page.get(b"Resources").ok())
        .cloned();
    match resources {
        Some(Object::Reference(resources_id)) => {
            add_named_to_resources(document, resources_id, category, name, object_id)
        }
        Some(Object::Dictionary(mut resources)) => {
            add_named_to_dictionary(document, &mut resources, category, name, object_id)?;
            document
                .get_object_mut(page_id)
                .and_then(Object::as_dict_mut)
                .map_err(|error| invalid_template(error.to_string()))?
                .set("Resources", resources);
            Ok(())
        }
        None => {
            document
                .get_object_mut(page_id)
                .and_then(Object::as_dict_mut)
                .map_err(|error| invalid_template(error.to_string()))?
                .set(
                    "Resources",
                    dictionary! { category => dictionary! { name => object_id } },
                );
            Ok(())
        }
        Some(other) => Err(invalid_template(format!(
            "Unsupported Resources object: {other:?}"
        ))),
    }
}

fn add_named_to_resources(
    document: &mut Document,
    resources_id: lopdf::ObjectId,
    category: &str,
    name: &str,
    object_id: lopdf::ObjectId,
) -> AppResult<()> {
    let xobjects = document
        .get_object(resources_id)
        .and_then(Object::as_dict)
        .ok()
        .and_then(|resources| resources.get(category.as_bytes()).ok())
        .cloned();
    match xobjects {
        Some(Object::Reference(xobjects_id)) => document
            .get_object_mut(xobjects_id)
            .and_then(Object::as_dict_mut)
            .map_err(|error| invalid_template(error.to_string()))?
            .set(name, object_id),
        Some(Object::Dictionary(mut dictionary)) => {
            dictionary.set(name, object_id);
            document
                .get_object_mut(resources_id)
                .and_then(Object::as_dict_mut)
                .map_err(|error| invalid_template(error.to_string()))?
                .set(category, dictionary);
        }
        None => document
            .get_object_mut(resources_id)
            .and_then(Object::as_dict_mut)
            .map_err(|error| invalid_template(error.to_string()))?
            .set(category, dictionary! { name => object_id }),
        Some(other) => {
            return Err(invalid_template(format!(
                "Unsupported {category} resources: {other:?}"
            )))
        }
    }
    Ok(())
}

fn add_named_to_dictionary(
    document: &mut Document,
    resources: &mut Dictionary,
    category: &str,
    name: &str,
    object_id: lopdf::ObjectId,
) -> AppResult<()> {
    match resources.get(category.as_bytes()).ok().cloned() {
        Some(Object::Reference(xobjects_id)) => document
            .get_object_mut(xobjects_id)
            .and_then(Object::as_dict_mut)
            .map_err(|error| invalid_template(error.to_string()))?
            .set(name, object_id),
        Some(Object::Dictionary(mut dictionary)) => {
            dictionary.set(name, object_id);
            resources.set(category, dictionary);
        }
        None => resources.set(category, dictionary! { name => object_id }),
        Some(other) => {
            return Err(invalid_template(format!(
                "Unsupported {category} resources: {other:?}"
            )))
        }
    }
    Ok(())
}

fn invalid_template(detail: impl Into<String>) -> AppError {
    pdf_error(
        "RFI_TEMPLATE_INVALID",
        "The custom RFI template is not compatible with SiteDatum.",
        "Choose another one-page portrait Letter PDF in Settings.",
        detail,
    )
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

pub fn write(
    rfi: &Rfi,
    project: &Project,
    settings: &RfiPdfSettings,
    destination: &Path,
) -> AppResult<()> {
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
            "Choose a different filename. SiteDatum never overwrites an existing document.",
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
    let settings = settings.clone().normalized()?;
    let (mut document, page_id) = template_document(&settings)?;
    let mut operations = if settings.template_mode == RfiTemplateMode::SiteDatum {
        standard_background(&settings)?
    } else {
        Vec::new()
    };
    operations.extend(data_operations(rfi, project)?);
    add_overlay(&mut document, page_id, operations)?;
    let temporary = parent.join(format!(".sitedatum-rfi-{}.tmp", Uuid::new_v4()));
    if let Err(error) = document.save(&temporary).map_err(|error| {
        pdf_error(
            "RFI_PDF_WRITE_FAILED",
            "The RFI PDF could not be saved.",
            "Check folder permissions and available disk space, then try again.",
            error.to_string(),
        )
    }) {
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
    fn writes_neutral_branded_pdf_and_refuses_overwrite() {
        let folder = std::env::temp_dir().join(format!("sitedatum-rfi-pdf-{}", Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let destination = folder.join("rfi.pdf");
        let settings = RfiPdfSettings {
            company_name: "Northstar Controls".into(),
            company_details: "100 Main Street | support@example.test".into(),
            accent_color: "#087F89".into(),
            ..RfiPdfSettings::default()
        };
        write(&rfi(), &project(&folder), &settings, &destination).unwrap();
        let document = Document::load(&destination).unwrap();
        let text = document.extract_text(&[1]).unwrap();
        assert_eq!(document.get_pages().len(), 1);
        assert!(text.contains("Northstar Controls"));
        assert!(text.contains("RFI-012"));
        assert!(!text.contains("supplied company template"));
        if let Ok(review_path) = std::env::var("SITEDATUM_RFI_PDF_REVIEW_PATH") {
            fs::copy(&destination, review_path).unwrap();
        }
        assert_eq!(
            write(&rfi(), &project(&folder), &settings, &destination)
                .unwrap_err()
                .code,
            "RFI_PDF_ALREADY_EXISTS"
        );
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn custom_template_is_referenced_without_modifying_source() {
        let folder = std::env::temp_dir().join(format!("sitedatum-rfi-custom-{}", Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let template = folder.join("customer-template.pdf");
        let (mut source, page_id) = blank_document();
        add_overlay(
            &mut source,
            page_id,
            text_operations(
                "SDBold",
                12.0,
                54.0,
                752.0,
                &["CUSTOM CUSTOMER TEMPLATE".into()],
                14.0,
                (0.0, 0.0, 0.0),
            ),
        )
        .unwrap();
        source.save(&template).unwrap();
        let before = fs::read(&template).unwrap();
        let destination = folder.join("rfi.pdf");
        let settings = RfiPdfSettings {
            template_mode: RfiTemplateMode::Custom,
            custom_template_path: Some(template.to_string_lossy().into_owned()),
            ..RfiPdfSettings::default()
        };
        write(&rfi(), &project(&folder), &settings, &destination).unwrap();
        assert_eq!(fs::read(&template).unwrap(), before);
        let text = Document::load(&destination)
            .unwrap()
            .extract_text(&[1])
            .unwrap();
        assert!(text.contains("CUSTOM CUSTOMER TEMPLATE"));
        assert!(text.contains("RFI-012"));
        fs::remove_dir_all(folder).unwrap();
    }
}
