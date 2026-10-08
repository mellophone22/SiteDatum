use crate::{
    error::{AppError, AppResult},
    persistence::Database,
    work_item::{self, WorkItemInput},
};
use calamine::{open_workbook_auto, Reader};
use rust_xlsxwriter::Workbook;
use serde::Serialize;
use std::borrow::Cow;
use std::path::Path;
use uuid::Uuid;

const HEADERS: [&str; 12] = [
    "number",
    "title",
    "description",
    "status",
    "priority",
    "due_date",
    "occurred_date",
    "responsible_party",
    "company",
    "location",
    "amount",
    "checklist_total",
];

fn safe_csv_text(value: &str) -> Cow<'_, str> {
    match value.as_bytes().first() {
        Some(b'=' | b'+' | b'-' | b'@' | b'\t' | b'\r') => Cow::Owned(format!("'{value}")),
        _ => Cow::Borrowed(value),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub format: String,
    pub valid_rows: usize,
    pub errors: Vec<String>,
}

fn err(code: &'static str, message: &str, detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        code,
        message,
        "Check the selected file and try again.",
        detail,
    )
}
fn cells_to_input(cells: &[String], project_id: &str, item_type: &str) -> WorkItemInput {
    let get = |i: usize| {
        cells
            .get(i)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };
    let amount_cents = get(10)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| (v * 100.0).round() as i64);
    let total = get(11).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    WorkItemInput {
        project_id: project_id.into(),
        item_type: item_type.into(),
        number: get(0),
        title: get(1).unwrap_or_default(),
        description: get(2),
        status: get(3).unwrap_or_else(|| "open".into()),
        priority: get(4).unwrap_or_else(|| "medium".into()),
        due_date: get(5),
        occurred_date: get(6),
        responsible_party: get(7),
        company: get(8),
        location: get(9),
        amount_cents,
        checklist_total: total,
        checklist_completed: 0,
    }
}
fn read_rows(path: &Path) -> AppResult<Vec<Vec<String>>> {
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "csv" {
        let mut reader = csv::Reader::from_path(path).map_err(|e| {
            err(
                "IMPORT_READ_FAILED",
                "The CSV file could not be read.",
                e.to_string(),
            )
        })?;
        let headers = reader.headers().map_err(|e| {
            err(
                "IMPORT_HEADER_INVALID",
                "The CSV header could not be read.",
                e.to_string(),
            )
        })?;
        if !HEADERS
            .iter()
            .all(|h| headers.iter().any(|v| v.eq_ignore_ascii_case(h)))
        {
            return Err(err(
                "IMPORT_HEADER_INVALID",
                "The CSV does not contain the required columns.",
                headers.iter().collect::<Vec<_>>().join(","),
            ));
        }
        let positions: Vec<usize> = HEADERS
            .iter()
            .map(|h| {
                headers
                    .iter()
                    .position(|v| v.eq_ignore_ascii_case(h))
                    .unwrap()
            })
            .collect();
        return reader
            .records()
            .map(|record| {
                let r = record.map_err(|e| {
                    err(
                        "IMPORT_READ_FAILED",
                        "The CSV file could not be read.",
                        e.to_string(),
                    )
                })?;
                Ok(positions
                    .iter()
                    .map(|i| r.get(*i).unwrap_or("").to_owned())
                    .collect())
            })
            .collect();
    }
    if extension == "xlsx" {
        let mut workbook = open_workbook_auto(path).map_err(|e| {
            err(
                "IMPORT_READ_FAILED",
                "The Excel workbook could not be opened.",
                e.to_string(),
            )
        })?;
        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| {
                err(
                    "IMPORT_SHEET_MISSING",
                    "The workbook does not contain a worksheet.",
                    path.display().to_string(),
                )
            })?
            .map_err(|e| {
                err(
                    "IMPORT_READ_FAILED",
                    "The Excel worksheet could not be read.",
                    e.to_string(),
                )
            })?;
        let mut rows = range.rows();
        let header = rows
            .next()
            .ok_or_else(|| {
                err(
                    "IMPORT_HEADER_INVALID",
                    "The workbook is empty.",
                    path.display().to_string(),
                )
            })?
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let positions: Vec<usize> = HEADERS
            .iter()
            .map(|h| {
                header
                    .iter()
                    .position(|v| v.trim().eq_ignore_ascii_case(h))
                    .ok_or_else(|| {
                        err(
                            "IMPORT_HEADER_INVALID",
                            "The workbook does not contain the required columns.",
                            h.to_string(),
                        )
                    })
            })
            .collect::<AppResult<_>>()?;
        return Ok(rows
            .map(|row| {
                positions
                    .iter()
                    .map(|i| row.get(*i).map(ToString::to_string).unwrap_or_default())
                    .collect()
            })
            .collect());
    }
    Err(err(
        "IMPORT_FORMAT_UNSUPPORTED",
        "Choose a .csv or .xlsx file.",
        extension,
    ))
}

