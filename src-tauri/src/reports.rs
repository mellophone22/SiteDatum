use crate::{
    error::{AppError, AppResult},
    persistence::Database,
};
use std::{fs, path::Path};
const TEMPLATE_VERSION: &str = "1.0";
fn err(code: &'static str, message: &str, detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        code,
        message,
        "Choose a different destination and try again.",
        detail,
    )
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn row(cells: &[String]) -> String {
    format!(
        "<tr>{}</tr>",
        cells
            .iter()
            .map(|v| format!("<td>{}</td>", escape(v)))
            .collect::<String>()
    )
}
impl Database {
    pub fn export_operational_report(
        &self,
        report_type: &str,
        project_id: Option<&str>,
        output: &str,
    ) -> AppResult<String> {
        let path = Path::new(output);
        if path.exists() {
            return Err(err(
                "REPORT_COLLISION",
                "A file already exists at the selected location.",
                output,
            ));
        }
        if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| !v.eq_ignore_ascii_case("html"))
            .unwrap_or(true)
        {
            return Err(err(
                "REPORT_FORMAT_INVALID",
                "Reports must be saved as .html files.",
                output,
            ));
        }
        let project_name = project_id
            .and_then(|id| self.get_project(id).ok())
            .map(|p| format!("{} — {}", p.number, p.name))
            .unwrap_or_else(|| "All projects".into());
        let (title, headers, rows) = match report_type {
            "meeting_minutes" => {
                let values = self
                    .list_work_items()?
                    .into_iter()
                    .filter(|v| {
                        v.item_type == "meeting_minute"
                            && project_id.map(|id| v.project_id == id).unwrap_or(true)
                    })
                    .map(|v| {
                        vec![
                            v.occurred_date.unwrap_or_default(),
                            v.number.unwrap_or_default(),
                            v.title,
                            v.responsible_party.unwrap_or_default(),
                            v.status,
                        ]
                    })
                    .collect();
                (
                    "Meeting minutes register",
                    vec!["Date", "Meeting", "Subject", "Responsible", "Status"],
                    values,
                )
            }
            "transmittals" => {
                let values = self
                    .list_work_items()?
                    .into_iter()
                    .filter(|v| {
                        v.item_type == "transmittal"
                            && project_id.map(|id| v.project_id == id).unwrap_or(true)
                    })
                    .map(|v| {
                        vec![
                            v.number.unwrap_or_default(),
                            v.title,
                            v.company.unwrap_or_default(),
                            v.occurred_date.or(v.due_date).unwrap_or_default(),
                            v.status,
                        ]
                    })
                    .collect();
                (
                    "Transmittal register",
                    vec![
                        "Number",
                        "Description",
                        "Recipient / company",
                        "Date",
                        "Status",
                    ],
                    values,
                )
            }
            "contacts" => {
                let values = self
                    .list_contacts()?
                    .into_iter()
                    .map(|v| {
                        vec![
                            v.name,
                            v.company.unwrap_or_default(),
                            v.role.unwrap_or_default(),
                            v.email.unwrap_or_default(),
                            v.phone.unwrap_or_default(),
                        ]
                    })
                    .collect();
                (
                    "Project contact list",
                    vec!["Name", "Company", "Role", "Email", "Phone"],
                    values,
                )
            }
            "submittal_cover" => {
                let values = self
                    .list_submittals()?
                    .into_iter()
                    .filter(|v| project_id.map(|id| v.project_id == id).unwrap_or(true))
                    .map(|v| {
                        vec![
                            v.number,
                            v.name,
                            v.revision.unwrap_or_default(),
                            v.recipient.unwrap_or_default(),
                            v.status,
                        ]
                    })
                    .collect();
                (
                    "Submittal cover register",
                    vec!["Number", "Submittal", "Revision", "Recipient", "Status"],
                    values,
                )
            }
            "open_items" | "weekly_status" => {
                let mut values = self
                    .list_tasks()?
                    .into_iter()
                    .filter(|v| {
                        !["completed", "cancelled"].contains(&v.status.as_str())
                            && project_id.map(|id| v.project_id == id).unwrap_or(true)
                    })
                    .map(|v| {
                        vec![
                            "Task".into(),
                            v.title,
                            v.due_date.or(v.follow_up_date).unwrap_or_default(),
                            v.status,
                        ]
                    })
                    .collect::<Vec<_>>();
                values.extend(
                    self.list_work_items()?
                        .into_iter()
                        .filter(|v| {
                            !["completed", "cancelled"].contains(&v.status.as_str())
                                && project_id.map(|id| v.project_id == id).unwrap_or(true)
                        })
                        .map(|v| {
                            vec![
                                v.item_type.replace('_', " "),
                                v.title,
                                v.due_date.unwrap_or_default(),
                                v.status,
                            ]
                        }),
                );
                (
                    if report_type == "weekly_status" {
                        "Weekly project status"
                    } else {
                        "Open-item report"
                    },
                    vec!["Type", "Item", "Due", "Status"],
                    values,
                )
            }
            _ => {
                return Err(err(
                    "REPORT_TYPE_INVALID",
                    "Choose a valid report template.",
                    report_type,
                ))
            }
        };
        let heading = headers
            .iter()
            .map(|v| format!("<th>{}</th>", escape(v)))
            .collect::<String>();
        let body = rows.iter().map(|v| row(v)).collect::<String>();
        let html=format!("<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>body{{font:12px 'Segoe UI',Arial,sans-serif;color:#172033;margin:32px}}h1{{font-size:22px;margin-bottom:4px}}p{{color:#566176}}table{{width:100%;border-collapse:collapse;margin-top:20px}}th,td{{padding:8px;border:1px solid #cfd6e1;text-align:left;vertical-align:top}}th{{background:#eef2f7}}footer{{margin-top:20px;color:#697386;font-size:10px}}@media print{{body{{margin:12mm}}}}</style></head><body><h1>{}</h1><p>{}</p><table><thead><tr>{}</tr></thead><tbody>{}</tbody></table><footer>AnyDesk template {} · {} records</footer></body></html>",escape(title),escape(title),escape(&project_name),heading,body,TEMPLATE_VERSION,rows.len());
        let temporary = path.with_extension("html.tmp");
        fs::write(&temporary, html).map_err(|e| {
            err(
                "REPORT_WRITE_FAILED",
                "The report could not be created.",
                e.to_string(),
            )
        })?;
        fs::rename(&temporary, path).map_err(|e| {
            err(
                "REPORT_WRITE_FAILED",
                "The report could not be finalized.",
                e.to_string(),
            )
        })?;
        Ok(path.to_string_lossy().into_owned())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    #[test]
    fn creates_versioned_report_and_refuses_overwrite() {
        let root = std::env::temp_dir().join(format!("anydesk-report-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::open(&root.join("workspace.sqlite3")).unwrap();
        let output = root.join("report.html");
        db.export_operational_report("contacts", None, &output.to_string_lossy())
            .unwrap();
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.contains("AnyDesk template 1.0"));
        assert_eq!(
            db.export_operational_report("contacts", None, &output.to_string_lossy())
                .unwrap_err()
                .code,
            "REPORT_COLLISION"
        );
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
}
