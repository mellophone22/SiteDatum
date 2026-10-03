use std::path::Path;

use lopdf::Document;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const SETTINGS_KEY: &str = "rfi_pdf_settings";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RfiPdfSettings {
    pub template_mode: RfiTemplateMode,
    pub company_name: String,
    pub company_details: String,
    pub accent_color: String,
    pub custom_template_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RfiTemplateMode {
    SiteDatum,
    Custom,
}

impl Default for RfiPdfSettings {
    fn default() -> Self {
        Self {
            template_mode: RfiTemplateMode::SiteDatum,
            company_name: String::new(),
            company_details: String::new(),
            accent_color: "#087F89".into(),
            custom_template_path: None,
        }
    }
}

impl RfiPdfSettings {
    fn normalized_values(mut self) -> AppResult<Self> {
        self.company_name = self.company_name.trim().chars().take(120).collect();
        self.company_details = self.company_details.trim().chars().take(360).collect();
        self.accent_color = normalize_color(&self.accent_color)?;
        self.custom_template_path = self
            .custom_template_path
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        Ok(self)
    }

    pub fn normalized(self) -> AppResult<Self> {
        let settings = self.normalized_values()?;
        if settings.template_mode == RfiTemplateMode::Custom {
            let path = settings.custom_template_path.as_deref().ok_or_else(|| {
                settings_error(
                    "RFI_TEMPLATE_REQUIRED",
                    "Choose a custom PDF template.",
                    "Select a one-page, portrait Letter PDF and save the settings again.",
                    "Custom mode did not include a path.",
                )
            })?;
            validate_custom_template(Path::new(path))?;
        }
        Ok(settings)
    }
}

pub fn decode(value: Option<String>) -> AppResult<RfiPdfSettings> {
    match value {
        Some(value) => serde_json::from_str::<RfiPdfSettings>(&value)
            .map_err(|error| {
                settings_error(
                    "RFI_SETTINGS_INVALID",
                    "The saved RFI PDF settings could not be read.",
                    "Open Settings, review RFI PDF branding, and save it again.",
                    error.to_string(),
                )
            })?
            .normalized_values(),
        None => Ok(RfiPdfSettings::default()),
    }
}

pub fn encode(settings: RfiPdfSettings) -> AppResult<(RfiPdfSettings, String)> {
    let normalized = settings.normalized()?;
    let value = serde_json::to_string(&normalized).map_err(|error| {
        settings_error(
            "RFI_SETTINGS_INVALID",
            "The RFI PDF settings could not be prepared.",
            "Review the branding fields and try again.",
            error.to_string(),
        )
    })?;
    Ok((normalized, value))
}

pub fn validate_custom_template(path: &Path) -> AppResult<()> {
    if !path.is_absolute()
        || path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| !value.eq_ignore_ascii_case("pdf"))
            .unwrap_or(true)
    {
        return Err(settings_error(
            "RFI_TEMPLATE_PATH_INVALID",
            "Choose an absolute PDF template path.",
            "Use Browse to select a local one-page PDF.",
            path.display().to_string(),
        ));
    }
    if !path.is_file() {
        return Err(settings_error(
            "RFI_TEMPLATE_UNAVAILABLE",
            "The custom RFI template is unavailable.",
            "Reconnect its drive or choose another template in Settings.",
            path.display().to_string(),
        ));
    }
    let document = Document::load(path).map_err(|error| {
        settings_error(
            "RFI_TEMPLATE_INVALID",
            "The custom RFI template could not be opened.",
            "Choose an unencrypted, one-page portrait Letter PDF.",
            error.to_string(),
        )
    })?;
    let pages = document.get_pages();
    if pages.len() != 1 {
        return Err(settings_error(
            "RFI_TEMPLATE_PAGE_COUNT_INVALID",
            "The custom RFI template must contain exactly one page.",
            "Export a one-page template and select it again.",
            format!("{} pages", pages.len()),
        ));
    }
    let page_id = pages[&1];
    let page = document
        .get_object(page_id)
        .and_then(|value| value.as_dict())
        .map_err(|error| {
            settings_error(
                "RFI_TEMPLATE_INVALID",
                "The custom RFI template page is damaged.",
                "Choose another unencrypted PDF.",
                error.to_string(),
            )
        })?;
    let media_box = page
        .get(b"MediaBox")
        .and_then(|value| value.as_array())
        .map_err(|error| {
            settings_error(
                "RFI_TEMPLATE_SIZE_INVALID",
                "The custom RFI template has no readable page size.",
                "Export it as a portrait Letter PDF and try again.",
                error.to_string(),
            )
        })?;
    let dimensions = media_box
        .iter()
        .map(|value| value.as_float())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            settings_error(
                "RFI_TEMPLATE_SIZE_INVALID",
                "The custom RFI template page size could not be read.",
                "Export it as a portrait Letter PDF and try again.",
                error.to_string(),
            )
        })?;
    if dimensions.len() != 4 {
        return Err(settings_error(
            "RFI_TEMPLATE_SIZE_INVALID",
            "The custom RFI template page size is invalid.",
            "Export it as a portrait Letter PDF and try again.",
            format!("MediaBox entries: {}", dimensions.len()),
        ));
    }
    let width = (dimensions[2] - dimensions[0]).abs();
    let height = (dimensions[3] - dimensions[1]).abs();
    if (width - 612.0).abs() > 3.0 || (height - 792.0).abs() > 3.0 {
        return Err(settings_error(
            "RFI_TEMPLATE_SIZE_INVALID",
            "The custom RFI template must use portrait US Letter size.",
            "Export it at 8.5 × 11 inches and select it again.",
            format!("{width} x {height} points"),
        ));
    }
    Ok(())
}

fn normalize_color(value: &str) -> AppResult<String> {
    let value = value.trim().to_ascii_uppercase();
    let valid = value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit());
    valid.then_some(value.clone()).ok_or_else(|| {
        settings_error(
            "RFI_BRAND_COLOR_INVALID",
            "Enter the accent color as a six-digit hex value.",
            "Use a value such as #087F89.",
            value,
        )
    })
}

fn settings_error(
    code: &'static str,
    message: &str,
    recovery: &str,
    detail: impl Into<String>,
) -> AppError {
    AppError::from_technical(code, message, recovery, detail.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_neutral_sitedatum_template() {
        let settings = decode(None).unwrap();
        assert_eq!(settings.template_mode, RfiTemplateMode::SiteDatum);
        assert_eq!(settings.accent_color, "#087F89");
        assert!(settings.custom_template_path.is_none());
    }

    #[test]
    fn rejects_invalid_brand_color() {
        let error = RfiPdfSettings {
            accent_color: "teal".into(),
            ..RfiPdfSettings::default()
        }
        .normalized()
        .unwrap_err();
        assert_eq!(error.code, "RFI_BRAND_COLOR_INVALID");
    }

    #[test]
    fn reads_an_unavailable_custom_reference_for_recovery() {
        let value = r##"{"templateMode":"custom","companyName":"","companyDetails":"","accentColor":"#087F89","customTemplatePath":"C:\\Unavailable\\customer-rfi.pdf"}"##;
        let settings = decode(Some(value.into())).unwrap();
        assert_eq!(settings.template_mode, RfiTemplateMode::Custom);
        assert_eq!(
            settings.custom_template_path.as_deref(),
            Some("C:\\Unavailable\\customer-rfi.pdf")
        );
    }
}