impl Database {
    pub fn preview_work_item_import(
        &self,
        path: &str,
        project_id: &str,
        item_type: &str,
    ) -> AppResult<ImportPreview> {
        let rows = read_rows(Path::new(path))?;
        let mut errors = Vec::new();
        let mut valid = 0;
        for (index, row) in rows.iter().enumerate() {
            let input = cells_to_input(row, project_id, item_type);
            match work_item::validate(&input) {
                Ok(()) => valid += 1,
                Err(e) => errors.push(format!("Row {}: {}", index + 2, e.message)),
            }
        }
        Ok(ImportPreview {
            format: Path::new(path)
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_uppercase(),
            valid_rows: valid,
            errors,
        })
    }
    pub fn import_work_items(
        &mut self,
        path: &str,
        project_id: &str,
        item_type: &str,
    ) -> AppResult<usize> {
        let rows = read_rows(Path::new(path))?;
        let inputs: Vec<WorkItemInput> = rows
            .iter()
            .map(|row| cells_to_input(row, project_id, item_type))
            .collect();
        for input in &inputs {
            work_item::validate(input)?;
        }
        let tx = self.connection.transaction().map_err(db)?;
        for input in &inputs {
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO work_items(id,project_id,item_type,number,title,description,status,priority,due_date,occurred_date,responsible_party,company,location,amount_cents,checklist_total,checklist_completed,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,0,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,input.project_id,input.item_type,input.number,input.title,input.description,input.status,input.priority,input.due_date,input.occurred_date,input.responsible_party,input.company,input.location,input.amount_cents,input.checklist_total]).map_err(db)?;
        }
        tx.execute("INSERT INTO activity_events(event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES('work_items.imported',?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![item_type,project_id,format!("Imported {} records",inputs.len())]).map_err(db)?;
        tx.commit().map_err(db)?;
        Ok(inputs.len())
    }
    pub fn export_work_items(&self, path: &str, ids: &[String]) -> AppResult<usize> {
        let destination = Path::new(path);
        if destination.exists() {
            return Err(err(
                "EXPORT_COLLISION",
                "A file already exists at the selected location.",
                path,
            ));
        }
        let selected = self
            .list_work_items()?
            .into_iter()
            .filter(|v| ids.is_empty() || ids.contains(&v.id))
            .collect::<Vec<_>>();
        let extension = destination
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if extension == "csv" {
            let mut writer = csv::Writer::from_path(destination).map_err(|e| {
                err(
                    "EXPORT_WRITE_FAILED",
                    "The CSV file could not be created.",
                    e.to_string(),
                )
            })?;
            writer
                .write_record([
                    "project_number",
                    "project_name",
                    "register",
                    "number",
                    "title",
                    "description",
                    "status",
                    "priority",
                    "due_date",
                    "occurred_date",
                    "responsible_party",
                    "company",
                    "location",
                    "amount",
                    "checklist_completed",
                    "checklist_total",
                ])
                .map_err(|e| {
                    err(
                        "EXPORT_WRITE_FAILED",
                        "The CSV file could not be written.",
                        e.to_string(),
                    )
                })?;
            for v in &selected {
                let amount = v
                    .amount_cents
                    .map(|c| format!("{:.2}", c as f64 / 100.0))
                    .unwrap_or_default();
                let checklist_completed = v.checklist_completed.to_string();
                let checklist_total = v.checklist_total.to_string();
                let values = [
                    safe_csv_text(&v.project_number),
                    safe_csv_text(&v.project_name),
                    safe_csv_text(&v.item_type),
                    safe_csv_text(v.number.as_deref().unwrap_or("")),
                    safe_csv_text(&v.title),
                    safe_csv_text(v.description.as_deref().unwrap_or("")),
                    safe_csv_text(&v.status),
                    safe_csv_text(&v.priority),
                    safe_csv_text(v.due_date.as_deref().unwrap_or("")),
                    safe_csv_text(v.occurred_date.as_deref().unwrap_or("")),
                    safe_csv_text(v.responsible_party.as_deref().unwrap_or("")),
                    safe_csv_text(v.company.as_deref().unwrap_or("")),
                    safe_csv_text(v.location.as_deref().unwrap_or("")),
                    Cow::Owned(amount),
                    Cow::Owned(checklist_completed),
                    Cow::Owned(checklist_total),
                ];
                writer
                    .write_record(values.iter().map(|value| value.as_bytes()))
                    .map_err(|e| {
                        err(
                            "EXPORT_WRITE_FAILED",
                            "The CSV file could not be written.",
                            e.to_string(),
                        )
                    })?;
            }
            writer.flush().map_err(|e| {
                err(
                    "EXPORT_WRITE_FAILED",
                    "The CSV file could not be finalized.",
                    e.to_string(),
                )
            })?;
        } else if extension == "xlsx" {
            let mut workbook = Workbook::new();
            let sheet = workbook.add_worksheet();
            let headers = [
                "Project number",
                "Project name",
                "Register",
                "Number",
                "Title",
                "Description",
                "Status",
                "Priority",
                "Due date",
                "Occurred date",
                "Responsible party",
                "Company",
                "Location",
                "Amount",
                "Checklist completed",
                "Checklist total",
            ];
            for (col, value) in headers.iter().enumerate() {
                sheet.write_string(0, col as u16, *value).map_err(xlsx)?;
            }
            for (index, v) in selected.iter().enumerate() {
                let row = (index + 1) as u32;
                let values = [
                    v.project_number.clone(),
                    v.project_name.clone(),
                    v.item_type.clone(),
                    v.number.clone().unwrap_or_default(),
                    v.title.clone(),
                    v.description.clone().unwrap_or_default(),
                    v.status.clone(),
                    v.priority.clone(),
                    v.due_date.clone().unwrap_or_default(),
                    v.occurred_date.clone().unwrap_or_default(),
                    v.responsible_party.clone().unwrap_or_default(),
                    v.company.clone().unwrap_or_default(),
                    v.location.clone().unwrap_or_default(),
                    v.amount_cents
                        .map(|c| format!("{:.2}", c as f64 / 100.0))
                        .unwrap_or_default(),
                    v.checklist_completed.to_string(),
                    v.checklist_total.to_string(),
                ];
                for (col, value) in values.iter().enumerate() {
                    sheet.write_string(row, col as u16, value).map_err(xlsx)?;
                }
            }
            sheet.autofit();
            workbook.save(destination).map_err(xlsx)?;
        } else {
            return Err(err(
                "EXPORT_FORMAT_UNSUPPORTED",
                "Choose a .csv or .xlsx destination.",
                extension,
            ));
        }
        Ok(selected.len())
    }
}
fn db(e: rusqlite::Error) -> AppError {
    err(
        "DATABASE_ERROR",
        "The data operation failed.",
        e.to_string(),
    )
}
fn xlsx(e: rust_xlsxwriter::XlsxError) -> AppError {
    err(
        "EXPORT_WRITE_FAILED",
        "The Excel workbook could not be created.",
        e.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_text_cells_neutralize_spreadsheet_formula_prefixes() {
        for dangerous in [
            "=SUM(A1:A2)",
            "+cmd",
            "-2+3",
            "@lookup",
            "\tformula",
            "\rformula",
        ] {
            assert_eq!(safe_csv_text(dangerous), format!("'{dangerous}"));
        }
        assert_eq!(safe_csv_text("ordinary text"), "ordinary text");
        assert_eq!(safe_csv_text(""), "");
    }

    #[test]
    fn csv_and_excel_exports_are_created_without_overwrite() {
        let root = std::env::temp_dir().join(format!("anydesk-exchange-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let mut db = Database::open(&root.join("workspace.sqlite3")).unwrap();
        db.connection.execute("INSERT INTO projects(id,number,name,status,phase,project_path,created_at_utc,updated_at_utc) VALUES('p1','P-1','Test','active','engineering','C:\\Test',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",[]).unwrap();
        let item = db
            .create_work_item(&WorkItemInput {
                project_id: "p1".into(),
                item_type: "procurement".into(),
                number: Some("PO-1".into()),
                title: "Order controller".into(),
                description: None,
                status: "open".into(),
                priority: "high".into(),
                due_date: Some("2026-10-01".into()),
                occurred_date: None,
                responsible_party: None,
                company: Some("Vendor".into()),
                location: None,
                amount_cents: Some(125050),
                checklist_total: 0,
                checklist_completed: 0,
            })
            .unwrap();
        let csv = root.join("records.csv");
        let xlsx = root.join("records.xlsx");
        assert_eq!(
            db.export_work_items(&csv.to_string_lossy(), &[item.id.clone()])
                .unwrap(),
            1
        );
        assert_eq!(
            db.export_work_items(&xlsx.to_string_lossy(), &[item.id])
                .unwrap(),
            1
        );
        assert!(csv.metadata().unwrap().len() > 0 && xlsx.metadata().unwrap().len() > 0);
        assert_eq!(
            db.preview_work_item_import(&csv.to_string_lossy(), "p1", "procurement")
                .unwrap()
                .valid_rows,
            1
        );
        assert_eq!(
            db.export_work_items(&csv.to_string_lossy(), &[])
                .unwrap_err()
                .code,
            "EXPORT_COLLISION"
        );
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn csv_export_neutralizes_text_formulas_but_preserves_numeric_columns() {
        let root = std::env::temp_dir().join(format!("sitedatum-csv-safety-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let mut db = Database::open(&root.join("workspace.sqlite3")).unwrap();
        db.connection.execute("INSERT INTO projects(id,number,name,status,phase,project_path,created_at_utc,updated_at_utc) VALUES('p1','P-1','=Formula Project','active','engineering','C:\\Test',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",[]).unwrap();
        let item = db
            .create_work_item(&WorkItemInput {
                project_id: "p1".into(),
                item_type: "procurement".into(),
                number: Some("@PO-1".into()),
                title: "+Order controller".into(),
                description: Some("-unsafe".into()),
                status: "open".into(),
                priority: "high".into(),
                due_date: Some("2026-10-01".into()),
                occurred_date: None,
                responsible_party: Some("\tOwner".into()),
                company: Some("=Vendor".into()),
                location: Some("\rYard".into()),
                amount_cents: Some(125050),
                checklist_total: 4,
                checklist_completed: 2,
            })
            .unwrap();
        db.connection
            .execute(
                "UPDATE work_items SET responsible_party=?1, location=?2 WHERE id=?3",
                rusqlite::params!["\tOwner", "\rYard", item.id],
            )
            .unwrap();
        let csv_path = root.join("records.csv");
        db.export_work_items(&csv_path.to_string_lossy(), &[item.id])
            .unwrap();
        let mut reader = csv::Reader::from_path(&csv_path).unwrap();
        let record = reader.records().next().unwrap().unwrap();
        assert_eq!(&record[1], "'=Formula Project");
        assert_eq!(&record[3], "'@PO-1");
        assert_eq!(&record[4], "'+Order controller");
        assert_eq!(&record[5], "'-unsafe");
        assert_eq!(&record[10], "'\tOwner");
        assert_eq!(&record[11], "'=Vendor");
        assert_eq!(&record[12], "'\rYard");
        assert_eq!(&record[13], "1250.50");
        assert_eq!(&record[14], "2");
        assert_eq!(&record[15], "4");
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
}
